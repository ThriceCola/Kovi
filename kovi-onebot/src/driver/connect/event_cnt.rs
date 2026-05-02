use crate::driver::config::Server;
use crate::driver::{self};
use futures_util::stream::{Select, SplitStream};
use futures_util::{StreamExt, stream};
use http::HeaderValue;
use kovi::drive::{AnyError, DriveEvent};
use kovi::futures_util;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

impl driver::OneBotDriver {
    pub(crate) async fn ws_event_connect(
        server: Server,
        event_rx: tokio::sync::mpsc::Receiver<Result<DriveEvent, AnyError>>,
    ) -> Result<
        std::pin::Pin<
            Box<dyn futures_util::Stream<Item = Result<DriveEvent, kovi::drive::AnyError>> + Send>,
        >,
        kovi::drive::AnyError,
    > {
        let mut request = server
            .ws_url("event")
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
        let (_, read): (_, SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>) =
            ws_stream.split();

        fn handle_msg(
            msg: tokio_tungstenite::tungstenite::Message,
        ) -> Result<DriveEvent, AnyError> {
            if !msg.is_text() {
                return Err("The WebSocket message is not text".into());
            }
            let text = msg.to_text().expect("unreachable");
            Ok(DriveEvent::Normal(serde_json::from_str(text)?))
        }

        let ws_stream = read.map(|msg_result| match msg_result {
            Ok(msg) => handle_msg(msg),
            Err(e) => Err(e.into()),
        });
        let injected_stream = stream::unfold(event_rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        });
        let stream: Select<_, _> = stream::select(ws_stream, injected_stream);

        Ok(Box::pin(stream))
    }
}
