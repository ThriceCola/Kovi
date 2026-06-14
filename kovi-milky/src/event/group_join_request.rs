use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 入群请求事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupJoinRequestReceiveEventData {
    /// 群号
    pub group_id: i64,
    /// 请求对应的通知序列号
    pub notification_seq: i64,
    /// 请求是否被过滤（发起自风险账户）
    pub is_filtered: bool,
    /// 申请入群的用户 QQ 号
    pub initiator_id: i64,
    /// 申请附加信息
    pub comment: String,
}

pub type GroupJoinRequestEvent = MilkyEvent<GroupJoinRequestReceiveEventData>;

impl Event for GroupJoinRequestEvent {
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

impl GroupJoinRequestEvent {
    pub(crate) fn new(temp: &Value) -> Result<GroupJoinRequestEvent, EventBuildError> {
        let event: GroupJoinRequestEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
