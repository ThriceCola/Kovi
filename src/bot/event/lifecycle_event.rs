use crate::{
    ApiReturn,
    bot::{
        BotInformation, SendApi,
        event::InternalEvent,
        plugin_builder::event::{Event, PostType},
    },
    types::ApiAndOneshot,
};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone, Deserialize)]
pub struct LifecycleEvent {
    pub meta_event_type: String,
    pub post_type: PostType,
    pub self_id: i64,
    pub time: i64,
    pub sub_type: LifecycleAction,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LifecycleAction {
    Enable,
    Disable,
    Connect,
}

impl Event for LifecycleEvent {
    fn de(
        event: &InternalEvent,
        _: &BotInformation,
        _: &tokio::sync::mpsc::Sender<ApiAndOneshot>,
    ) -> Option<Self>
    where
        Self: Sized,
    {
        let InternalEvent::OneBotEvent(json_str) = event else {
            return None;
        };
        let event: LifecycleEvent = serde_json::from_str(json_str).ok()?;
        if event.meta_event_type == "lifecycle" {
            Some(event)
        } else {
            None
        }
    }
}

impl LifecycleEvent {
    pub(crate) async fn handler_lifecycle(api_tx_: mpsc::Sender<ApiAndOneshot>) {
        let api_msg = SendApi::new("get_login_info", json!({}));

        #[allow(clippy::type_complexity)]
        let (api_tx, api_rx): (
            oneshot::Sender<Result<ApiReturn, ApiReturn>>,
            oneshot::Receiver<Result<ApiReturn, ApiReturn>>,
        ) = oneshot::channel();

        api_tx_
            .send((api_msg, Some(api_tx)))
            .await
            .expect("The api_tx channel closed");

        let receive = match api_rx.await {
            Ok(v) => v,
            Err(e) => {
                error!("Lifecycle Error, get bot info failed: {}", e);
                return;
            }
        };

        let self_info_value = match receive {
            Ok(v) => v,
            Err(e) => {
                error!("Lifecycle Error, get bot info failed: {}", e);
                return;
            }
        };

        let self_id = match self_info_value.data.get("user_id") {
            Some(user_id) => match user_id.as_i64() {
                Some(id) => id,
                None => {
                    error!("Expected 'user_id' to be an integer");
                    return;
                }
            },
            None => {
                error!("Missing 'user_id' in self_info_value data");
                return;
            }
        };
        let self_name = match self_info_value.data.get("nickname") {
            Some(nickname) => nickname.to_string(),
            None => {
                error!("Missing 'nickname' in self_info_value data");
                return;
            }
        };
        info!(
            "Bot connection successful，Nickname:{},ID:{}",
            self_name, self_id
        );
    }
}
