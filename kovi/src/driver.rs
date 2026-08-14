use crate::ApiReturn;
use crate::bot::SendApi;
use crate::event::MessageEventTrait;
use crate::types::ArcTypeDeMsgEventFn;
use futures_util::Stream;
use serde_json::Value;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub enum DriverEvent {
    /// Drive 的退出事件
    Exit,
    /// 正常的运行时事件
    Normal(Value),
}

pub type AnyError = Box<dyn std::error::Error + Send + Sync>;

pub type ApiHandlerResult =
    Pin<Box<dyn Future<Output = Result<Result<ApiReturn, ApiReturn>, AnyError>> + Send>>;

/// 事件通道断开后的自动重连参数
#[derive(Debug, Clone, Copy)]
pub struct ReconnectConfig {
    /// 首次重连延迟
    pub base_delay: Duration,
    /// 重连延迟上限
    pub max_delay: Duration,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

#[async_trait::async_trait]
pub trait Driver: Send + Sync {
    async fn event_channel(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<DriverEvent, AnyError>> + Send>>, AnyError>;

    fn api_handler(&self, value: SendApi) -> ApiHandlerResult;

    fn message_event_register(&self) -> MessageEventRegister;

    /// 事件通道断开后的重连参数，默认 1s 起步、指数增长、30s 封顶
    fn reconnect_config(&self) -> ReconnectConfig {
        ReconnectConfig::default()
    }
}

pub struct MessageEventRegister {
    pub(crate) type_de: ArcTypeDeMsgEventFn,
}
impl MessageEventRegister {
    pub fn register<T: MessageEventTrait + Send + Sync>() -> Self {
        MessageEventRegister {
            type_de: Arc::new(|value, bot_info, sender| {
                Some(Arc::new(T::de(value, bot_info, sender)?))
            }),
        }
    }
}
