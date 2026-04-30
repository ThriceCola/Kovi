use crate::ApiReturn;
use crate::bot::SendApi;
use crate::event::MessageEventTrait;
use crate::types::ArcTypeDeFn;
use futures_util::Stream;
use serde_json::Value;
use std::fmt::Display;
use std::pin::Pin;
use std::sync::Arc;

pub enum DriveEvent {
    /// Drive 的退出事件
    Exit,
    /// 正常的运行时事件
    Normal(Value),
}

pub type AnyError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait::async_trait]
pub trait Drive: Send + Sync {
    async fn event_channel(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<DriveEvent, AnyError>> + Send>>, AnyError>;

    fn api_handler(
        &self,
        value: SendApi,
    ) -> Pin<Box<dyn Future<Output = Result<Result<ApiReturn, ApiReturn>, AnyError>> + Send>>;

    fn message_event_register(&self) -> MessageEventRegister;
}

pub struct MessageEventRegister {
    pub(crate) type_id: std::any::TypeId,
    pub(crate) type_de: ArcTypeDeFn,
}
impl MessageEventRegister {
    pub fn register<T: crate::event::Event + MessageEventTrait + Send + Sync>() -> Self {
        MessageEventRegister {
            type_id: std::any::TypeId::of::<T>(),
            type_de: Arc::new(|value, bot_info, sender| {
                Some(Arc::new(T::de(value, bot_info, sender)?))
            }),
        }
    }
}
