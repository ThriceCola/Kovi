use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 好友文件上传事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendFileUploadReceiveEventData {
    /// 好友 QQ 号
    pub user_id: i64,
    /// 文件 ID
    pub file_id: String,
    /// 文件名称
    pub file_name: String,
    /// 文件大小（字节）
    pub file_size: i64,
    /// 文件的 TriSHA1 哈希值
    pub file_hash: String,
    /// 是否是自己发送的文件
    pub is_self: bool,
}

pub type FriendFileUploadEvent = MilkyEvent<FriendFileUploadReceiveEventData>;

impl Event for FriendFileUploadEvent {
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

impl FriendFileUploadEvent {
    pub(crate) fn new(temp: &Value) -> Result<FriendFileUploadEvent, EventBuildError> {
        let event: FriendFileUploadEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
