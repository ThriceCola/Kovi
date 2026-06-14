use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 群管理员变更事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupAdminChangeReceiveEventData {
    /// 群号
    pub group_id: i64,
    /// 发生变更的用户 QQ 号
    pub user_id: i64,
    /// 操作者 QQ 号
    pub operator_id: i64,
    /// 是否被设置为管理员，`false` 表示被取消管理员
    pub is_set: bool,
}

pub type GroupAdminChangeEvent = MilkyEvent<GroupAdminChangeReceiveEventData>;

impl Event for GroupAdminChangeEvent {
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

impl GroupAdminChangeEvent {
    pub(crate) fn new(temp: &Value) -> Result<GroupAdminChangeEvent, EventBuildError> {
        let event: GroupAdminChangeEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
