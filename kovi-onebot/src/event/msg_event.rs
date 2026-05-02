use super::{Anonymous, Sender};
use crate::event::{PostType, RepliableEvent, Sex, UniversalMessage};
use crate::onebot_message::{OneBotMessage, cq_to_arr_inner};
use kovi::bot::runtimebot::{CanSendApi, send_api_request_with_forget};
use kovi::bot::{BotInformation, SendApi};
use kovi::error::EventBuildError;
use kovi::event::id::ref_id::RefID;
use kovi::event::{Event, InternalEvent, MessageEventTrait};
use kovi::message::Message as KoviMessage;
use kovi::types::ApiAndOptOneshot;
use log::{debug, info};
use serde::Serialize;
use serde_json::value::Index;
use serde_json::{self, Value, json};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct MsgEvent {
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

impl MessageEventTrait for MsgEvent {
    fn get_sender_name(&self) -> Option<&str> {
        self.sender.nickname.as_ref().map(|s| s.as_str())
    }

    fn get_message(&self) -> &KoviMessage {
        &self.message
    }

    fn get_message_type_str(&self) -> Option<&str> {
        Some(&self.message_type)
    }

    fn get_sender_id(&self) -> RefID<'_> {
        RefID::new(&self.sender.user_id)
    }

    fn get_ref_group_id(&self) -> Option<RefID<'_>> {
        self.group_id.as_ref().map(RefID::new)
    }
}

impl Event for MsgEvent {
    fn de(
        event: &InternalEvent,
        _: &BotInformation,
        api_tx: &mpsc::Sender<ApiAndOptOneshot>,
    ) -> Option<Self> {
        let InternalEvent::OneBotEvent(json) = event else {
            return None;
        };

        Self::new(api_tx.clone(), json.clone()).ok()
    }
}

