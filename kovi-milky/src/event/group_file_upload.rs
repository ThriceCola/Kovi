use crate::MilkyEvent;
use kovi::bot::BotInformation;
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::sync::mpsc;

/// 群文件上传事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupFileUploadReceiveEventData {
    /// 群号
    pub group_id: i64,
    /// 发送者 QQ 号
    pub user_id: i64,
    /// 文件 ID
    pub file_id: String,
    /// 文件名称
    pub file_name: String,
    /// 文件大小（字节）
    pub file_size: i64,
}

pub type GroupFileUploadEvent = MilkyEvent<GroupFileUploadReceiveEventData>;

impl Event for GroupFileUploadEvent {
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

impl GroupFileUploadEvent {
    pub(crate) fn new(temp: &Value) -> Result<GroupFileUploadEvent, EventBuildError> {
        let event: GroupFileUploadEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        debug!("{event:?}");

        Ok(event)
    }
}
