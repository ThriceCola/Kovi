use super::Server;
use super::{ApiReturn, Bot, Host};
use crate::bot::handler::InternalInternalEvent;
use crate::event::InternalEvent;
use crate::types::ApiAndOneshot;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use http::HeaderValue;
use log::{debug, error, warn};
use parking_lot::{Mutex, RwLock};
use std::error::Error;
use std::fmt::Display;
use std::{net::IpAddr, sync::Arc};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest};

type ApiTxMap = Arc<Mutex<ahash::HashMap<String, ApiAndOneshot>>>;

impl Bot {
    pub(crate) async fn ws_connect(
        server: Server,
        api_rx: mpsc::Receiver<ApiAndOneshot>,
        event_tx: mpsc::Sender<InternalInternalEvent>,
        bot: Arc<RwLock<Bot>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        #[allow(clippy::type_complexity)]
        let (event_connected_tx, event_connected_rx): (
            oneshot::Sender<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
            oneshot::Receiver<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
        ) = oneshot::channel();

        #[allow(clippy::type_complexity)]
        let (api_connected_tx, api_connected_rx): (
            oneshot::Sender<Result<(), Box<dyn Error + Send + Sync>>>,
            oneshot::Receiver<Result<(), Box<dyn Error + Send + Sync>>>,
        ) = oneshot::channel();

        {
            let mut bot_write = bot.write();
            bot_write.spawn(Self::ws_event_connect(
                server.clone(),
                event_tx.clone(),
                event_connected_tx,
                bot.clone(),
            ));
            bot_write.spawn(Self::ws_send_api(
                server,
                api_rx,
                event_tx,
                api_connected_tx,
                bot.clone(),
            ));
        }

        let (res1, res2) = tokio::join!(event_connected_rx, api_connected_rx);
        let (res1, res2) = (res1.expect("unreachable"), res2.expect("unreachable"));
        match (res1, res2) {
            (Ok(_), Ok(_)) => Ok(()),
            (Err(e), _) | (_, Err(e)) => Err(e),
        }
    }

    pub(crate) async fn ws_event_connect(
        server: Server,
        event_tx: mpsc::Sender<InternalInternalEvent>,
        connected_tx: oneshot::Sender<Result<(), Box<dyn Error + Send + Sync>>>,
        bot: Arc<RwLock<Bot>>,
    ) {
        let (host, port, access_token, secure) =
            (server.host, server.port, server.access_token, server.secure);

        let protocol = if secure { "wss" } else { "ws" };
        let mut request = match host {
            Host::IpAddr(ip) => match ip {
                IpAddr::V4(ip) => format!("{protocol}://{ip}:{port}/event")
                    .into_client_request()
                    .expect("The domain name is invalid"),
                IpAddr::V6(ip) => format!("{protocol}://[{ip}]:{port}/event")
                    .into_client_request()
                    .expect("The domain name is invalid"),
            },
            Host::Domain(domain) => format!("{protocol}://{domain}:{port}/event")
                .into_client_request()
                .expect("The domain name is invalid"),
        };

        //增加Authorization头
        if !access_token.is_empty() {
            request.headers_mut().insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {access_token}")).expect("unreachable"),
            );
        }

        let (ws_stream, _) = match connect_async(request).await {
            Ok(v) => v,
            Err(e) => {
                connected_tx
                    .send(Err(e.into()))
                    .expect("The OneBot connect channel has been established");
                return;
            }
        };

        connected_tx
            .send(Ok(()))
            .expect("The OneBot connect channel has been established");

        let (_, read) = ws_stream.split();

        let mut bot_write = bot.write();
        bot_write.spawn(ws_event_connect_read(read, event_tx));
    }

    pub(crate) async fn ws_send_api(
        server: Server,
        api_rx: mpsc::Receiver<ApiAndOneshot>,
        event_tx: mpsc::Sender<InternalInternalEvent>,
        connected_tx: oneshot::Sender<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
        bot: Arc<RwLock<Bot>>,
    ) {
        let (host, port, access_token, secure) =
            (server.host, server.port, server.access_token, server.secure);

        let protocol = if secure { "wss" } else { "ws" };
        let mut request = match host {
            Host::IpAddr(ip) => match ip {
                IpAddr::V4(ip) => format!("{protocol}://{ip}:{port}/api")
                    .into_client_request()
                    .expect("The domain name is invalid"),
                IpAddr::V6(ip) => format!("{protocol}://[{ip}]:{port}/api")
                    .into_client_request()
                    .expect("The domain name is invalid"),
            },
            Host::Domain(domain) => format!("{protocol}://{domain}:{port}/api")
                .into_client_request()
                .expect("The domain name is invalid"),
        };

        //增加Authorization头
        if !access_token.is_empty() {
            request.headers_mut().insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {access_token}")).expect("unreachable"),
            );
        }

        let (ws_stream, _) = match connect_async(request).await {
            Ok(v) => v,
            Err(e) => {
                connected_tx
                    .send(Err(e.into()))
                    .expect("The OneBot connect channel has been established");
                return;
            }
        };

        connected_tx
            .send(Ok(()))
            .expect("The OneBot connect channel has been established");

        let (write, read) = ws_stream.split();
        let api_tx_map: ApiTxMap = Arc::new(Mutex::new(ahash::HashMap::<_, _>::default()));

        let mut bot_write = bot.write();

        //读
        bot_write.spawn(ws_send_api_read(
            read,
            event_tx.clone(),
            Arc::clone(&api_tx_map),
        ));

        //写
        bot_write.spawn(ws_send_api_write(
            write,
            api_rx,
            event_tx,
            api_tx_map.clone(),
        ));
    }
}

