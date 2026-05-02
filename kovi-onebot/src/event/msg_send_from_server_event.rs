use super::{Anonymous, Sender};
use kovi::bot::runtimebot::{CanSendApi, send_api_request_with_forget};
use kovi::bot::{BotInformation, SendApi};
use kovi::error::EventBuildError;
use kovi::event::{Event, InternalEvent};
use kovi::types::ApiAndOptOneshot;
use log::info;
use serde::Serialize;
use serde_json::value::Index;
use serde_json::{self, Value, json};
use tokio::sync::mpsc;

#[cfg(feature = "cqstring")]
use crate::bot::message::CQMessage;
use crate::event::{MsgEvent, PostType, RepliableEvent, UniversalMessage};
use crate::onebot_message::OneBotMessage;
use kovi::message::Message as KoviMessage;

#[derive(Debug, Clone)]
pub struct MsgSendFromServerEvent {
    /// 事件发生的时间戳
    pub time: i64,
    /// 收到事件的机器人 登陆号
    pub self_id: i64,
    /// 上报类型
    pub post_type: PostType,
    /// 消息类型
    pub message_type: String,
    /// 消息子类型，如果是好友则是 friend，如果是群临时会话则是 group
    pub sub_type: String,
    /// 消息内容
    pub message: KoviMessage,
    /// 消息 ID
    pub message_id: i32,
    /// 群号
    pub group_id: Option<i64>,
    /// 发送者号
    pub user_id: i64,
    /// 匿名信息，如果不是匿名消息则为 null
    pub anonymous: Option<Anonymous>,
    /// 原始消息内容
    pub raw_message: String,
    /// 字体
    pub font: i32,
    /// 发送人信息
    pub sender: Sender,

    /// 处理过的纯文本，如果是纯图片或无文本，此处为None
    pub text: Option<String>,
    /// 处理过的文本，会解析成人类易读形式，里面会包含\[image\]\[face\]等解析后字符串
    pub human_text: String,
    /// 原始的onebot消息，已处理成json格式
    pub original_json: Value,

    /// 不推荐的消息发送方式
    pub api_tx: mpsc::Sender<ApiAndOptOneshot>,
}

impl Event for MsgSendFromServerEvent {
    fn de(
        event: &InternalEvent,
        _: &BotInformation,
        api_tx: &mpsc::Sender<ApiAndOptOneshot>,
    ) -> Option<Self> {
        let InternalEvent::OneBotEvent(json) = event else {
            return None;
        };

        let event = Self::new(api_tx.clone(), json.clone()).ok()?;

        Some(event)
    }
}

impl MsgSendFromServerEvent {
    fn new(
        api_tx: mpsc::Sender<ApiAndOptOneshot>,
        json: Value,
    ) -> Result<MsgSendFromServerEvent, EventBuildError> {
        let msg_event = MsgEvent::new(api_tx, json)?;

        if msg_event.post_type != PostType::MessageSent {
            return Err(EventBuildError::ParseError(
                "MsgSendFromServerEvent Not message_sent".to_string(),
            ));
        }

        Ok(MsgSendFromServerEvent {
            time: msg_event.time,
            self_id: msg_event.self_id,
            post_type: msg_event.post_type,
            message_type: msg_event.message_type,
            sub_type: msg_event.sub_type,
            message: msg_event.message,
            message_id: msg_event.message_id,
            group_id: msg_event.group_id,
            user_id: msg_event.user_id,
            anonymous: msg_event.anonymous,
            raw_message: msg_event.raw_message,
            font: msg_event.font,
            sender: msg_event.sender,
            text: msg_event.text,
            human_text: msg_event.human_text,
            original_json: msg_event.original_json,
            api_tx: msg_event.api_tx,
        })
    }
}

impl MsgSendFromServerEvent {
    /// 直接从原始的 Json Value 获取某值
    ///
    /// # example
    ///
    /// ```ignore
    /// use kovi::PluginBuilder;
    ///
    /// PluginBuilder::on_msg(|event| async move {
    ///     let time = event.get("time").and_then(|v| v.as_i64()).unwrap();
    ///
    ///     assert_eq!(time, event.time);
    /// });
    /// ```
    pub fn get<I: Index>(&self, index: I) -> Option<&Value> {
        self.original_json.get(index)
    }
}

