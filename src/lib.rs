//! # Kovi
//!
//! A OneBot V11 bot framework developed using Rust.
//!
//! More documentation can be found at [Github-Kovi](https://github.com/ThriceCola/Kovi) Or [Kovi-doc](https://thricecola.github.io/kovi-doc/)
//!
//! 中文文档或更多文档请查看[Github-Kovi](https://github.com/ThriceCola/Kovi) 和 [Kovi-doc](https://thricecola.github.io/kovi-doc/)
#![deny(clippy::unwrap_used)]

/// Everything about bots is inside
pub mod bot;
pub mod config;
/// 连接服务端的驱动
pub mod drive;
/// 一些错误枚举
pub mod error;
pub mod event;
/// 控制台输出日志
pub mod logger;
pub mod message;
/// 关于插件的一切
pub mod plugin;
/// task 提供 kovi 运行时的多线程处理
pub mod task;
/// 这里包含一些集成类型
pub mod types;
/// 提供一些方便的插件开发函数
pub mod utils;

pub use bot::runtimebot::RuntimeBot;
pub use bot::{ApiReturn, Bot};
pub use config::kovi_conf::load_local_conf;
pub use kovi_macros::plugin;
pub use plugin::plugin_builder::PluginBuilder;
pub use task::spawn;

pub use chrono;
pub use croner;
pub use futures_util;
pub use log;
pub use serde_json;
pub use tokio;
pub use toml;
