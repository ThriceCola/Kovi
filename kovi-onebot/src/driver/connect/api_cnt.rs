use crate::driver::config::Server;
use crate::driver::{self, AbortOnDrop, OneshotTxMap};
use ahash::RandomState;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use http::HeaderValue;
use kovi::bot::SendApi;
use kovi::driver::{AnyError, DriverEvent};
use kovi::{ApiReturn, futures_util};
use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

pub(crate) type OneBotApiOneshotSender = oneshot::Sender<Result<OneBotApiReturn, OneBotApiReturn>>;
type OneBotApiOneshotReceiver = oneshot::Receiver<Result<OneBotApiReturn, OneBotApiReturn>>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OneBotSendApi {
    pub action: String,
    pub params: Value,
    pub echo: String,
}

impl From<SendApi> for OneBotSendApi {
    fn from(api: SendApi) -> Self {
        Self {
            action: api.action,
            params: api.params,
            echo: rand_echo(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OneBotApiReturn {
    pub status: String,
    pub retcode: i32,
    pub data: Value,
    pub echo: String,
}

impl From<OneBotApiReturn> for ApiReturn {
    fn from(value: OneBotApiReturn) -> Self {
        Self {
            status: value.status,
            retcode: value.retcode,
            message: None,
            data: value.data,
        }
    }
}

pub fn rand_echo() -> String {
    RandomState::new()
        .hash_one(chrono::Utc::now().timestamp_micros())
        .to_string()
}

impl driver::OneBotDriver {
    /// api_handler 热路径：直接接收已就绪的 Sender，不再持有 server / tasks
    pub(crate) async fn send_api_inner(
        api_tx: mpsc::Sender<(OneBotSendApi, Option<OneBotApiOneshotSender>)>,
        send_api: SendApi,
    ) -> Result<Result<ApiReturn, ApiReturn>, AnyError> {
        let (temp_tx, temp_rx): (OneBotApiOneshotSender, OneBotApiOneshotReceiver) =
            oneshot::channel();

        api_tx
            .send((OneBotSendApi::from(send_api), Some(temp_tx)))
            .await
            .map_err(|e| {
                error!("Failed to send API request: {e}");
                Box::new(e) as AnyError
            })?;

        let value = temp_rx.await.map_err(|e| {
            error!("Failed to receive API response: {e}");
            Box::new(e) as AnyError
        })?;

        Ok(match value {
            Ok(v) => Ok(ApiReturn::from(v)),
            Err(v) => Err(ApiReturn::from(v)),
        })
    }

    /// 冷路径：建立 WS 连接并启动后台任务，返回 ApiContext（只在首次调用时执行）
    pub(crate) async fn init_api_context(
        server: Arc<Server>,
        event_tx: Arc<Mutex<Option<mpsc::Sender<Result<DriverEvent, AnyError>>>>>,
    ) -> Result<driver::ApiContext, AnyError> {
        let mut request = server
            .ws_url("api")
            .into_client_request()
            .expect("invalid WS URL");

        if !server.access_token.is_empty() {
            request.headers_mut().insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", server.access_token))
                    .expect("unreachable"),
            );
        }

        let (ws_stream, _) = connect_async(request).await?;
        let (write, read) = ws_stream.split();

        // echo -> oneshot sender 映射表，读写任务共享
        let tx_map: OneshotTxMap = Arc::new(parking_lot::Mutex::new(ahash::HashMap::default()));

        // mpsc channel：send_api_inner 把请求放进来，写任务消费
        let (api_tx, api_rx) = mpsc::channel::<(OneBotSendApi, Option<OneBotApiOneshotSender>)>(64);

        // 后台任务句柄存入 ApiContext，随 OnceCell 一起存活，Drop 时自动 abort
        let tasks = vec![
            AbortOnDrop(tokio::spawn(ws_read_task(
                read,
                Arc::clone(&tx_map),
                Arc::clone(&event_tx),
            ))),
            AbortOnDrop(tokio::spawn(ws_write_task(write, api_rx, tx_map, event_tx))),
        ];

        Ok(driver::ApiContext {
            api_tx,
            _tasks: tasks,
        })
    }
}

/// 读任务：从 WS 收到消息，按 echo 找到对应的 oneshot sender 并发送结果
async fn ws_read_task(
    read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    tx_map: OneshotTxMap,
    event_tx: Arc<Mutex<Option<mpsc::Sender<Result<DriverEvent, AnyError>>>>>,
) {
    read.for_each(|msg| {
        let tx_map = tx_map.clone();
        let event_tx = Arc::clone(&event_tx);
        async move {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    error!("WS read error: {e}");
                    send_exit_event(&event_tx).await;
                    return;
                }
            };

            if msg.is_close() {
                warn!("API WS connection closed by remote");
                send_exit_event(&event_tx).await;
                return;
            }
            if !msg.is_text() {
                return;
            }

            let text = msg.to_text().expect("unreachable");
            debug!("api recv: {text}");

            let ret: OneBotApiReturn = match serde_json::from_str(text) {
                Ok(v) => v,
                Err(_) => {
                    debug!("Unknown api return: {text}");
                    return;
                }
            };

            if ret.status.to_lowercase() != "ok" {
                warn!("Api return error: {text}");
            }

            let sender = tx_map.lock().remove(&ret.echo);
            let Some(sender) = sender else {
                error!("Api return echo not found in tx_map: {text}");
                return;
            };

            let result = if ret.status.to_lowercase() == "ok" {
                Ok(ret)
            } else {
                Err(ret)
            };

            if sender.send(result).is_err() {
                debug!("Return Api to plugin failed, the receiver has been closed");
            }
        }
    })
    .await;
}

/// 写任务：从 mpsc 收到请求，存入 map，再通过 WS 发出
async fn ws_write_task(
    mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut api_rx: mpsc::Receiver<(OneBotSendApi, Option<OneBotApiOneshotSender>)>,
    tx_map: OneshotTxMap,
    event_tx: Arc<Mutex<Option<mpsc::Sender<Result<DriverEvent, AnyError>>>>>,
) {
    while let Some((api_msg, return_tx)) = api_rx.recv().await {
        debug!("api send: {api_msg}");

        if let Some(tx) = return_tx {
            tx_map.lock().insert(api_msg.echo.clone(), tx);
        }

        if let Err(e) = write.send(Message::text(api_msg.to_string())).await {
            error!("WS write error: {e}");
            send_exit_event(&event_tx).await;
            return;
        }
    }
}

async fn send_exit_event(
    event_tx: &Arc<Mutex<Option<mpsc::Sender<Result<DriverEvent, AnyError>>>>>,
) {
    let tx = {
        let guard = event_tx.lock().await;
        guard.as_ref().cloned()
    };

    if let Some(tx) = tx {
        if tx.send(Ok(DriverEvent::Exit)).await.is_err() {
            debug!("Failed to forward DriveEvent::Exit to event channel");
        }
    }
}
