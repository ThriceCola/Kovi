#![feature(async_fn_traits, unboxed_closures)]

pub mod driver;
pub mod event;
pub mod event_registrar;
pub mod message_trait;
pub mod onebot_api;
pub mod onebot_message;

// ── Driver ──
pub use driver::OneBotDriver;
pub use driver::config::{Host, OneBotDriverConfig, Server, load_local_conf};

// ── Events ──
pub use event::{
    AdminMsgEvent, GroupMsgEvent, MsgEvent, MsgSendFromKoviEvent, MsgSendFromServerEvent,
    NoticeEvent, PrivateMsgEvent, RepliableEvent, RequestEvent,
};
pub use event_registrar::EventRegistrar;
pub use onebot_message::OneBotMessage;

// ── Message builder ──
pub use message_trait::MessageRegistrar;

// ── API traits ──
pub use onebot_api::OnebotTrait;
