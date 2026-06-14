use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 好友请求事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRequestReceiveEventData {
    /// 申请好友的用户 QQ 号
    pub initiator_id: i64,
    /// 用户 UID
    pub initiator_uid: String,
    /// 申请附加信息
    pub comment: String,
    /// 申请来源
    pub via: String,
}

pub type FriendRequestEvent = MilkyEvent<FriendRequestReceiveEventData>;

impl Event for FriendRequestEvent {
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

impl FriendRequestEvent {
    pub(crate) fn new(temp: &Value) -> Result<FriendRequestEvent, EventBuildError> {
        let event: FriendRequestEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
