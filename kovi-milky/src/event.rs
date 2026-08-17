use kovi::event::Event;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub use admin_msg_event::AdminMsgEvent;
pub use friend_file_upload::FriendFileUploadEvent;
pub use friend_msg_event::FriendMsgEvent;
pub use friend_nudge::FriendNudgeEvent;
pub use friend_request::FriendRequestEvent;
pub use group_admin_change::GroupAdminChangeEvent;
pub use group_essence_message_change::GroupEssenceMessageChangeEvent;
pub use group_file_upload::GroupFileUploadEvent;
pub use group_invitation::GroupInvitationEvent;
pub use group_invited_join_request::GroupInvitedJoinRequestEvent;
pub use group_join_request::GroupJoinRequestEvent;
pub use group_member_decrease::GroupMemberDecreaseEvent;
pub use group_member_increase::GroupMemberIncreaseEvent;
pub use group_message_reaction::{GroupMessageReactionEvent, ReactionType};
pub use group_msg_event::GroupMsgEvent;
pub use group_mute::GroupMuteEvent;
pub use group_name_change::GroupNameChangeEvent;
pub use group_nudge::GroupNudgeEvent;
pub use group_whole_mute::GroupWholeMuteEvent;
pub use message_recall::MessageRecallEvent;
pub use msg_event::{MessageReceiveEventData, MessageScene, MsgEvent};
pub use msg_send_from_kovi_event::{MsgSendFromKoviEvent, MsgSendFromKoviType};
pub use peer_pin_change::PeerPinChangeEvent;

pub mod admin_msg_event;
pub mod bot_offline;
pub mod friend_file_upload;
pub mod friend_msg_event;
pub mod friend_nudge;
pub mod friend_request;
pub mod group_admin_change;
pub mod group_essence_message_change;
pub mod group_file_upload;
pub mod group_invitation;
pub mod group_invited_join_request;
pub mod group_join_request;
pub mod group_member_decrease;
pub mod group_member_increase;
pub mod group_message_reaction;
pub mod group_msg_event;
pub mod group_mute;
pub mod group_name_change;
pub mod group_nudge;
pub mod group_whole_mute;
pub mod message_recall;
pub mod msg_event;
pub mod msg_send_from_kovi_event;
pub mod peer_pin_change;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroupMemberEntity {
    /// 用户 QQ 号
    pub user_id: i64,
    /// 用户昵称
    pub nickname: String,
    /// 用户性别，可能值：male female unknown
    pub sex: Sex,
    /// 群号
    pub group_id: i64,
    /// 成员备注
    pub card: String,
    /// 专属头衔
    pub title: String,
    /// 群等级，注意和 QQ 等级区分
    pub level: i32,
    /// 权限等级，可能值：owner admin member
    pub role: String,
    /// 入群时间，Unix 时间戳（秒）
    pub join_time: i64,
    /// 最后发言时间，Unix 时间戳（秒）
    pub last_sent_time: i64,
    /// 禁言结束时间，Unix 时间戳（秒）
    pub shut_up_end_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupEntity {
    /// 群号
    pub group_id: i64,
    /// 群名称
    pub group_name: String,
    /// 群成员数量
    pub member_count: i32,
    /// 群容量
    pub max_member_count: i32,
    /// 群备注
    pub remark: String,
    /// 群创建时间，Unix 时间戳（秒）
    pub created_time: i64,
    /// 群简介
    pub description: String,
    /// 加群验证问题
    pub question: String,
    /// 群公告预览
    pub announcement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendEntity {
    /// 用户 QQ 号
    pub user_id: i64,
    /// 用户昵称
    pub nickname: String,
    /// 用户性别
    pub sex: Sex,
    /// 用户 QID
    pub qid: String,
    /// 好友备注
    pub remark: String,
    /// 好友分组
    pub category: FriendCategoryEntity,
}

/// 用户性别，可能值：male female unknown
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sex {
    Male,
    Female,
    Unknown,
}

/// 好友分组
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendCategoryEntity {
    /// 好友分组 ID
    pub category_id: i32,
    /// 好友分组名称
    pub category_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: DeserializeOwned"))]
pub struct MilkyEvent<T = serde_json::Value>
where
    T: Sized,
{
    pub event_type: String,
    pub time: i64,
    pub self_id: i64,
    pub data: T,
}

impl Event for MilkyEvent<serde_json::Value> {
    fn de(
        event: &kovi::event::InternalEvent,
        _bot_info: &kovi::bot::BotInformation,
        _api_tx: &tokio::sync::mpsc::Sender<kovi::types::ApiAndOptOneshot>,
    ) -> Option<Self>
    where
        Self: Sized,
    {
        if let kovi::event::InternalEvent::DriverEvent(data) = event {
            serde_json::from_value(data.clone()).ok()
        } else {
            None
        }
    }
}

/// 满足此 trait 即可判断消息来源
pub trait UniversalMessage {
    fn is_group(&self) -> bool;

    fn is_private(&self) -> bool;

    fn is_temp_chat(&self) -> bool;
}