impl<I> std::ops::Index<I> for MsgSendFromServerEvent
where
    I: Index,
{
    type Output = Value;

    fn index(&self, index: I) -> &Self::Output {
        &self.original_json[index]
    }
}

impl MsgSendFromServerEvent {
    fn reply_builder<M>(&self, msg: M, auto_escape: bool) -> SendApi
    where
        M: Into<OneBotMessage>,
    {
        RepliableEvent::reply_builder(self, msg, auto_escape)
    }

    #[cfg(not(feature = "cqstring"))]
    pub fn reply<T>(&self, msg: T)
    where
        KoviMessage: From<T>,
        T: Serialize,
    {
        RepliableEvent::reply(self, msg)
    }

    #[cfg(feature = "cqstring")]
    pub fn reply<T>(&self, msg: T)
    where
        CQMessage: From<T>,
        T: Serialize,
    {
        RepliableEvent::reply(self, msg)
    }

    #[cfg(not(feature = "cqstring"))]
    pub fn reply_and_quote<T>(&self, msg: T)
    where
        KoviMessage: From<T>,
        T: Serialize,
    {
        RepliableEvent::reply_and_quote(self, msg);
    }

    #[cfg(feature = "cqstring")]
    fn reply_and_quote<T>(&self, msg: T)
    where
        CQMessage: From<T>,
        T: Serialize,
    {
        RepliableEvent::reply_and_quote(self, msg);
    }

    #[cfg(feature = "cqstring")]
    fn reply_text<T>(&self, msg: T)
    where
        String: From<T>,
        T: Serialize,
    {
        RepliableEvent::reply_text(self, msg)
    }

    pub fn get_text(&self) -> String {
        RepliableEvent::get_text(self)
    }

    pub fn get_sender_nickname(&self) -> String {
        RepliableEvent::get_sender_nickname(self)
    }

    pub fn borrow_text(&self) -> Option<&str> {
        RepliableEvent::borrow_text(self)
    }

    pub fn is_group(&self) -> bool {
        UniversalMessage::is_group(self)
    }

    pub fn is_private(&self) -> bool {
        UniversalMessage::is_private(self)
    }
}

impl UniversalMessage for MsgSendFromServerEvent {
    fn is_group(&self) -> bool {
        self.group_id.is_some()
    }

    fn is_private(&self) -> bool {
        self.group_id.is_none()
    }
}

impl RepliableEvent for MsgSendFromServerEvent {
    fn reply_builder<M>(&self, msg: M, auto_escape: bool) -> SendApi
    where
        M: Into<OneBotMessage>,
    {
        let msg = msg.into();
        if self.is_private() {
            SendApi::new(
                "send_msg",
                json!({
                    "message_type":"private",
                "user_id":self.user_id,
                "message":msg,
                "auto_escape":auto_escape,
                }),
            )
        } else {
            SendApi::new(
                "send_msg",
                json!({
                    "message_type":"group",
                    "group_id":self.group_id.expect("unreachable"),
                    "message":msg,
                    "auto_escape":auto_escape,
                }),
            )
        }
    }

    #[cfg(not(feature = "cqstring"))]
    /// 快速回复消息
    fn reply<T>(&self, msg: T)
    where
        KoviMessage: From<T>,
        T: Serialize,
    {
        let msg = KoviMessage::from(msg);
        let mut nickname = self.get_sender_nickname();
        nickname.insert(0, ' ');
        let id = &self.sender.user_id;
        let message_type = &self.message_type;
        let group_id = match &self.group_id {
            Some(v) => format!(" {v}"),
            None => "".to_string(),
        };
        let human_msg = msg.to_human_string();
        info!("[reply] [to {message_type}{group_id}{nickname} {id}]: {human_msg}");

        let send_msg = self.reply_builder(msg, false);
        send_api_request_with_forget(&self.api_tx, send_msg)
    }

