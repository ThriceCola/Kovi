use crate::event::msg_event::{MessageScene, MsgEvent};
use crate::event::{FriendEntity, MilkyEvent, UniversalMessage};
use crate::message_trait::MessageRegistrar as _;
use crate::milky_message::MilkyMessage;
use kovi::bot::runtimebot::{CanSendApi, send_api_request_with_forget};
use kovi::bot::{BotInformation, SendApi};
use kovi::error::EventBuildError;
use kovi::event::id::ref_id::RefID;
use kovi::event::{Event, InternalEvent, MessageEventTrait, MessageEventUtil, RepliableEvent};
use kovi::message::Message as KoviMessage;
use kovi::types::ApiAndOptOneshot;
use log::info;
use serde::Serialize;
use serde_json::{self, Value, json};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct FriendMessageReceiveEventData {
    /// 消息 Unix 时间戳（秒）
    pub time: i64,
    pub message_scene: MessageScene,

    /// 消息 ID
    pub message_seq: i64,
    /// 好友 QQ 号或群号
    pub peer_id: Option<i64>,
    /// 发送者号
    pub sender_id: i64,

    /// 消息内容
    pub segments: MilkyMessage,

    pub friend: FriendEntity,

    pub message: KoviMessage,
    /// 处理过的纯文本，如果是纯图片或无文本，此处为None
    pub text: Option<String>,
    /// 处理过的文本，会解析成人类易读形式，里面会包含\[image\]\[face\]等解析后字符串
    pub human_text: String,

    /// 不推荐的消息发送方式
    pub api_tx: mpsc::Sender<ApiAndOptOneshot>,
}

pub type FriendMsgEvent = MilkyEvent<FriendMessageReceiveEventData>;
pub type PrivateMsgEvent = FriendMsgEvent;

impl MessageEventTrait for FriendMsgEvent {
    fn get_sender_name(&self) -> Option<&str> {
        Some(self.data.friend.nickname.as_str())
    }

    fn get_message(&self) -> &KoviMessage {
        &self.data.message
    }

    fn get_sender_id(&self) -> RefID<'_> {
        RefID::new(&self.data.sender_id)
    }

    fn get_group_id(&self) -> Option<RefID<'_>> {
        None
    }

    fn get_message_type_str(&self) -> Option<&str> {
        Some(self.data.message_scene.as_ref())
    }
}

impl Event for FriendMsgEvent {
    fn de(
        event: &InternalEvent,
        _: &BotInformation,
        api_tx: &mpsc::Sender<ApiAndOptOneshot>,
    ) -> Option<Self> {
        let InternalEvent::DriverEvent(json) = event else {
            return None;
        };

        Self::new(api_tx.clone(), json.clone()).ok()
    }
}

impl TryFrom<MsgEvent> for FriendMsgEvent {
    type Error = EventBuildError;

    fn try_from(event: MsgEvent) -> Result<Self, Self::Error> {
        let data = event.data;
        Ok(FriendMsgEvent {
            event_type: event.event_type,
            time: event.time,
            self_id: event.self_id,
            data: FriendMessageReceiveEventData {
                time: data.time,
                message_scene: data.message_scene,
                message_seq: data.message_seq,
                peer_id: data.peer_id,
                sender_id: data.sender_id,
                segments: data.segments,
                friend: data
                    .friend
                    .ok_or(EventBuildError::ParseError("missing friend".to_string()))?,

                message: data.message,
                text: data.text,
                human_text: data.human_text,
                api_tx: data.api_tx,
            },
        })
    }
}

impl FriendMsgEvent {
    pub(crate) fn new(
        api_tx: mpsc::Sender<ApiAndOptOneshot>,
        temp: Value,
    ) -> Result<FriendMsgEvent, EventBuildError> {
        let event = MsgEvent::new(api_tx, temp)?;

        if event.data.message_scene != MessageScene::Friend {
            return Err(EventBuildError::ParseError(
                "message_scene must be Friend".to_string(),
            ));
        }

        let event = FriendMsgEvent::try_from(event)?;

        Ok(event)
    }
}

