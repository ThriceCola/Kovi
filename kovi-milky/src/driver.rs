pub mod config;

use crate::driver::config::{MilkyDriverConfig, Server};
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use kovi::bot::SendApi;
use kovi::driver::{Driver, DriverEvent, MessageEventRegister};
use kovi::futures_util;
use log::{error, info};
use std::sync::Arc;

pub(crate) mod connect;

pub struct MilkyDriver {
    pub(crate) server: Arc<Server>,
    pub(crate) req_client: reqwest::Client,
}

impl MilkyDriver {
    pub fn new(config: MilkyDriverConfig) -> Self {
        use reqwest::header;
        let mut headers = header::HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        if !config.server.access_token.is_empty() {
            let auth = format!("Bearer {}", config.server.access_token);
            headers.insert(AUTHORIZATION, header::HeaderValue::from_str(&auth).unwrap());
        }
        Self {
            server: Arc::new(config.server),
            req_client: reqwest::Client::builder()
                .default_headers(headers)
                .build()
                .expect("failed to create reqwest client"),
        }
    }
}

#[async_trait::async_trait]
impl Driver for MilkyDriver {
    async fn event_channel(
        &self,
    ) -> Result<
        std::pin::Pin<
            Box<
                dyn futures_util::Stream<Item = Result<DriverEvent, kovi::driver::AnyError>> + Send,
            >,
        >,
        kovi::driver::AnyError,
    > {
        match self.handler_lifecycle_log_bot_enable().await {
            Ok(_) => {}
            Err(_) => {
                log::error!("Failed to initialize onebot connection");
                return Err("Failed to initialize onebot connection".into());
            }
        };

        MilkyDriver::ws_event_connect((*self.server).clone()).await
    }

    fn api_handler(
        &self,
        value: kovi::bot::SendApi,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        Result<kovi::ApiReturn, kovi::ApiReturn>,
                        kovi::driver::AnyError,
                    >,
                > + Send,
        >,
    > {
        let client = self.req_client.clone();
        let server = self.server.clone();
        Box::pin(async move { MilkyDriver::send_api_inner(value, client, server).await })
    }

    fn message_event_register(&self) -> MessageEventRegister {
        // MessageEventRegister::register::<MsgEvent>()
        todo!()
    }
}

impl MilkyDriver {
    pub(crate) async fn handler_lifecycle_log_bot_enable(&self) -> Result<(), ()> {
        let api_msg = SendApi::new("get_login_info", serde_json::json!({}));

        let res = match self.api_handler(api_msg).await {
            Ok(v) => v,
            Err(err) => {
                let server_url = self.server.api_url("get_login_info");
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

        let self_id = match self_info_value.data.get("uin") {
            Some(user_id) => match user_id.as_i64() {
                Some(id) => id,
                None => {
                    error!("Expected 'uin' to be an integer");
                    return Err(());
                }
            },
            None => {
                error!("Missing 'uin' in self_info_value data");
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
