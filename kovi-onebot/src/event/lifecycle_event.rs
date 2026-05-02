use crate::event::PostType;
use kovi::bot::BotInformation;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct LifecycleEvent {
    pub meta_event_type: String,
    pub post_type: PostType,
    pub self_id: i64,
    pub time: i64,
    pub sub_type: LifecycleAction,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LifecycleAction {
    Enable,
    Disable,
    Connect,
}

impl Event for LifecycleEvent {
    fn de(
        event: &InternalEvent,
        _: &BotInformation,
        _: &tokio::sync::mpsc::Sender<ApiAndOptOneshot>,
    ) -> Option<Self>
    where
        Self: Sized,
    {
        let InternalEvent::OneBotEvent(json_str) = event else {
            return None;
        };
        let event: LifecycleEvent = serde_json::from_value(json_str.clone()).ok()?;
        if event.meta_event_type == "lifecycle" {
            Some(event)
        } else {
            None
        }
    }
}
