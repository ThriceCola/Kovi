use crate::{
    bot::BotInformation,
    types::{ApiAndOneshot, ApiAndRuturn},
};
use serde::{Deserialize, Serialize};
use std::any::Any;

pub use admin_msg_event::AdminMsgEvent;
pub use group_msg_event::GroupMsgEvent;
pub use msg_event::MsgEvent;
pub use msg_send_from_kovi_event::MsgSendFromKoviEvent;
pub use msg_send_from_server_event::MsgSendFromServerEvent;
pub use notice_event::NoticeEvent;
pub use private_msg_event::PrivateMsgEvent;
pub use request_event::RequestEvent;

pub mod admin_msg_event;
pub mod group_msg_event;
pub mod lifecycle_event;
pub mod msg_event;
pub mod msg_send_from_kovi_event;
pub mod msg_send_from_server_event;
pub mod notice_event;
pub mod private_msg_event;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostType {
    Message,
    Notice,
    Request,
    MetaEvent,
    MessageSent,

    Other(String),
}

impl<'de> Deserialize<'de> for PostType {
    fn deserialize<D>(deserializer: D) -> Result<PostType, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let post_type = match s.as_str() {
            "message" => PostType::Message,
            "notice" => PostType::Notice,
            "request" => PostType::Request,
            "meta_event" => PostType::MetaEvent,
            "message_sent" => PostType::MessageSent,
            _ => PostType::Other(s),
        };
        Ok(post_type)
    }
}

/// 满足此 trait 即可在Kovi运行时中监听并处理
///
/// # Warning
///
/// 最好不要阻塞解析事件，如果目标信息需要阻塞获取，请通知用户由用户处理，而非由事件解析器阻塞
///
/// 在 Kovi 0.12.4 之后，事件是并发解析的，所以阻塞解析事件并不会阻塞其他事件的解析，虽然如此，仍然不建议阻塞解析事件
pub trait Event: Any + Send + Sync {
    /// 解析事件
    ///
    /// 传入三个东西，按需所取。
    ///  - InternalEvent 内部消息，包含OneBot消息与由框架发出去的Api消息
    ///  - 借用的bot信息，可以通过 `BotInformation` 获取 `Bot` 相关的信息，例如管理员是谁。
    ///  - 借用的api发送通道，可以通过 `api_tx.clone()` 来让事件可以发送 api
    ///
    /// 如果认为此 json 不符合事件要求，请返回 `None`。
    ///
    /// 在一个消息周期内，Kovi 运行时会缓存此事件。
    ///
    /// 不需要的信息用 `_` 忽略，例如：
    ///
    /// ```ignore
    /// 
    /// impl Event for LifecycleEvent {
    ///     fn de(
    ///         event: &InternalEvent,
    ///         _: &BotInformation,
    ///         _: &tokio::sync::mpsc::Sender<ApiAndOneshot>,
    ///     ) -> Option<Self>
    ///     where
    ///         Self: Sized,
    ///     {
    ///         let InternalEvent::OneBotEvent(json_str) = event else {
    ///             return None;
    ///         };
    ///         let event: LifecycleEvent = serde_json::from_str(json_str).ok()?;
    ///         if event.meta_event_type == "lifecycle" {
    ///             Some(event)
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Warning
    ///
    /// 最好不要阻塞解析事件，如果目标信息需要阻塞获取，请通知用户由用户处理，而非由事件解析器阻塞
    ///
    /// 在 Kovi 0.12.4 之后，事件是并发解析的，所以阻塞解析事件并不会阻塞其他事件的解析，虽然如此，仍然不建议阻塞解析事件
    ///
    /// 可以使用类似于 `MsgSendFromKoviEvent` 的实现，将所需的交给用户就行。
    ///
    /// ```ignore
    /// 
    /// pub struct MsgSendFromKoviEvent {
    ///     pub event_type: MsgSendFromKoviType,
    ///     pub send_api: SendApi,
    ///     pub res: Result<ApiReturn, ApiReturn>,
    /// }
    /// ```
    fn de(
        event: &InternalEvent,
        bot_info: &BotInformation,
        api_tx: &tokio::sync::mpsc::Sender<ApiAndOneshot>,
    ) -> Option<Self>
    where
        Self: Sized;
}

/// 事件
pub enum InternalEvent {
    /// 来自OneBot的事件
    OneBotEvent(String),
    /// 来自Kovi发送给服务端并包含了返回结果
    OneBotApiEvent(ApiAndRuturn),
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
