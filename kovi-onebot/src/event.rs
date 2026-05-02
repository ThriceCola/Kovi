use kovi::bot::SendApi;
use serde::{Deserialize, Serialize};

pub use admin_msg_event::AdminMsgEvent;
pub use group_msg_event::GroupMsgEvent;
use kovi::message::Message as KoviMessage;
pub use msg_event::MsgEvent;
pub use msg_send_from_kovi_event::MsgSendFromKoviEvent;
pub use msg_send_from_server_event::MsgSendFromServerEvent;
pub use notice_event::NoticeEvent;
pub use private_msg_event::PrivateMsgEvent;
pub use request_event::RequestEvent;

#[cfg(not(feature = "cqstring"))]
use crate::onebot_message::OneBotMessage;

pub mod admin_msg_event;
pub mod group_msg_event;
pub mod lifecycle_event;
pub mod msg_event;
pub mod msg_send_from_kovi_event;
pub mod msg_send_from_server_event;
pub mod notice_event;
pub mod private_msg_event;
pub mod request_event;

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

/// 满足此 trait 即可判断消息来源
pub trait UniversalMessage {
    fn is_group(&self) -> bool;

    fn is_private(&self) -> bool;
}

/// 满足此 trait 即可被回复
pub trait RepliableEvent {
    fn reply_builder<M>(&self, msg: M, auto_escape: bool) -> SendApi
    where
        M: Into<OneBotMessage>;

    #[cfg(not(feature = "cqstring"))]
    /// 快速回复消息
    fn reply<T>(&self, msg: T)
    where
        KoviMessage: From<T>,
        T: Serialize;

    /// 快速回复消息并且**引用**
    fn reply_and_quote<T>(&self, msg: T)
    where
        KoviMessage: From<T>,
        T: Serialize;

    /// 便捷获取文本，如果没有文本则会返回空字符串，如果只需要借用，请使用 `borrow_text()`
    fn get_text(&self) -> String;

    /// 便捷获取发送者昵称，如果无名字，此处为空字符串
    fn get_sender_nickname(&self) -> String;

    /// 借用 event 的 text，只是做了一下self.text.as_deref()的包装
    fn borrow_text(&self) -> Option<&str>;
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
