use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 群成员增加事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMemberIncreaseReceiveEventData {
    /// 群号
    pub group_id: i64,
    /// 发生变更的用户 QQ 号
    pub user_id: i64,
    /// 管理员 QQ 号，如果是管理员同意入群
    #[serde(default)]
    pub operator_id: Option<i64>,
    /// 邀请者 QQ 号，如果是邀请入群
    #[serde(default)]
    pub invitor_id: Option<i64>,
}

pub type GroupMemberIncreaseEvent = MilkyEvent<GroupMemberIncreaseReceiveEventData>;

impl Event for GroupMemberIncreaseEvent {
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

impl GroupMemberIncreaseEvent {
    pub(crate) fn new(temp: &Value) -> Result<GroupMemberIncreaseEvent, EventBuildError> {
        let event: GroupMemberIncreaseEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
