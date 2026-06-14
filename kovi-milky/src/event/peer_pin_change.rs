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

/// 会话置顶变更事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerPinChangeReceiveEventData {
    /// 发生改变的会话的消息场景：friend（好友）、group（群）、temp（临时会话）
    pub message_scene: MessageScene,
    /// 发生改变的好友 QQ 号或群号
    pub peer_id: i64,
    /// 是否被置顶，`false` 表示取消置顶
    pub is_pinned: bool,
}

pub type PeerPinChangeEvent = MilkyEvent<PeerPinChangeReceiveEventData>;

impl Event for PeerPinChangeEvent {
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

impl PeerPinChangeEvent {
    pub(crate) fn new(temp: &Value) -> Result<PeerPinChangeEvent, EventBuildError> {
        let event: PeerPinChangeEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
