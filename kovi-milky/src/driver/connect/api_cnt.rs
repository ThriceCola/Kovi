use crate::driver::MilkyDriver;
use crate::driver::config::Server;
use kovi::ApiReturn;
use kovi::bot::SendApi;
use kovi::driver::AnyError;
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MilkyApiReturn {
    pub status: String,
    pub retcode: i32,
    pub message: Option<String>,
    pub data: Value,
}

impl From<MilkyApiReturn> for ApiReturn {
    fn from(value: MilkyApiReturn) -> Self {
        Self {
            status: value.status,
            retcode: value.retcode,
            data: value.data,
            message: value.message,
        }
    }
}

impl MilkyDriver {
    /// api_handler 热路径：直接接收已就绪的 Sender，不再持有 server / tasks
    pub(crate) async fn send_api_inner(
        send_api: SendApi,
        client: reqwest::Client,
        server: Arc<Server>,
    ) -> Result<Result<ApiReturn, ApiReturn>, AnyError> {
        let url = server.api_url(&send_api.action);

        let response = client
            .post(&url)
            .json(&send_api.params)
            .send()
            .await
            .map_err(|e| {
                format!(
                    "API request failed [{}]: cannot connect to server (url: {}), error: {}",
                    send_api.action, url, e
                )
            })?;

        let status = response.status();

        let body_bytes = response.bytes().await.map_err(|e| {
            format!(
                "API response read failed [{}]: cannot read response body (url: {}, status: {}), error: {}",
                send_api.action, url, status, e
            )
        })?;

        let body_str = String::from_utf8_lossy(&body_bytes);

        let res: MilkyApiReturn = match serde_json::from_slice(&body_bytes) {
            Ok(v) => v,
            Err(decode_err) => {
                let action = &send_api.action;
                let preview = if body_str.len() > 500 {
                    format!(
                        "{}... (truncated, total {} bytes)",
                        &body_str[..500],
                        body_bytes.len()
                    )
                } else {
                    body_str.to_string()
                };
                error!(
                    "API response parse failed [{}]: HTTP {}, body preview:\n{}",
                    action, status, preview
                );
                error!(
                    "API response parse failed [{}]: serde error: {}",
                    action, decode_err
                );
                return Err(format!(
                    "API response JSON parse failed [{}]: HTTP {}, url: {}, serde: {}",
                    action, status, url, decode_err
                )
                .into());
            }
        };

        let value = if res.status == "ok" {
            Ok(res)
        } else {
            Err(res)
        };

        Ok(match value {
            Ok(v) => Ok(ApiReturn::from(v)),
            Err(v) => Err(ApiReturn::from(v)),
        })
    }
}
