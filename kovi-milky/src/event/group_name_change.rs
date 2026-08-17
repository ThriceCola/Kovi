use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 群名称变更事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupNameChangeReceiveEventData {
    /// 群号
    pub group_id: i64,
    /// 新的群名称
    pub new_group_name: String,
    /// 操作者 QQ 号
    pub operator_id: i64,
}

pub type GroupNameChangeEvent = MilkyEvent<GroupNameChangeReceiveEventData>;

impl Event for GroupNameChangeEvent {
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

impl GroupNameChangeEvent {
    pub(crate) fn new(temp: &Value) -> Result<GroupNameChangeEvent, EventBuildError> {
        let event: GroupNameChangeEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