    #[cfg(feature = "cqstring")]
    /// 快速回复消息
    fn reply<T>(&self, msg: T)
    where
        CQMessage: From<T>,
        T: Serialize,
    {
        let msg = CQMessage::from(msg);
        let send_msg = self.reply_builder(&msg, false);
        let mut nickname = self.get_sender_nickname();
        nickname.insert(0, ' ');
        let id = &self.sender.user_id;
        let message_type = &self.message_type;
        let group_id = match &self.group_id {
            Some(v) => format!(" {v}"),
            None => "".to_string(),
        };
        let human_msg = Message::from(msg).to_human_string();
        info!("[reply] [to {message_type}{group_id}{nickname} {id}]: {human_msg}");
        send_api_request_with_forget(&self.api_tx, send_msg);
    }

    #[cfg(not(feature = "cqstring"))]
    /// 快速回复消息并且**引用**
    fn reply_and_quote<T>(&self, msg: T)
    where
        KoviMessage: From<T>,
        T: Serialize,
    {
        let msg = KoviMessage::from(msg).add_reply(self.message_id);
        let mut nickname = self.get_sender_nickname();
        nickname.insert(0, ' ');
        let id = &self.sender.user_id;
        let message_type = &self.message_type;
        let group_id = match &self.group_id {
            Some(v) => format!(" {v}"),
            None => "".to_string(),
        };
        let human_msg = msg.to_human_string();
        info!("[reply] [to {message_type}{group_id}{nickname} {id}]: {human_msg}");

        let send_msg = self.reply_builder(msg, false);
        send_api_request_with_forget(&self.api_tx, send_msg);
    }

    #[cfg(feature = "cqstring")]
    /// 快速回复消息并且**引用**
    fn reply_and_quote<T>(&self, msg: T)
    where
        CQMessage: From<T>,
        T: Serialize,
    {
        let msg = CQMessage::from(msg).add_reply(self.message_id);
        let send_msg = self.reply_builder(&msg, false);

        let mut nickname = self.get_sender_nickname();
        nickname.insert(0, ' ');
        let id = &self.sender.user_id;
        let message_type = &self.message_type;
        let group_id = match &self.group_id {
            Some(v) => format!(" {v}"),
            None => "".to_string(),
        };
        let human_msg = Message::from(msg).to_human_string();
        info!("[reply] [to {message_type}{group_id}{nickname} {id}]: {human_msg}");
        send_api_request_with_forget(&self.api_tx, send_msg);
    }

    #[cfg(feature = "cqstring")]
    /// 快速回复消息，并且**kovi不进行解析，直接发送此字符串**
    fn reply_text<T>(&self, msg: T)
    where
        String: From<T>,
        T: Serialize,
    {
        let send_msg = self.reply_builder(&msg, true);
        let mut nickname = self.get_sender_nickname();
        nickname.insert(0, ' ');
        let id = &self.sender.user_id;
        let message_type = &self.message_type;
        let group_id = match &self.group_id {
            Some(v) => format!(" {v}"),
            None => "".to_string(),
        };
        let msg = String::from(msg);
        info!("[reply] [to {message_type}{group_id} {nickname} {id}]: {msg}");
        send_api_request_with_forget(&self.api_tx, send_msg);
    }

    /// 便捷获取文本，如果没有文本则会返回空字符串，如果只需要借用，请使用 `borrow_text()`
    fn get_text(&self) -> String {
        match self.text.clone() {
            Some(v) => v,
            None => "".to_string(),
        }
    }

    /// 便捷获取发送者昵称，如果无名字，此处为空字符串
    fn get_sender_nickname(&self) -> String {
        if let Some(v) = &self.sender.nickname {
            v.clone()
        } else {
            "".to_string()
        }
    }

    /// 借用 event 的 text，只是做了一下self.text.as_deref()的包装
    fn borrow_text(&self) -> Option<&str> {
        self.text.as_deref()
    }
}

impl CanSendApi for MsgSendFromServerEvent {
    fn __get_api_tx(&self) -> &tokio::sync::mpsc::Sender<kovi::types::ApiAndOptOneshot> {
        &self.api_tx
    }
}