impl MsgEvent {
    pub(crate) fn new(
        api_tx: mpsc::Sender<ApiAndOptOneshot>,
        temp: Value,
    ) -> Result<MsgEvent, EventBuildError> {
        let temp_object = temp.as_object().ok_or(EventBuildError::ParseError(
            "Invalid JSON object".to_string(),
        ))?;

        let temp_sender = temp_object
            .get("sender")
            .and_then(|v| v.as_object())
            .ok_or(EventBuildError::ParseError(
                "Invalid sender object".to_string(),
            ))?;

        let sender = {
            Sender {
                user_id: temp_sender
                    .get("user_id")
                    .and_then(|v| v.as_i64())
                    .ok_or(EventBuildError::ParseError("Invalid user_id".to_string()))?,
                nickname: temp_sender.get("nickname").and_then(|v| {
                    if let Value::String(str) = v.clone() {
                        Some(str)
                    } else {
                        None
                    }
                }),
                card: temp_sender.get("card").and_then(|v| {
                    if let Value::String(str) = v.clone() {
                        Some(str)
                    } else {
                        None
                    }
                }),
                sex: if let Some(v) = temp_sender.get("sex").and_then(|v| v.as_str()) {
                    match v {
                        "male" => Some(Sex::Male),
                        "female" => Some(Sex::Female),
                        _ => None,
                    }
                } else {
                    None
                },
                age: temp_sender
                    .get("age")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                area: temp_sender.get("area").and_then(|v| {
                    if let Value::String(str) = v.clone() {
                        Some(str)
                    } else {
                        None
                    }
                }),
                level: temp_sender.get("level").and_then(|v| {
                    if let Value::String(str) = v.clone() {
                        Some(str)
                    } else {
                        None
                    }
                }),
                role: temp_sender.get("role").and_then(|v| {
                    if let Value::String(str) = v.clone() {
                        Some(str)
                    } else {
                        None
                    }
                }),
                title: temp_sender.get("title").and_then(|v| {
                    if let Value::String(str) = v.clone() {
                        Some(str)
                    } else {
                        None
                    }
                }),
            }
        };

        let group_id = temp_object.get("group_id").and_then(|v| v.as_i64());

        let message = if temp_object
            .get("message")
            .and_then(|v| v.as_array())
            .is_some()
        {
            let v = temp_object
                .get("message")
                .ok_or(EventBuildError::ParseError(
                    "Missing 'message' field".to_string(),
                ))?
                .as_array()
                .ok_or(EventBuildError::ParseError(
                    "Invalid 'message' array".to_string(),
                ))?
                .to_vec();
            OneBotMessage::from_vec_segment_value(v)
                .map_err(|e| EventBuildError::ParseError(format!("Parse error: {e}")))?
        } else {
            let str_v = temp_object["message"]
                .as_str()
                .ok_or(format!(
                    "message is not string:{:?}",
                    temp_object["message"]
                ))
                .map_err(|e| EventBuildError::ParseError(format!("Parse error: {e}")))?;
            let arr_v = cq_to_arr_inner(str_v);
            OneBotMessage::from_vec_segment_value(arr_v)
                .map_err(|e| EventBuildError::ParseError(format!("Parse error: {e}")))?
        };

        let anonymous: Option<Anonymous> =
            if temp_object.get("anonymous").is_none_or(|v| v.is_null()) {
                None
            } else {
                let anonymous = temp_object
                    .get("anonymous")
                    .ok_or(EventBuildError::ParseError(
                        "Invalid anonymous field".to_string(),
                    ))?
                    .clone();
                Some(
                    serde_json::from_value(anonymous)
                        .map_err(|e| EventBuildError::ParseError(e.to_string()))?,
                )
            };

        let text = {
            let mut text_vec = Vec::new();
            for msg in message.iter() {
                if msg.type_ == "text"
                    && let Some(text_value) = msg.data.get("text").and_then(|v| v.as_str())
                {
                    text_vec.push(text_value);
                };
            }
            if !text_vec.is_empty() {
                Some(text_vec.join("\n").trim().to_string())
            } else {
                None
            }
        };

        let message = KoviMessage::from(message);
        let event = MsgEvent {
            human_text: message.to_human_string(),
            time: temp_object
                .get("time")
                .and_then(|v| v.as_i64())
                .ok_or(EventBuildError::ParseError("Invalid time".to_string()))?,
            self_id: temp_object
                .get("self_id")
                .and_then(|v| v.as_i64())
                .ok_or(EventBuildError::ParseError("Invalid self_id".to_string()))?,
            post_type: temp_object
                .get("post_type")
                .and_then(|v| serde_json::from_value::<PostType>(v.clone()).ok())
                .ok_or(EventBuildError::ParseError("Invalid post_type".to_string()))?,
            message_type: temp_object
                .get("message_type")
                .and_then(|v| {
                    if let Value::String(str) = v.clone() {
                        Some(str)
                    } else {
                        None
                    }
                })
                .ok_or(EventBuildError::ParseError(
                    "Invalid message_type".to_string(),
                ))?,
            sub_type: temp_object
                .get("sub_type")
                .and_then(|v| {
                    if let Value::String(str) = v.clone() {
                        Some(str)
                    } else {
                        None
                    }
                })
                .ok_or(EventBuildError::ParseError("Invalid sub_type".to_string()))?,
            message,
            message_id: temp_object
                .get("message_id")
                .and_then(|v| v.as_i64())
                .ok_or(EventBuildError::ParseError(
                    "Invalid message_id".to_string(),
                ))? as i32,
            group_id,
            user_id: temp_object
                .get("user_id")
                .and_then(|v| v.as_i64())
                .ok_or(EventBuildError::ParseError("Invalid user_id".to_string()))?,
            anonymous,
            raw_message: temp_object
                .get("raw_message")
                .and_then(|v| {
                    if let Value::String(str) = v.clone() {
                        Some(str)
                    } else {
                        None
                    }
                })
                .ok_or(EventBuildError::ParseError(
                    "Invalid raw_message".to_string(),
                ))?,
            font: temp_object
                .get("font")
                .and_then(|v| v.as_i64())
                .ok_or(EventBuildError::ParseError("Invalid font".to_string()))?
                as i32,
            sender,
            api_tx,
            text,
            original_json: temp,
        };
        debug!("{event:?}");
        Ok(event)
    }
}

impl MsgEvent {
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

impl<I> std::ops::Index<I> for MsgEvent
where
    I: Index,
{
    type Output = Value;

    fn index(&self, index: I) -> &Self::Output {
        &self.original_json[index]
    }
}

impl MsgEvent {
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

impl UniversalMessage for MsgEvent {
    fn is_group(&self) -> bool {
        self.group_id.is_some()
    }

    fn is_private(&self) -> bool {
        self.group_id.is_none()
    }
}

impl RepliableEvent for MsgEvent {
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
                    "group_id":self.group_id.as_ref().expect("unreachable"),
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

impl CanSendApi for MsgEvent {
    fn __get_api_tx(&self) -> &tokio::sync::mpsc::Sender<kovi::types::ApiAndOptOneshot> {
        &self.api_tx
    }
}
