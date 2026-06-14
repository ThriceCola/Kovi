use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 群戳一戳事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupNudgeReceiveEventData {
    /// 群号
    pub group_id: i64,
    /// 发送者 QQ 号
    pub sender_id: i64,
    /// 接收者 QQ 号
    pub receiver_id: i64,
    /// 戳一戳提示的动作文本
    pub display_action: String,
    /// 戳一戳提示的后缀文本
    pub display_suffix: String,
    /// 戳一戳提示的动作图片 URL，用于取代动作提示文本
    pub display_action_img_url: String,
}

pub type GroupNudgeEvent = MilkyEvent<GroupNudgeReceiveEventData>;

impl Event for GroupNudgeEvent {
    fn de(
        event: &InternalEvent,
        _: &BotInformation,
        _: &mpsc::Sender<ApiAndOptOneshot>,
    ) -> Option<Self> {
        let InternalEvent::DriverEvent(json) = event else {
            return None;
        };

        Self::new(json).ok()
    }
}

impl GroupNudgeEvent {
    pub(crate) fn new(temp: &Value) -> Result<GroupNudgeEvent, EventBuildError> {
        let event: GroupNudgeEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
