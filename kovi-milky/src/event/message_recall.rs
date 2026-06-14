use crate::MilkyEvent;
use crate::event::msg_event::MessageScene;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 消息撤回事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecallReceiveEventData {
    /// 消息场景：friend（好友）、group（群）、temp（临时会话）
    pub message_scene: MessageScene,
    /// 好友 QQ 号或群号
    pub peer_id: i64,
    /// 消息序列号
    pub message_seq: i64,
    /// 被撤回的消息的发送者 QQ 号
    pub sender_id: i64,
    /// 操作者 QQ 号
    pub operator_id: i64,
    /// 撤回提示的后缀文本
    pub display_suffix: String,
}

pub type MessageRecallEvent = MilkyEvent<MessageRecallReceiveEventData>;

impl Event for MessageRecallEvent {
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

impl MessageRecallEvent {
    pub(crate) fn new(temp: &Value) -> Result<MessageRecallEvent, EventBuildError> {
        let event: MessageRecallEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
