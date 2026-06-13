use crate::driver::MilkyDriver;
use crate::driver::config::Server;
use kovi::ApiReturn;
use kovi::bot::SendApi;
use kovi::driver::AnyError;
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
        let res: MilkyApiReturn = client
            .post(url)
            .json(&send_api.params)
            .send()
            .await?
            .json()
            .await?;

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
