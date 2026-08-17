use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 群全体禁言事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupWholeMuteReceiveEventData {
    /// 群号
    pub group_id: i64,
    /// 操作者 QQ 号
    pub operator_id: i64,
    /// 是否全员禁言，`false` 表示取消全员禁言
    pub is_mute: bool,
}

pub type GroupWholeMuteEvent = MilkyEvent<GroupWholeMuteReceiveEventData>;

impl Event for GroupWholeMuteEvent {
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

impl GroupWholeMuteEvent {
    pub(crate) fn new(temp: &Value) -> Result<GroupWholeMuteEvent, EventBuildError> {
        let event: GroupWholeMuteEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
