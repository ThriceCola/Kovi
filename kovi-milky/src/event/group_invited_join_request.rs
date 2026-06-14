use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 群成员邀请他人入群请求事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInvitedJoinRequestReceiveEventData {
    /// 群号
    pub group_id: i64,
    /// 请求对应的通知序列号
    pub notification_seq: i64,
    /// 邀请者 QQ 号
    pub initiator_id: i64,
    /// 被邀请者 QQ 号
    pub target_user_id: i64,
}

pub type GroupInvitedJoinRequestEvent = MilkyEvent<GroupInvitedJoinRequestReceiveEventData>;

impl Event for GroupInvitedJoinRequestEvent {
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

impl GroupInvitedJoinRequestEvent {
    pub(crate) fn new(temp: &Value) -> Result<GroupInvitedJoinRequestEvent, EventBuildError> {
        let event: GroupInvitedJoinRequestEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
