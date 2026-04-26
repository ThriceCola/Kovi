use crate::bot::handler::InternalInternalEvent;
use crate::drive::{Drive, DriveEvent};
use crate::event::InternalEvent;
use crate::types::ApiAndOptOneshot;
use futures::StreamExt as _;
use std::sync::Arc;
use tokio::sync::mpsc::{self};

pub(crate) async fn event_connect(
    self_event_tx: mpsc::Sender<InternalInternalEvent>,
    drive: Arc<dyn Drive>,
) {
    let mut drive_stream = drive.event_channel();

    //处理事件，每个事件都会来到这里
    while let Some(event) = drive_stream.next().await {
        let internal_event = match event {
            DriveEvent::Exit => InternalInternalEvent::Exit,
            DriveEvent::Normal(value) => {
                InternalInternalEvent::OneBotEvent(InternalEvent::OneBotEvent(value))
            }
        };

        self_event_tx.send(internal_event).await.expect("Kovi kernel encountered an unrecoverable error during message forwarding (channel closed)");
    }
}

pub(crate) async fn send_connect(
    mut self_api_rx: mpsc::Receiver<ApiAndOptOneshot>,
    self_event_tx: mpsc::Sender<InternalInternalEvent>,
    drive: Arc<dyn Drive>,
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
    drive: Arc<dyn Drive>,
) {
    let (send_api, oneshot) = api_and_oneshot;

    let result = drive.api_handler(send_api.clone()).await;
    if let Some(oneshot) = oneshot {
        oneshot.send(result.clone()).ok();
    }

    self_event_tx
        .send(InternalInternalEvent::OneBotEvent(
            InternalEvent::OneBotApiEvent((send_api, result)),
        ))
        .await.expect("Kovi kernel encountered an unrecoverable error during message forwarding (channel closed)");
}
