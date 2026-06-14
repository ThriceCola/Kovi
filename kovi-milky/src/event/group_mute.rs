use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 群禁言事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMuteReceiveEventData {
    /// 群号
    pub group_id: i64,
    /// 发生变更的用户 QQ 号
    pub user_id: i64,
    /// 操作者 QQ 号
    pub operator_id: i64,
    /// 禁言时长（秒），为 0 表示取消禁言
    pub duration: i32,
}

pub type GroupMuteEvent = MilkyEvent<GroupMuteReceiveEventData>;

impl Event for GroupMuteEvent {
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

impl GroupMuteEvent {
    pub(crate) fn new(temp: &Value) -> Result<GroupMuteEvent, EventBuildError> {
        let event: GroupMuteEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
