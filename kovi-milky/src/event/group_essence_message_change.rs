use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 群精华消息变更事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupEssenceMessageChangeReceiveEventData {
    /// 群号
    pub group_id: i64,
    /// 发生变更的消息序列号
    pub message_seq: i64,
    /// 操作者 QQ 号
    pub operator_id: i64,
    /// 是否被设置为精华，`false` 表示被取消精华
    pub is_set: bool,
}

pub type GroupEssenceMessageChangeEvent = MilkyEvent<GroupEssenceMessageChangeReceiveEventData>;

impl Event for GroupEssenceMessageChangeEvent {
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

impl GroupEssenceMessageChangeEvent {
    pub(crate) fn new(temp: &Value) -> Result<GroupEssenceMessageChangeEvent, EventBuildError> {
        let event: GroupEssenceMessageChangeEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
