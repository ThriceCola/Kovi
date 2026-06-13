pub mod driver;
pub mod event;
pub mod event_registrar;
pub mod message_trait;
pub mod milky_api;
pub mod milky_message;

// ── Driver ──
pub use driver::MilkyDriver;
pub use driver::config::load_local_conf;

// ── Events ──
pub use event::{
    AdminMsgEvent, FriendMsgEvent, GroupMsgEvent, MilkyEvent, MsgEvent, MsgSendFromKoviEvent,
};
pub use event_registrar::EventRegistrar;
pub use milky_message::{MilkyMessage, Segment};

// ── Message builder ──
pub use message_trait::MessageRegistrar;

// ── API traits ──
pub use milky_api::*;
