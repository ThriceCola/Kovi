use crate::ApiReturn;
use crate::bot::SendApi;
use futures_util::Stream;
use serde_json::Value;
use std::pin::Pin;

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
}
