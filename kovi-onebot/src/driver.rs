use std::sync::Arc;

use crate::driver::config::{OneBotDriverConfig, Server};
use crate::driver::connect::api_cnt::{OneBotApiOneshotSender, OneBotSendApi};
use crate::driver::config::ReconnectConfig;
use crate::event::MsgEvent;
use kovi::bot::SendApi;
use kovi::driver::{Driver, DriverEvent, MessageEventRegister};
use kovi::futures_util;
use log::{error, info};
use tokio::sync::mpsc;

pub mod config;
pub(crate) mod connect;

/// echo -> oneshot sender，用于将 WS 返回的响应路由回调用者
pub(crate) type OneshotTxMap =
    Arc<parking_lot::Mutex<ahash::HashMap<String, OneBotApiOneshotSender>>>;

/// Drop 时自动 abort 的任务句柄
pub(crate) struct AbortOnDrop(pub(crate) tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// 初始化一次后持有的上下文：写端 sender + 后台任务句柄
pub(crate) struct ApiContext {
    pub(crate) api_tx: mpsc::Sender<(OneBotSendApi, Option<OneBotApiOneshotSender>)>,
    /// 字段名以 _ 开头，只用于 Drop 时自动 abort 任务
    _tasks: Vec<AbortOnDrop>,
}

/// API 连接上下文的共享槽位：`None` 表示未连接；断开时后台任务将其置回 `None` 以便自动重连
pub(crate) type ApiContextSlot = Arc<tokio::sync::Mutex<Option<ApiContext>>>;

pub struct OneBotDriver {
    pub(crate) server: Arc<Server>,
    /// API 连接上下文：连接断开时后台任务会将其置回 `None`，下次调用时自动重连
    ctx: ApiContextSlot,
    /// 事件通道断开后的自动重连参数
    reconnect: ReconnectConfig,
}

impl OneBotDriver {
    pub fn new(config: OneBotDriverConfig) -> Self {
        let config = OneBotDriverConfig::normalize_path(config);

        Self {
            server: Arc::new(config.server),
            ctx: Arc::new(tokio::sync::Mutex::new(None)),
            reconnect: config.reconnect,
        }
    }
}

#[async_trait::async_trait]
impl Driver for OneBotDriver {
    async fn event_channel(
        &self,
    ) -> Result<
        std::pin::Pin<
            Box<
                dyn futures_util::Stream<Item = Result<DriverEvent, kovi::driver::AnyError>> + Send,
            >,
        >,
        kovi::driver::AnyError,
    > {
        match self.handler_lifecycle_log_bot_enable().await {
            Ok(_) => {}
            Err(_) => {
                log::error!("Failed to initialize onebot connection");
                return Err("Failed to initialize onebot connection".into());
            }
        };

        OneBotDriver::ws_event_connect((*self.server).clone()).await
    }

    fn api_handler(
        &self,
        value: kovi::bot::SendApi,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Result<kovi::ApiReturn, kovi::ApiReturn>, kovi::driver::AnyError>> + Send>> {
        let server = Arc::clone(&self.server);
        let self_ctx = Arc::clone(&self.ctx);
        Box::pin(async move {
            // 每次调用都确保上下文就绪；断开后被重置为 None，这里会自动重连
            let mut guard = self_ctx.lock().await;
            if guard.is_none() {
                match OneBotDriver::init_api_context(server, Arc::clone(&self_ctx)).await {
                    Ok(api_ctx) => *guard = Some(api_ctx),
                    Err(err) => return Err(err),
                }
            }
            let api_tx = guard.as_ref().expect("unreachable").api_tx.clone();
            drop(guard);

            OneBotDriver::send_api_inner(api_tx, value).await
        })
    }

    fn message_event_register(&self) -> MessageEventRegister {
        MessageEventRegister::register::<MsgEvent>()
    }

    fn reconnect_config(&self) -> kovi::driver::ReconnectConfig {
        self.reconnect.to_kovi()
    }
}

impl std::fmt::Display for OneBotSendApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).expect("unreachable"))
    }
}

