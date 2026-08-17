use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 他人邀请自身入群事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInvitationReceiveEventData {
    /// 群号
    pub group_id: i64,
    /// 邀请序列号
    pub invitation_seq: i64,
    /// 邀请者 QQ 号
    pub initiator_id: i64,
    /// 来源群号，如果是通过 QQ 群邀请
    #[serde(default)]
    pub source_group_id: Option<i64>,
}

pub type GroupInvitationEvent = MilkyEvent<GroupInvitationReceiveEventData>;

impl Event for GroupInvitationEvent {
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

impl GroupInvitationEvent {
    pub(crate) fn new(temp: &Value) -> Result<GroupInvitationEvent, EventBuildError> {
        let event: GroupInvitationEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