impl FriendMsgEvent {
    pub fn reply<T>(&self, msg: T)
    where
        KoviMessage: From<T>,
        T: Serialize,
    {
        RepliableEvent::reply(self, msg)
    }

    pub fn reply_and_quote<T>(&self, msg: T)
    where
        KoviMessage: From<T>,
        T: Serialize,
    {
        RepliableEvent::reply_and_quote(self, msg);
    }

    pub fn get_text(&self) -> String {
        MessageEventUtil::get_text(self)
    }

    pub fn get_sender_nickname(&self) -> String {
        MessageEventUtil::get_sender_nickname(self)
    }

    pub fn borrow_text(&self) -> Option<&str> {
        MessageEventUtil::borrow_text(self)
    }

    pub fn is_group(&self) -> bool {
        UniversalMessage::is_group(self)
    }

    pub fn is_private(&self) -> bool {
        UniversalMessage::is_private(self)
    }
}

impl UniversalMessage for FriendMsgEvent {
    fn is_group(&self) -> bool {
        self.data.message_scene == MessageScene::Group
    }

    fn is_private(&self) -> bool {
        self.data.message_scene == MessageScene::Friend
    }

    fn is_temp_chat(&self) -> bool {
        self.data.message_scene == MessageScene::Temp
    }
}

impl RepliableEvent for FriendMsgEvent {
    /// 快速回复消息
    fn reply<T>(&self, msg: T)
    where
        KoviMessage: From<T>,
        T: Serialize,
    {
        let msg = KoviMessage::from(msg);
        let mut nickname = self.get_sender_nickname();
        nickname.insert(0, ' ');
        let id = &self.get_sender_id();
        let message_type = self.get_message_type_str().unwrap_or_default();
        let human_msg = msg.to_human_string();
        info!("[reply] [to {message_type}{nickname} {id}]: {human_msg}");

        let send_msg = self.reply_builder(msg);
        send_api_request_with_forget(&self.data.api_tx, send_msg)
    }

    /// 快速回复消息并且**引用**
    fn reply_and_quote<T>(&self, msg: T)
    where
        KoviMessage: From<T>,
        T: Serialize,
    {
        let msg = KoviMessage::from(msg).add_reply(self.data.message_seq);
        let mut nickname = self.get_sender_nickname();
        nickname.insert(0, ' ');
        let id = &self.get_sender_id();
        let message_type = self.get_message_type_str().unwrap_or_default();
        let human_msg = msg.to_human_string();
        info!("[reply] [to {message_type}{nickname} {id}]: {human_msg}");

        let send_msg = self.reply_builder(msg);
        send_api_request_with_forget(&self.data.api_tx, send_msg);
    }
}

impl FriendMsgEvent {
    fn reply_builder<M>(&self, msg: M) -> SendApi
    where
        M: Into<MilkyMessage>,
    {
        let msg = msg.into();
        SendApi::new(
            "send_private_message",
            json!({
            "user_id": self.data.sender_id,
            "message": msg,
            }),
        )
    }
}

impl MessageEventUtil for FriendMsgEvent {
    /// 便捷获取文本，如果没有文本则会返回空字符串，如果只需要借用，请使用 `borrow_text()`
    fn get_text(&self) -> String {
        match self.data.text.clone() {
            Some(v) => v,
            None => "".to_string(),
        }
    }

    /// 便捷获取发送者昵称，如果无名字，此处为空字符串
    fn get_sender_nickname(&self) -> String {
        if let Some(v) = &self.get_sender_name() {
            v.to_string()
        } else {
            "".to_string()
        }
    }

    /// 借用 event 的 text，只是做了一下self.text.as_deref()的包装
    fn borrow_text(&self) -> Option<&str> {
        self.data.text.as_deref()
    }
}

impl CanSendApi for FriendMsgEvent {
    fn __get_api_tx(&self) -> &tokio::sync::mpsc::Sender<kovi::types::ApiAndOptOneshot> {
        &self.data.api_tx
    }
}
