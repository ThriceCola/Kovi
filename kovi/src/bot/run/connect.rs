use crate::ExitEvent;
use crate::bot::ApiReturn;
use crate::bot::handler::InternalInternalEvent;
use crate::driver::{Driver, DriverEvent, ReconnectConfig};
use crate::event::InternalEvent;
use crate::types::ApiAndOptOneshot;
use futures::StreamExt as _;
use std::sync::Arc;
use tokio::sync::mpsc::{self};

pub(crate) async fn event_connect(
    self_event_tx: mpsc::Sender<InternalInternalEvent>,
    drive: Arc<dyn Driver>,
) {
    // 重连退避参数由驱动提供，默认 1s 起步、指数增长、30s 封顶
    let ReconnectConfig {
        base_delay,
        max_delay,
    } = drive.reconnect_config();
    let mut retry_delay = base_delay;

    loop {
        let mut drive_stream = match drive.event_channel().await {
            Ok(drive_stream) => drive_stream,
            Err(err) => {
                eprintln!("Failed to get drive event channel: {err}; retrying in {retry_delay:?}");
                tokio::time::sleep(retry_delay).await;
                retry_delay = (retry_delay * 2).min(max_delay);
                continue;
            }
        };

        // 连接成功，重置
        retry_delay = base_delay;

        //处理事件，每个事件都会来到这里
        let mut connection_lost = false;
        while let Some(event) = drive_stream.next().await {
            let event = match event {
                Ok(event) => event,
                Err(err) => {
                    eprintln!("Failed to get drive event: {err}");
                    connection_lost = true;
                    break;
                }
            };

            match event {
                DriverEvent::Exit => {
                    self_event_tx
                        .send(InternalInternalEvent::Exit(ExitEvent::FromDrive))
                        .await
                        .expect("Kovi kernel encountered an unrecoverable error during message forwarding (channel closed)");
                    return;
                }
                DriverEvent::Normal(value) => {
                    self_event_tx
                        .send(InternalInternalEvent::OneBotEvent(Box::new(
                            InternalEvent::DriverEvent(value),
                        )))
                        .await
                        .expect("Kovi kernel encountered an unrecoverable error during message forwarding (channel closed)");
                }
            }
        }

        // 连接断开（服务端关闭或网络错误），等待后重连
        eprintln!(
            "Driver event channel {}; reconnecting in {retry_delay:?}",
            if connection_lost { "lost" } else { "ended" }
        );
        tokio::time::sleep(retry_delay).await;
        retry_delay = (retry_delay * 2).min(max_delay);
    }
}

pub(crate) async fn send_connect(
    mut self_api_rx: mpsc::Receiver<ApiAndOptOneshot>,
    self_event_tx: mpsc::Sender<InternalInternalEvent>,
    drive: Arc<dyn Driver>,
) {
    //处理事件，每个事件都会来到这里
    while let Some(api_and_oneshot) = self_api_rx.recv().await {
        tokio::spawn(send_api_inner(
            api_and_oneshot,
            self_event_tx.clone(),
            drive.clone(),
        ));
    }
}

async fn send_api_inner(
    api_and_oneshot: ApiAndOptOneshot,
    self_event_tx: mpsc::Sender<InternalInternalEvent>,
    drive: Arc<dyn Driver>,
) {
    let (send_api, oneshot) = api_and_oneshot;

    let result = drive.api_handler(send_api.clone()).await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            let err_msg = err.to_string();
            log::error!(
                "Kovi failed to handle API [{}]: {}",
                send_api.action,
                err_msg
            );

            // 构造一个错误返回值，避免调用方永久挂起
            let err_return = Err(ApiReturn {
                status: "failed".to_string(),
                retcode: -500,
                message: Some(format!("Kovi failed to handle API: {err_msg}")),
                data: serde_json::Value::Null,
            });

            // 如果有 oneshot，返回错误结果
            if let Some(oneshot) = oneshot {
                oneshot.send(err_return.clone()).ok();
            }

            // 继续发送 DriverApiEvent，让监听 MsgSendFromKoviEvent 的插件能感知到错误
            self_event_tx
                .send(InternalInternalEvent::OneBotEvent(Box::new(
                    InternalEvent::DriverApiEvent((send_api, err_return)),
                )))
                .await
                .expect(
                    "Kovi kernel encountered an unrecoverable error during message forwarding (channel closed)",
                );
            return;
        }
    };

    if let Some(oneshot) = oneshot {
        oneshot.send(result.clone()).ok();
    }

    self_event_tx
        .send(InternalInternalEvent::OneBotEvent(
           Box::new(InternalEvent::DriverApiEvent((send_api, result))),
        ))
        .await.expect("Kovi kernel encountered an unrecoverable error during message forwarding (channel closed)");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::{AnyError, MessageEventRegister};
    use async_trait::async_trait;
    use futures::Stream;
    use serde_json::json;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    /// 模拟驱动：每次 `event_channel` 返回一段事件流。
    /// 第一次连接发一条事件后流立即结束（模拟服务端断开）；
    /// 重连后发一条事件并保持在线。
    struct FakeDriver {
        connect_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Driver for FakeDriver {
        async fn event_channel(
            &self,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<DriverEvent, AnyError>> + Send>>, AnyError>
        {
            let n = self.connect_count.fetch_add(1, Ordering::SeqCst) + 1;
            let events = vec![Ok(DriverEvent::Normal(json!({ "conn": n })))];
            let iter = futures::stream::iter(events);

            let stream: Pin<Box<dyn Stream<Item = Result<DriverEvent, AnyError>> + Send>> =
                if n == 1 {
                    // 第一次连接：事件后流结束，模拟服务端断开
                    Box::pin(iter)
                } else {
                    // 重连成功：事件后保持在线
                    Box::pin(iter.chain(futures::stream::pending()))
                };
            Ok(stream)
        }

        fn api_handler(&self, _: crate::bot::SendApi) -> crate::driver::ApiHandlerResult {
            Box::pin(async { unreachable!() })
        }

        fn message_event_register(&self) -> MessageEventRegister {
            MessageEventRegister {
                type_de: Arc::new(|_, _, _| None),
            }
        }
    }

    #[tokio::test]
    async fn event_connect_reconnects_after_disconnect() {
        let connect_count = Arc::new(AtomicUsize::new(0));
        let drive = Arc::new(FakeDriver {
            connect_count: Arc::clone(&connect_count),
        });

        let (tx, mut rx) = mpsc::channel::<InternalInternalEvent>(32);
        let handle = tokio::spawn(event_connect(tx, drive));

        // 第一次连接的事件
        let first = rx.recv().await.expect("first connection event");
        assert!(matches!(first, InternalInternalEvent::OneBotEvent(_)));

        // 流断开后应自动重连，收到第二次连接的事件
        let second = tokio::time::timeout(Duration::from_secs(10), rx.recv())
            .await
            .expect("reconnect within 10s")
            .expect("second connection event");
        assert!(matches!(second, InternalInternalEvent::OneBotEvent(_)));

        // 确认确实重连了（不是同一条流继续产出）
        assert_eq!(connect_count.load(Ordering::SeqCst), 2);

        handle.abort();
    }
}
