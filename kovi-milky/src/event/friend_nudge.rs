use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 好友戳一戳事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendNudgeReceiveEventData {
    /// 好友 QQ 号
    pub user_id: i64,
    /// 是否是自己发送的戳一戳
    pub is_self_send: bool,
    /// 是否是自己接收的戳一戳
    pub is_self_receive: bool,
    /// 戳一戳提示的动作文本
    pub display_action: String,
    /// 戳一戳提示的后缀文本
    pub display_suffix: String,
    /// 戳一戳提示的动作图片 URL，用于取代动作提示文本
    pub display_action_img_url: String,
}

pub type FriendNudgeEvent = MilkyEvent<FriendNudgeReceiveEventData>;

impl Event for FriendNudgeEvent {
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

impl FriendNudgeEvent {
    pub(crate) fn new(temp: &Value) -> Result<FriendNudgeEvent, EventBuildError> {
        let event: FriendNudgeEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
