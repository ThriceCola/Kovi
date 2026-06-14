use crate::bot::ApiReturn;
use crate::bot::handler::{ExitEvent, InternalInternalEvent};
use crate::driver::{Driver, DriverEvent};
use crate::event::InternalEvent;
use crate::types::ApiAndOptOneshot;
use futures::StreamExt as _;
use std::sync::Arc;
use tokio::sync::mpsc::{self};

pub(crate) async fn event_connect(
    self_event_tx: mpsc::Sender<InternalInternalEvent>,
    drive: Arc<dyn Driver>,
) {
    let mut drive_stream = match drive.event_channel().await {
        Ok(drive_stream) => drive_stream,
        Err(err) => {
            eprintln!("Failed to get drive event channel: {}", err);
            self_event_tx.send(InternalInternalEvent::Exit(ExitEvent::FromDrive)).await.expect("Kovi kernel encountered an unrecoverable error during message forwarding (channel closed)");
            return;
        }
    };

    //处理事件，每个事件都会来到这里
    while let Some(event) = drive_stream.next().await {
        let event = match event {
            Ok(event) => event,
            Err(err) => {
                eprintln!("Failed to get drive event: {}", err);
                self_event_tx.send(InternalInternalEvent::Exit(ExitEvent::FromDrive)).await.expect("Kovi kernel encountered an unrecoverable error during message forwarding (channel closed)");
                return;
            }
        };

        let internal_event = match event {
            DriverEvent::Exit => InternalInternalEvent::Exit(ExitEvent::FromDrive),
            DriverEvent::Normal(value) => {
                InternalInternalEvent::OneBotEvent(Box::new(InternalEvent::DriverEvent(value)))
            }
        };

        self_event_tx.send(internal_event).await.expect("Kovi kernel encountered an unrecoverable error during message forwarding (channel closed)");
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
