use crate::types::{ApiAndOneshot, ApiOneshotReceiver, ApiOneshotSender};

use super::{ApiReturn, Bot, Host, SendApi};
use log::error;
use parking_lot::RwLock;
use serde_json::Value;
use std::sync::Weak;
use tokio::sync::{mpsc, oneshot};

pub mod kovi_api;
pub mod onebot_api;

pub use kovi_api::SetAdmin;

/// 运行时的Bot，可以用来发送api，需要从PluginBuilder的.get_runtime_bot()获取。
/// # Examples
/// ```
/// let bot = PluginBuilder::get_runtime_bot();
/// let user_id = bot.main_admin;
///
/// bot.send_private_msg(user_id, "bot online")
/// ```
#[derive(Clone)]
pub struct RuntimeBot {
    pub host: Host,
    pub port: u16,

    pub(crate) bot: Weak<RwLock<Bot>>,
    pub(crate) plugin_name: String,
    pub api_tx: mpsc::Sender<ApiAndOneshot>,
}

/// 提供给拓展API插件开发者的异步 API 请求发送函数，返回一个 Future ，用于等待在 Kovi 中已经缓存好的API响应。
pub fn send_api_request_with_response(
    api_tx: &mpsc::Sender<ApiAndOneshot>,
    send_api: SendApi,
) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
    let api_rx = send_api_request(api_tx, send_api);
    send_api_await_response(api_rx)
}

/// 提供给拓展 API 插件开发者的 API 请求发送函数，返回一个 API 通道，可以用于等待 API 响应。
pub fn send_api_request(
    api_tx: &mpsc::Sender<ApiAndOneshot>,
    send_api: SendApi,
) -> ApiOneshotReceiver {
    let (api_tx_, api_rx): (ApiOneshotSender, ApiOneshotReceiver) = oneshot::channel();

    if let Err(e) = api_tx.try_send((send_api, Some(api_tx_))) {
        match e {
            mpsc::error::TrySendError::Full(v) => {
                log::trace!("RuntimeBot Api Queue Full, spawn new task to send");

                let api_tx = api_tx.clone();

                tokio::task::spawn(async move {
                    if let Err(e) = api_tx.send(v).await {
                        error!("The mpsc sender failed to send API request: {}", e);
                    }
                });
            }
            mpsc::error::TrySendError::Closed(_) => {
                log::error!("RuntimeBot Api Queue Closed");
            }
        }
    };

    api_rx
}

/// 提供给拓展 API 插件开发者的 API 请求发送函数，忽略返回值。
pub fn send_api_request_with_forget(api_tx: &mpsc::Sender<ApiAndOneshot>, send_api: SendApi) {
    if let Err(e) = api_tx.try_send((send_api, None)) {
        match e {
            mpsc::error::TrySendError::Full(v) => {
                log::trace!("RuntimeBot Api Queue Full, spawn new task to send");

                let api_tx = api_tx.clone();

                tokio::task::spawn(async move {
                    if let Err(e) = api_tx.send(v).await {
                        error!("The mpsc sender failed to send API request: {}", e);
                    }
                });
            }
            mpsc::error::TrySendError::Closed(_) => {
                log::error!("RuntimeBot Api Queue Closed");
            }
        }
    };
}

/// 一个异步 Future ，传入一个 API 通道，可以用于等待在 Kovi 中缓存好的 API 响应。
pub async fn send_api_await_response(api_rx: ApiOneshotReceiver) -> Result<ApiReturn, ApiReturn> {
    match api_rx.await {
        Ok(v) => v,
        Err(e) => {
            error!("{e}");
            panic!()
        }
    }
}

/// 实现这个 trait, 让用户方便地发送 API 的方法。
pub trait CanSendApi {
    fn __get_api_tx(&self) -> &mpsc::Sender<ApiAndOneshot>;

    /// 发送拓展 Api, 此方法不关注返回值，返回值将丢弃。
    ///
    /// 如需要返回值，请使用 `send_api_return()`
    ///
    /// # Arguments
    ///
    /// `action`: 拓展 Api 的方法名
    ///
    /// `params`: 参数
    fn send_api(&self, action: &str, params: Value) {
        let send_api = SendApi::new(action, params);
        send_api_request_with_forget(&self.__get_api_tx(), send_api)
    }
    /// 发送拓展 Api, 此方法关注返回值。
    ///
    /// 如不需要返回值，推荐使用 `send_api()`
    ///
    /// # Arguments
    ///
    /// `action`: 拓展 Api 的方法名
    ///
    /// `params`: 参数
    fn send_api_return(
        &self,
        action: &str,
        params: Value,
    ) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new(action, params);
        send_api_request_with_response(&self.__get_api_tx(), send_api)
    }
}
