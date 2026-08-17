use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 群消息表情回应类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReactionType {
    /// 系统表情
    Face,
    /// 自定义 emoji
    Emoji,
}

/// 群消息表情回应事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMessageReactionReceiveEventData {
    /// 群号
    pub group_id: i64,
    /// 发送回应者 QQ 号
    pub user_id: i64,
    /// 消息序列号
    pub message_seq: i64,
    /// 表情 ID
    pub face_id: String,
    /// 收到的回应类型
    pub reaction_type: ReactionType,
    /// 是否为添加，`false` 表示取消回应
    pub is_add: bool,
}

pub type GroupMessageReactionEvent = MilkyEvent<GroupMessageReactionReceiveEventData>;

impl Event for GroupMessageReactionEvent {
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

impl GroupMessageReactionEvent {
    pub(crate) fn new(temp: &Value) -> Result<GroupMessageReactionEvent, EventBuildError> {
        let event: GroupMessageReactionEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
