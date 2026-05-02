use std::sync::Arc;

use crate::driver::config::{OneBotDriverConfig, Server};
use crate::driver::connect::api_cnt::{OneBotApiOneshotSender, OneBotSendApi};
use crate::event::MsgEvent;
use kovi::bot::SendApi;
use kovi::drive::{Drive, DriveEvent, MessageEventRegister};
use kovi::futures_util;
use log::{error, info};
use tokio::sync::{Mutex, OnceCell, mpsc};

pub mod config;
pub(crate) mod connect;

/// echo -> oneshot sender，用于将 WS 返回的响应路由回调用者
pub(crate) type OneshotTxMap =
    Arc<parking_lot::Mutex<ahash::HashMap<String, OneBotApiOneshotSender>>>;

/// Drop 时自动 abort 的任务句柄
pub(crate) struct AbortOnDrop(pub(crate) tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// 初始化一次后持有的上下文：写端 sender + 后台任务句柄
pub(crate) struct ApiContext {
    pub(crate) api_tx: mpsc::Sender<(OneBotSendApi, Option<OneBotApiOneshotSender>)>,
    /// 字段名以 _ 开头，只用于 Drop 时自动 abort 任务
    _tasks: Vec<AbortOnDrop>,
}

pub struct OneBotDriver {
    pub(crate) server: Arc<Server>,
    /// 异步 OnceCell：保证并发时只初始化一次
    ctx: Arc<OnceCell<ApiContext>>,
    pub(crate) event_tx:
        Arc<Mutex<Option<mpsc::Sender<Result<DriveEvent, kovi::drive::AnyError>>>>>,
}

impl OneBotDriver {
    pub fn new(config: OneBotDriverConfig) -> Self {
        Self {
            server: Arc::new(config.server),
            ctx: Arc::new(OnceCell::new()),
            event_tx: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl Drive for OneBotDriver {
    async fn event_channel(
        &self,
    ) -> Result<
        std::pin::Pin<
            Box<dyn futures_util::Stream<Item = Result<DriveEvent, kovi::drive::AnyError>> + Send>,
        >,
        kovi::drive::AnyError,
    > {
        let (event_tx, event_rx) = mpsc::channel(64);
        {
            let mut guard = self.event_tx.lock().await;
            *guard = Some(event_tx);
        }

        match self.handler_lifecycle_log_bot_enable().await {
            Ok(_) => {}
            Err(_) => {
                log::error!("Failed to initialize onebot connection");
                return Err("Failed to initialize onebot connection".into());
            }
        };

        OneBotDriver::ws_event_connect((*self.server).clone(), event_rx).await
    }

    fn api_handler(
        &self,
        value: kovi::bot::SendApi,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        Result<kovi::ApiReturn, kovi::ApiReturn>,
                        kovi::drive::AnyError,
                    >,
                > + Send,
        >,
    > {
        if self.ctx.initialized() {
            let ctx = Arc::clone(&self.ctx);
            Box::pin(async move {
                // 初始化后只用 api_tx（Sender clone 极廉价），server 不再传入热路径
                let api_tx = ctx.get().expect("unreachable").api_tx.clone();

                OneBotDriver::send_api_inner(api_tx, value).await
            })
        } else {
            let server = Arc::clone(&self.server);
            let event_tx = Arc::clone(&self.event_tx);
            let self_ctx = Arc::clone(&self.ctx);
            Box::pin(async move {
                let api_tx = self_ctx
                    .get_or_try_init(|| OneBotDriver::init_api_context(server, event_tx))
                    .await?
                    .api_tx
                    .clone();
                OneBotDriver::send_api_inner(api_tx, value).await
            })
        }
    }

    fn message_event_register(&self) -> MessageEventRegister {
        MessageEventRegister::register::<MsgEvent>()
    }
}

impl std::fmt::Display for OneBotSendApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).expect("unreachable"))
    }
}

impl OneBotDriver {
    pub(crate) async fn handler_lifecycle_log_bot_enable(&self) -> Result<(), ()> {
        let api_msg = SendApi::new("get_login_info", serde_json::json!({}));

        let res = match self.api_handler(api_msg).await {
            Ok(v) => v,
            Err(err) => {
                let server_url = self.server.ws_url("api");
                error!("failed to initialize api_handler (server url: {server_url}): {err}");
                return Err(());
            }
        };

        let self_info_value = match res {
            Ok(v) => v,
            Err(e) => {
                error!("Lifecycle Error, get bot info failed: {e}");
                return Err(());
            }
        };

        let self_id = match self_info_value.data.get("user_id") {
            Some(user_id) => match user_id.as_i64() {
                Some(id) => id,
                None => {
                    error!("Expected 'user_id' to be an integer");
                    return Err(());
                }
            },
            None => {
                error!("Missing 'user_id' in self_info_value data");
                return Err(());
            }
        };
        let self_name = match self_info_value.data.get("nickname") {
            Some(nickname) => nickname.to_string(),
            None => {
                error!("Missing 'nickname' in self_info_value data");
                return Err(());
            }
        };
        info!("Bot connection successful，Nickname:{self_name},ID:{self_id}");

        Ok(())
    }
}
