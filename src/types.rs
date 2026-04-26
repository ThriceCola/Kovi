use crate::ApiReturn;
use crate::bot::SendApi;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::oneshot;

pub(crate) type KoviAsyncFn = dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;

pub type PinFut = Pin<Box<dyn Future<Output = ()> + Send>>;

pub type NoArgsFn = Arc<dyn Fn() -> PinFut + Send + Sync>;

pub type ApiOneshotSender = oneshot::Sender<Result<ApiReturn, ApiReturn>>;
pub type ApiOneshotReceiver = oneshot::Receiver<Result<ApiReturn, ApiReturn>>;

pub type ApiAndOptOneshot = (SendApi, Option<ApiOneshotSender>);

pub type ApiAndRuturn = (SendApi, Result<ApiReturn, ApiReturn>);
