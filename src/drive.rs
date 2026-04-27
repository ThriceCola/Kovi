use crate::ApiReturn;
use crate::bot::SendApi;
use crate::config::runtime_config::RuntimeConfig;
use futures_util::Stream;
use serde_json::Value;
use std::pin::Pin;

pub enum DriveEvent {
    /// Drive 的退出事件
    Exit,
    /// 正常的运行时事件
    Normal(Value),
}

pub trait Drive: Send + Sync {
    fn event_channel(&self) -> Pin<Box<dyn Stream<Item = DriveEvent> + Send>>;

    fn api_handler(
        &self,
        value: SendApi,
    ) -> Pin<Box<dyn Future<Output = Result<ApiReturn, ApiReturn>> + Send>>;
}
