//! Bot 驱动器抽象层

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use tokio::sync::mpsc;

use crate::bot::SendApi;

/// 驱动器错误类型
#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    #[error("连接失败: {0}")]
    ConnectionFailed(#[source] Box<dyn Error + Send + Sync>),

    #[error("发送消息失败: {0}")]
    SendFailed(#[source] Box<dyn Error + Send + Sync>),

    #[error("接收消息失败: {0}")]
    ReceiveFailed(#[source] Box<dyn Error + Send + Sync>),

    #[error("驱动器未初始化")]
    NotInitialized,

    #[error("驱动器已关闭")]
    Closed,

    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("序列化错误: {0}")]
    SerializationError(String),

    #[error("反序列化错误: {0}")]
    DeserializationError(String),
}

/// 驱动器连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitStatus {
    Ready,
    Retry,
    Failure,
}

/// Bot 驱动器 trait
///
/// 所有的通信驱动器都需要实现这个 trait，专注于消息的输入和输出
pub trait BotDriver: Send + Sync {
    /// 接收来自外部服务的消息
    fn recv_event(&self) -> impl Future<Output = Result<Value, DriverError>> + Send;

    /// 发送消息到外部服务
    fn send_event(&self, value: SendApi)
    -> impl Future<Output = Result<Value, DriverError>> + Send;

    /// 启动驱动器
    fn initialize(&self) -> impl Future<Output = InitStatus> + Send;

    /// 等待驱动器退出
    ///
    /// 这个方法会在启动驱动器后会马上执行，等待驱动器退出事件.
    fn wait_until_exit(&self) -> impl Future<Output = ()> + Send;
}

// /// 驱动器管理器
// ///
// /// 用于管理驱动器的生命周期和消息路由
// pub struct DriverManager {
//     driver: Box<dyn BotDriver>,
//     input_tx: mpsc::Sender<MessageType>,
//     output_rx: mpsc::Receiver<MessageType>,
//     status: DriverStatus,
// }

// impl DriverManager {
//     /// 创建新的驱动器管理器
//     pub fn new(
//         driver: Box<dyn BotDriver>,
//         buffer_size: usize,
//     ) -> (Self, mpsc::Receiver<MessageType>, mpsc::Sender<MessageType>) {
//         let (input_tx, input_rx) = mpsc::channel(buffer_size);
//         let (output_tx, output_rx) = mpsc::channel(buffer_size);

//         let manager = Self {
//             driver,
//             input_tx: input_tx.clone(),
//             output_rx,
//             status: DriverStatus::Disconnected,
//         };

//         (manager, input_rx, output_tx)
//     }

//     /// 启动驱动器消息循环
//     pub async fn start(&mut self) -> Result<(), DriverError> {
//         self.status = DriverStatus::Connecting;

//         // 启动发送循环
//         let mut driver_clone =
//             Box::from_raw(Box::into_raw(self.driver.as_mut()) as *mut dyn BotDriver);
//         tokio::spawn(async move {
//             // 发送循环逻辑
//             // 这里可以根据需要实现具体的消息发送逻辑
//         });

//         // 启动接收循环
//         tokio::spawn(async move {
//             // 接收循环逻辑
//             // 这里可以根据需要实现具体的消息接收逻辑
//         });

//         self.status = DriverStatus::Connected;
//         Ok(())
//     }

//     /// 停止驱动器
//     pub async fn stop(&mut self) -> Result<(), DriverError> {
//         self.status = DriverStatus::Closing;
//         // 实现停止逻辑
//         self.status = DriverStatus::Closed;
//         Ok(())
//     }

//     /// 获取输入通道发送端
//     pub fn input_sender(&self) -> mpsc::Sender<MessageType> {
//         self.input_tx.clone()
//     }

//     /// 获取当前状态
//     pub fn status(&self) -> DriverStatus {
//         self.status
//     }
// }

// /// 辅助函数：将 API 调用转换为消息
// pub fn api_call_to_message(id: String, method: String, params: serde_json::Value) -> MessageType {
//     MessageType::ApiCall { id, method, params }
// }

// /// 辅助函数：将事件数据转换为消息
// pub fn event_to_message(event_type: String, data: serde_json::Value) -> MessageType {
//     MessageType::Event { event_type, data }
// }

// /// 辅助函数：创建控制消息
// pub fn control_message(control_type: ControlType, data: Option<serde_json::Value>) -> MessageType {
//     MessageType::Control { control_type, data }
// }