impl OneBotDriver {
    pub(crate) async fn handler_lifecycle_log_bot_enable(&self) -> Result<(), ()> {
        let api_msg = SendApi::new("get_login_info", serde_json::json!({}));

        let res = match self.api_handler(api_msg).await {
            Ok(v) => v,
            Err(err) => {
                let server_url = self.server.ws_url("api");
                error!("failed to initialize api_handler (server url: {server_url}): {err}");
                return Err(());
            }
        };

        let self_info_value = match res {
            Ok(v) => v,
            Err(e) => {
                error!("Lifecycle Error, get bot info failed: {e}");
                return Err(());
            }
        };

        let self_id = match self_info_value.data.get("user_id") {
            Some(user_id) => match user_id.as_i64() {
                Some(id) => id,
                None => {
                    error!("Expected 'user_id' to be an integer");
                    return Err(());
                }
            },
            None => {
                error!("Missing 'user_id' in self_info_value data");
                return Err(());
            }
        };
        let self_name = match self_info_value.data.get("nickname") {
            Some(nickname) => nickname.to_string(),
            None => {
                error!("Missing 'nickname' in self_info_value data");
                return Err(());
            }
        };
        info!("Bot connection successful，Nickname:{self_name},ID:{self_id}");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use kovi::tokio;
    use serde_json::Value;
    use std::net::Ipv4Addr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;
    use tokio_tungstenite::{accept_async, WebSocketStream};

    /// 服务端断开方式
    #[derive(Copy, Clone)]
    enum CloseMode {
        /// 正常发送 Close 帧
        CleanClose,
        /// 直接丢弃连接（读端报错）
        AbruptDrop,
    }

    /// 模拟 OneBot API WS 服务端：每连接处理一条请求并回包，然后按 mode 断开
    async fn spawn_api_server(
        mode: CloseMode,
        conn_count: Arc<AtomicUsize>,
    ) -> (u16, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind test server");
        let port = listener.local_addr().expect("local addr").port();

        let handle = tokio::spawn(async move {
            loop {
                let (stream, _) = match listener.accept().await {
                    Ok(v) => v,
                    Err(_) => break,
                };
                conn_count.fetch_add(1, Ordering::SeqCst);

                tokio::spawn(async move {
                    let mut ws: WebSocketStream<_> =
                        accept_async(stream).await.expect("ws accept");

                    // 处理一条 API 请求
                    let msg = ws.next().await.expect("api request").expect("msg");
                    if let Message::Text(text) = msg {
                        let req: Value = serde_json::from_str(&text).expect("req json");
                        let echo = req["echo"].as_str().expect("echo");
                        let reply = serde_json::json!({
                            "status": "ok",
                            "retcode": 0,
                            "data": { "user_id": 1, "nickname": "test-bot" },
                            "echo": echo,
                        });
                        ws.send(Message::text(reply.to_string()))
                            .await
                            .expect("send reply");
                    }

                    match mode {
                        CloseMode::CleanClose => {
                            ws.close(None).await.expect("close handshake");
                        }
                        CloseMode::AbruptDrop => {}
                    }
                });
            }
        });

        (port, handle)
    }

    fn driver_for(port: u16) -> OneBotDriver {
        OneBotDriver::new(OneBotDriverConfig {
            server: Server {
                host: crate::driver::config::Host::IpAddr(std::net::IpAddr::V4(
                    Ipv4Addr::LOCALHOST,
                )),
                port,
                access_token: String::new(),
                secure: false,
                path: "/".to_string(),
                all_in_one: true,
            },
            reconnect: ReconnectConfig::default(),
        })
    }

    /// 等待 API 上下文被后台任务重置（连接断开被感知）
    async fn wait_ctx_reset(driver: &OneBotDriver) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if driver.ctx.lock().await.is_none() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("API context should be reset after disconnect");
    }

    async fn assert_one_call_succeeds(driver: &OneBotDriver) {
        let api = SendApi::new("get_login_info", serde_json::json!({}));
        let res = driver
            .api_handler(api)
            .await
            .expect("api handler should succeed")
            .expect("api return should be ok");
        assert_eq!(res.data["user_id"], 1);
    }

    async fn run_reconnect_scenario(mode: CloseMode) {
        let conn_count = Arc::new(AtomicUsize::new(0));
        let (port, _server) = spawn_api_server(mode, Arc::clone(&conn_count)).await;
        let driver = driver_for(port);

        // 首次调用：建立第一条连接并成功
        assert_one_call_succeeds(&driver).await;
        assert_eq!(conn_count.load(Ordering::SeqCst), 1);

        // 等待服务端断开被感知（上下文重置）
        wait_ctx_reset(&driver).await;

        // 再次调用：应自动建立新连接并成功
        assert_one_call_succeeds(&driver).await;
        assert_eq!(conn_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn api_reconnects_after_clean_close() {
        run_reconnect_scenario(CloseMode::CleanClose).await;
    }

    #[tokio::test]
    async fn api_reconnects_after_abrupt_drop() {
        run_reconnect_scenario(CloseMode::AbruptDrop).await;
    }

    #[test]
    fn reconnect_config_is_honored() {
        let driver = OneBotDriver::new(OneBotDriverConfig {
            server: Server {
                host: crate::driver::config::Host::IpAddr(std::net::IpAddr::V4(
                    Ipv4Addr::LOCALHOST,
                )),
                port: 8081,
                access_token: String::new(),
                secure: false,
                path: "/".to_string(),
                all_in_one: true,
            },
            reconnect: ReconnectConfig {
                base_delay_secs: 3,
                max_delay_secs: 60,
            },
        });

        let config = driver.reconnect_config();
        assert_eq!(config.base_delay, Duration::from_secs(3));
        assert_eq!(config.max_delay, Duration::from_secs(60));
    }

    #[test]
    fn reconnect_config_normalizes_invalid_values() {
        // base 为 0 → 至少 1s；max 小于 base → 提升到 base
        let config = OneBotDriverConfig {
            server: Server {
                host: crate::driver::config::Host::IpAddr(std::net::IpAddr::V4(
                    Ipv4Addr::LOCALHOST,
                )),
                port: 8081,
                access_token: String::new(),
                secure: false,
                path: "/".to_string(),
                all_in_one: true,
            },
            reconnect: ReconnectConfig {
                base_delay_secs: 0,
                max_delay_secs: 0,
            },
        }
        .normalize_path();

        assert_eq!(config.reconnect.base_delay_secs, 1);
        assert_eq!(config.reconnect.max_delay_secs, 1);
    }
}