async fn ws_event_connect_read(
    read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    event_tx: Sender<InternalInternalEvent>,
) {
    read.for_each(|msg| {
        let event_tx = event_tx.clone();
        async {
            match msg {
                Ok(msg) => handle_msg(msg, event_tx).await,
                Err(e) => connection_failed_eprintln(e, event_tx).await,
            }
        }
    })
    .await;

    async fn handle_msg(
        msg: tokio_tungstenite::tungstenite::Message,
        event_tx: Sender<InternalInternalEvent>,
    ) {
        if !msg.is_text() {
            return;
        }

        let text = msg.to_text().expect("unreachable");
        if let Err(e) = event_tx
            .send(InternalInternalEvent::OneBotEvent(
                InternalEvent::OneBotEvent(text.to_string()),
            ))
            .await
        {
            debug!("通道关闭：{e}")
        }
    }
}

async fn ws_send_api_read(
    read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    event_tx: Sender<InternalInternalEvent>,
    api_tx_map: ApiTxMap,
) {
    read.for_each(|msg| {
        let event_tx = event_tx.clone();
        async {
            match msg {
                Ok(msg) => handle_msg(msg, event_tx, api_tx_map.clone()).await,
                Err(e) => connection_failed_eprintln(e, event_tx).await,
            }
        }
    })
    .await;

    async fn handle_msg(
        msg: tokio_tungstenite::tungstenite::Message,
        event_tx: Sender<InternalInternalEvent>,
        api_tx_map: ApiTxMap,
    ) {
        if msg.is_close() {
            connection_failed_eprintln(format!("{msg}\nBot api connection failed"), event_tx).await;
            return;
        }
        if !msg.is_text() {
            return;
        }

        let text = msg.to_text().expect("unreachable");

        debug!("{text}");

        let return_value: ApiReturn = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => {
                debug!("Unknow api return： {text}");
                return;
            }
        };

        if return_value.status != "ok" {
            warn!("Api return error: {text}")
        }

        let api_tx_cache = {
            let mut api_tx_map = api_tx_map.lock();
            match api_tx_map.remove(&return_value.echo) {
                Some(v) => v,
                None => {
                    log::error!("Api return echo not found from api_tx_map: {text}");
                    return;
                }
            }
        };

        let return_value = if return_value.status.to_lowercase() == "ok" {
            Ok(return_value)
        } else {
            Err(return_value)
        };

        if let Some(tx) = api_tx_cache.1
            && tx.send(return_value.clone()).is_err() {
                log::debug!("Return Api to plugin failed, the receiver has been closed")
            };

        event_tx
            .send(InternalInternalEvent::OneBotEvent(
                InternalEvent::OneBotApiEvent((api_tx_cache.0, return_value)),
            ))
            .await
            .expect("The event_tx is closed");
    }
}

async fn ws_send_api_write(
    mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut api_rx: mpsc::Receiver<ApiAndOneshot>,
    event_tx: Sender<InternalInternalEvent>,
    api_tx_map: ApiTxMap,
) {
    while let Some((api_msg, return_api_tx)) = api_rx.recv().await {
        let event_tx = event_tx.clone();
        debug!("{api_msg}");

        api_tx_map
            .lock()
            .insert(api_msg.echo.clone(), (api_msg.clone(), return_api_tx));

        let msg = tokio_tungstenite::tungstenite::Message::text(api_msg.to_string());

        if let Err(e) = write.send(msg).await {
            connection_failed_eprintln(e, event_tx).await;
        }
    }
}

async fn connection_failed_eprintln<E>(e: E, event_tx: Sender<InternalInternalEvent>)
where
    E: Display,
{
    log::error!("{e}\nBot connection failed, please check the configuration and restart.");
    if let Err(e) = event_tx
        .send(InternalInternalEvent::KoviEvent(
            crate::bot::handler::KoviEvent::Drop,
        ))
        .await
    {
        error!("通道关闭,{e}")
    };
}
