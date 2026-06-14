use crate::driver::MilkyDriver;
use crate::driver::config::Server;
use futures_util::{SinkExt, StreamExt};
use http::HeaderValue;
use kovi::driver::{AnyError, DriverEvent};
use kovi::futures_util;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite};

struct WsEventStream {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    closed: bool,
}

impl futures_util::Stream for WsEventStream {
    type Item = Result<DriverEvent, AnyError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.closed {
            return Poll::Ready(None);
        }

        loop {
            match this.ws.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(tungstenite::Message::Text(text)))) => {
                    println!("{text}");
                    match serde_json::from_str(&text) {
                        Ok(event) => return Poll::Ready(Some(Ok(DriverEvent::Normal(event)))),
                        Err(e) => return Poll::Ready(Some(Err(e.into()))),
                    }
                }
                Poll::Ready(Some(Ok(tungstenite::Message::Close(frame)))) => {
                    this.closed = true;
                    // 完成关闭握手：回应 Close
                    let _ = this.ws.start_send_unpin(tungstenite::Message::Close(frame));
                    let _ = this.ws.poll_flush_unpin(cx);
                    return Poll::Ready(None);
                }
                Poll::Ready(Some(Ok(tungstenite::Message::Ping(data)))) => {
                    // 即使 tungstenite 内部已自动回复，兜底处理
                    let _ = this.ws.start_send_unpin(tungstenite::Message::Pong(data));
                    let _ = this.ws.poll_flush_unpin(cx);
                    continue;
                }
                Poll::Ready(Some(Ok(
                    tungstenite::Message::Pong(_) | tungstenite::Message::Frame(_),
                ))) => {
                    continue;
                }
                Poll::Ready(Some(Ok(_))) => {
                    return Poll::Ready(Some(Err("The WebSocket message is not text".into())));
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e.into()))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl MilkyDriver {
    pub(crate) async fn ws_event_connect(
        server: Server,
    ) -> Result<
        std::pin::Pin<
            Box<
                dyn futures_util::Stream<Item = Result<DriverEvent, kovi::driver::AnyError>> + Send,
            >,
        >,
        kovi::driver::AnyError,
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

        let stream = WsEventStream {
            ws: ws_stream,
            closed: false,
        };

        Ok(Box::pin(stream))
    }
}
