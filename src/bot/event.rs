use crate::{bot::BotInformation, types::ApiAndOneshot};
use serde::{Deserialize, Serialize};
use std::any::Any;
use thiserror::Error;

pub use msg_event::MsgEvent;
pub use notice_event::NoticeEvent;
pub use request_event::RequestEvent;

pub mod lifecycle_event;
pub mod msg_event;
pub mod notice_event;
pub mod request_event;

#[deprecated(since = "0.11.0", note = "请使用 `MsgEvent` 代替")]
pub type AllMsgEvent = MsgEvent;
#[deprecated(since = "0.11.0", note = "请使用 `NoticeEvent` 代替")]
pub type AllNoticeEvent = NoticeEvent;
#[deprecated(since = "0.11.0", note = "请使用 `RequestEvent` 代替")]
pub type AllRequestEvent = RequestEvent;

#[derive(Debug, Copy, Clone)]
pub enum Sex {
    Male,
    Female,
}

#[derive(Debug, Clone)]
pub struct Sender {
    pub user_id: i64,
    pub nickname: Option<String>,
    pub card: Option<String>,
    pub sex: Option<Sex>,
    pub age: Option<i32>,
    pub area: Option<String>,
    pub level: Option<String>,
    pub role: Option<String>,
    pub title: Option<String>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Anonymous {
    pub id: i64,
    pub name: String,
    pub flag: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PostType {
    Message,
    Notice,
    Request,
    MetaEvent,
}

/// 满足此 trait 即可在Kovi运行时中监听并处理
pub trait Event: Any + Send + Sync {
    /// 解析事件
    ///
    /// 传入三个东西，按需所取。
    ///  - 原始的json字符串
    ///  - 借用的bot信息，可以通过 `BotInformation` 获取 `Bot` 相关的信息，例如管理员是谁。
    ///  - 借用的api发送通道，可以通过 `api_tx.clone()` 来让事件可以发送 api
    ///
    /// 如果认为此 json 不符合事件要求，请返回 `None`。
    ///
    /// 在一个消息周期内，Kovi 运行时会缓存此事件。
    ///
    /// 不需要的信息用 `_` 忽略，例如：
    ///
    /// ```
    /// impl Event for LifecycleEvent {
    ///     fn de(
    ///         json_str: &str,
    ///         _: &BotInformation,
    ///         _: &tokio::sync::mpsc::Sender<ApiAndOneshot>,
    ///     ) -> Option<Self>
    ///     where
    ///         Self: Sized,
    ///     {
    ///         if json_str.contains("lifecycle") {
    ///             return None;
    ///         }
    ///         serde_json::from_str(json_str).ok()
    ///     }
    /// }
    /// ```
    fn de(
        json_str: &str,
        bot: &BotInformation,
        api_tx: &tokio::sync::mpsc::Sender<ApiAndOneshot>,
    ) -> Option<Self>
    where
        Self: Sized;
}

#[derive(Error, Debug)]
pub(crate) enum EventBuildError {
    /// 解析出错
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[test]
fn post_type_is_ok() {
    use serde_json::json;

    assert_eq!(
        PostType::Message,
        serde_json::from_value::<PostType>(json!("message")).unwrap()
    );
    assert_eq!(
        PostType::Notice,
        serde_json::from_value::<PostType>(json!("notice")).unwrap()
    );
    assert_eq!(
        PostType::Request,
        serde_json::from_value::<PostType>(json!("request")).unwrap()
    );
    assert_eq!(
        PostType::MetaEvent,
        serde_json::from_value::<PostType>(json!("meta_event")).unwrap()
    );
}
