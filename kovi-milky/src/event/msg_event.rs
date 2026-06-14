use crate::event::{FriendEntity, GroupEntity, GroupMemberEntity, MilkyEvent, UniversalMessage};
use crate::message_trait::MessageRegistrar as _;
use crate::milky_message::MilkyMessage;
use kovi::bot::runtimebot::{CanSendApi, send_api_request_with_forget};
use kovi::bot::{BotInformation, SendApi};
use kovi::error::EventBuildError;
use kovi::event::id::ref_id::RefID;
use kovi::event::{Event, InternalEvent, MessageEventTrait, MessageEventUtil, RepliableEvent};
use kovi::message::Message as KoviMessage;
use kovi::types::ApiAndOptOneshot;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serde_json::{self, Value, json};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageScene {
    Friend,
    Group,
    Temp,
}

impl AsRef<str> for MessageScene {
    fn as_ref(&self) -> &str {
        match self {
            MessageScene::Friend => "friend",
            MessageScene::Group => "group",
            MessageScene::Temp => "temp",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageReceiveEventData {
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

    pub friend: Option<FriendEntity>,
    pub group: Option<GroupEntity>,
    pub group_member: Option<GroupMemberEntity>,

    pub message: KoviMessage,
    /// 处理过的纯文本，如果是纯图片或无文本，此处为None
    pub text: Option<String>,
    /// 处理过的文本，会解析成人类易读形式，里面会包含\[image\]\[face\]等解析后字符串
    pub human_text: String,

    /// 不推荐的消息发送方式
    pub api_tx: mpsc::Sender<ApiAndOptOneshot>,
}

pub type MsgEvent = MilkyEvent<MessageReceiveEventData>;

impl MessageEventTrait for MsgEvent {
    fn get_sender_name(&self) -> Option<&str> {
        match self.data.message_scene {
            MessageScene::Friend => Some(
                self.data
                    .friend
                    .as_ref()
                    .expect("unreachable")
                    .nickname
                    .as_str(),
            ),

            MessageScene::Group => Some(
                self.data
                    .group_member
                    .as_ref()
                    .expect("unreachable")
                    .nickname
                    .as_str(),
            ),
            MessageScene::Temp => None,
        }
    }

    fn get_message(&self) -> &KoviMessage {
        &self.data.message
    }

    fn get_sender_id(&self) -> RefID<'_> {
        RefID::new(&self.data.sender_id)
    }

    fn get_group_id(&self) -> Option<RefID<'_>> {
        self.data.group.as_ref().map(|v| RefID::new(&v.group_id))
    }

    fn get_message_type_str(&self) -> Option<&str> {
        Some(self.data.message_scene.as_ref())
    }
}

impl Event for MsgEvent {
    fn de(
        event: &InternalEvent,
        _: &BotInformation,
        api_tx: &mpsc::Sender<ApiAndOptOneshot>,
    ) -> Option<Self> {
        let InternalEvent::DriverEvent(json) = event else {
            return None;
        };

        Self::new(api_tx, json).ok()
    }
}

impl MsgEvent {
    pub(crate) fn new(
        api_tx: &mpsc::Sender<ApiAndOptOneshot>,
        temp: &Value,
    ) -> Result<MsgEvent, EventBuildError> {
        #[derive(Serialize, Deserialize, Debug, Clone)]
        pub struct TempMessageReceiveEventData {
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

            pub friend: Option<FriendEntity>,
            pub group: Option<GroupEntity>,
            pub group_member: Option<GroupMemberEntity>,
        }

        type TempMsgEvent = MilkyEvent<TempMessageReceiveEventData>;

        let temp_msg_event: TempMsgEvent = serde_json::from_value(temp.clone())
            .map_err(|e| EventBuildError::ParseError(e.to_string()))?;

        debug!("{temp_msg_event:?}");

        let message = KoviMessage::from(temp_msg_event.data.segments.clone());
        let text = get_text_from_message(&message);
        let human_text = message.to_human_string();

        let message_receive_event_data = MessageReceiveEventData {
            time: temp_msg_event.data.time,
            message_scene: temp_msg_event.data.message_scene,
            message_seq: temp_msg_event.data.message_seq,
            peer_id: temp_msg_event.data.peer_id,
            sender_id: temp_msg_event.data.sender_id,
            segments: temp_msg_event.data.segments,
            friend: temp_msg_event.data.friend,
            group: temp_msg_event.data.group,
            group_member: temp_msg_event.data.group_member,
            message,
            text,
            human_text,
            api_tx: api_tx.clone(),
        };
        let event = MsgEvent {
            data: message_receive_event_data,

            event_type: temp_msg_event.event_type,
            time: temp_msg_event.time,
            self_id: temp_msg_event.self_id,
        };

        Ok(event)
    }
}

fn get_text_from_message(message: &KoviMessage) -> Option<String> {
    let texts = message.get("text");
    if texts.is_empty() {
        return None;
    }
    let text = texts
        .into_iter()
        .filter_map(|v| {
            v.data
                .get("text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect::<Vec<String>>()
        .join("\n");

    Some(text)
}

impl MsgEvent {
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

impl UniversalMessage for MsgEvent {
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

impl RepliableEvent for MsgEvent {
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
        let group_id = match &self.get_group_id() {
            Some(v) => format!(" {v}"),
            None => "".to_string(),
        };
        let human_msg = msg.to_human_string();
        info!("[reply] [to {message_type}{group_id}{nickname} {id}]: {human_msg}");

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
        let group_id = match &self.get_group_id() {
            Some(v) => format!(" {v}"),
            None => "".to_string(),
        };
        let human_msg = msg.to_human_string();
        info!("[reply] [to {message_type}{group_id}{nickname} {id}]: {human_msg}");

        let send_msg = self.reply_builder(msg);
        send_api_request_with_forget(&self.data.api_tx, send_msg);
    }
}

impl MsgEvent {
    fn reply_builder<M>(&self, msg: M) -> SendApi
    where
        M: Into<MilkyMessage>,
    {
        let msg = msg.into();

        match self.data.message_scene {
            MessageScene::Friend | MessageScene::Temp => SendApi::new(
                "send_private_message",
                json!({
                "user_id": self.data.sender_id,
                "message": msg,
                }),
            ),
            MessageScene::Group => SendApi::new(
                "send_group_message",
                json!({
                    "group_id": self
                        .data
                        .group
                        .as_ref()
                        .map(|v| v.group_id)
                        .expect("unreachable"),
                    "message": msg,
                }),
            ),
        }
    }
}

impl MessageEventUtil for MsgEvent {
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

impl CanSendApi for MsgEvent {
    fn __get_api_tx(&self) -> &tokio::sync::mpsc::Sender<kovi::types::ApiAndOptOneshot> {
        &self.data.api_tx
    }
}
