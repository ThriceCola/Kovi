//! # Kovi
//!
//! A OneBot V11 bot framework developed using Rust.
//!
//! More documentation can be found at [Github-Kovi](https://github.com/Threkork/Kovi) Or [Kovi-doc](https://threkork.github.io/kovi-doc/)
//!
//! 中文文档或更多文档请查看[Github-Kovi](https://github.com/Threkork/Kovi) 和 [Kovi-doc](https://threkork.github.io/kovi-doc/)
#![deny(clippy::unwrap_used)]

/// Everything about bots is inside
pub mod bot;
/// 一些错误枚举
pub mod error;
/// 控制台输出日志
pub mod logger;
/// 关于插件的一切
pub mod plugin;
/// task 提供 kovi 运行时的多线程处理
pub mod task;
/// 这里包含一些集成类型
pub mod types;
/// 提供一些方便的插件开发函数
pub mod utils;

pub use bot::ApiReturn;
pub use bot::Bot;
pub use bot::event;
pub use bot::message::Message;
pub use bot::runtimebot::RuntimeBot;
pub use error::MessageError;
pub use kovi_macros::plugin;
pub use plugin::plugin_builder::PluginBuilder;
pub use plugin::plugin_builder::event::MsgEvent;
pub use plugin::plugin_builder::event::NoticeEvent;
pub use plugin::plugin_builder::event::RequestEvent;
pub use task::spawn;

#[deprecated(since = "0.11.0", note = "请使用 `MsgEvent` 代替")]
pub type AllMsgEvent = bot::plugin_builder::event::MsgEvent;
#[deprecated(since = "0.11.0", note = "请使用 `NoticeEvent` 代替")]
pub type AllNoticeEvent = bot::plugin_builder::event::NoticeEvent;
#[deprecated(since = "0.11.0", note = "请使用 `RequestEvent` 代替")]
pub type AllRequestEvent = bot::plugin_builder::event::RequestEvent;

pub use chrono;
pub use croner;
pub use futures_util;
pub use log;
pub use serde_json;
pub use tokio;
pub use toml;

pub(crate) use crate::bot::run::RUNTIME as RT;
