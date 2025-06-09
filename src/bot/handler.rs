use crate::{
    bot::{
        plugin_builder::{ListenInner, event::Event},
        *,
    },
    plugin::PLUGIN_NAME,
    types::{ApiAndOneshot, NoArgsFn},
};
use log::{debug, error, info, warn};
use parking_lot::RwLock;
use plugin_builder::event::{MsgEvent, NoticeEvent, RequestEvent};
use serde_json::{Value, json};
use std::{rc::Rc, sync::Arc};
use tokio::sync::oneshot;

/// Kovi内部事件
pub enum InternalEvent {
    KoviEvent(KoviEvent),
    OneBotEvent(String),
}

pub enum KoviEvent {
    Drop,
}

impl Bot {
    pub(crate) async fn handler_event(
        bot: Arc<RwLock<Self>>,
        event: InternalEvent,
        api_tx: mpsc::Sender<ApiAndOneshot>,
    ) {
        match event {
            InternalEvent::KoviEvent(event) => Self::handle_kovi_event(bot, event).await,
            InternalEvent::OneBotEvent(msg) => Self::handler_msg(bot, msg, api_tx).await,
        }
    }

    pub(crate) async fn handle_kovi_event(bot: Arc<RwLock<Self>>, event: KoviEvent) {
        let drop_task = {
            let mut bot_write = bot.write();
            match event {
                KoviEvent::Drop => {
                    #[cfg(any(feature = "save_plugin_status", feature = "save_bot_admin"))]
                    bot_write.save_bot_status();
                    let mut task_vec = Vec::new();
                    for plugin in bot_write.plugins.values_mut() {
                        task_vec.push(plugin.shutdown());
                    }
                    Some(task_vec)
                }
            }
        };
        if let Some(drop_task) = drop_task {
            for task in drop_task {
                let _ = task.await;
            }
        }
    }

    async fn handler_msg(bot: Arc<RwLock<Self>>, msg: String, api_tx: mpsc::Sender<ApiAndOneshot>) {
        let msg_json: Value = match serde_json::from_str(&msg) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to parse JSON from message: {}", e);
                return;
            }
        };

        debug!("{msg_json}");

        if let Some(meta_event_type) = msg_json.get("meta_event_type") {
            match meta_event_type.as_str() {
                Some("lifecycle") => {
                    Self::handler_lifecycle(api_tx).await;
                    return;
                }
                Some("heartbeat") => {
                    return;
                }
                Some(_) | None => {
                    return;
                }
            }
        }

        let post_type = match msg_json.get("post_type") {
            Some(value) => match value.as_str() {
                Some(s) => s,
                None => {
                    error!("Invalid 'post_type' value in message JSON");
                    return;
                }
            },
            None => {
                error!("Missing 'post_type' in message JSON");
                return;
            }
        };

        //     info!("[{message_type}{group_id}{nickname} {id}]: {text}");

        let bot_read = bot.read();

        let mut cache: ahash::HashMap<std::any::TypeId, Option<Arc<dyn Event>>> =
            ahash::HashMap::default();

        let msg: Rc<str> = Rc::from(msg);
        for (name, plugin) in bot_read.plugins.iter() {
            // // 判断是否黑白名单
            // #[cfg(feature = "plugin-access-control")]
            // if !is_access(plugin, &e) {
            //     continue;
            // }

            let name_ = Arc::new(name.clone());
            let msg = msg.clone();

            for listen in &plugin.listen.list {
                let name = name_.clone();
                let api_tx = api_tx.clone();

                let cache_event = match cache.get(&listen.type_id) {
                    Some(event) => match event {
                        None => {
                            continue;
                        }
                        Some(event) => event.clone(),
                    },
                    None => {
                        let event_opt = (listen.type_de)(&msg, &*bot_read, api_tx);
                        println!("!!!: 11 type: {:?}", listen.type_id);
                        cache.insert(listen.type_id, event_opt.clone());
                        match event_opt {
                            Some(event) => event,
                            None => continue,
                        }
                    }
                };

                let listen = listen.clone();
                let enabled = plugin.enabled.subscribe();

                RT.spawn(async move {
                    tokio::select! {
                        _ = PLUGIN_NAME.scope(name, Self::handle_listen(listen, cache_event)) => {}
                        _ = monitor_enabled_state(enabled) => {}
                    }
                });
            }
        }

        async fn monitor_enabled_state(mut enabled: watch::Receiver<bool>) {
            loop {
                enabled.changed().await.unwrap();
                if !*enabled.borrow_and_update() {
                    break;
                }
            }
        }
    }

    async fn handle_listen(listen: Arc<ListenInner>, cache_event: Arc<dyn Event + 'static>) {
        (*listen.handler)(cache_event).await;
    }

    pub(crate) async fn handler_lifecycle(api_tx_: mpsc::Sender<ApiAndOneshot>) {
        let api_msg = SendApi::new("get_login_info", json!({}), "kovi");

        #[allow(clippy::type_complexity)]
        let (api_tx, api_rx): (
            oneshot::Sender<Result<ApiReturn, ApiReturn>>,
            oneshot::Receiver<Result<ApiReturn, ApiReturn>>,
        ) = oneshot::channel();

        api_tx_.send((api_msg, Some(api_tx))).await.unwrap();

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

#[cfg(feature = "plugin-access-control")]
fn is_access(plugin: &Plugin, event: &MsgEvent) -> bool {
    if !plugin.access_control {
        return true;
    }

    let access_list = &plugin.access_list;
    let in_group = event.is_group();

    match (plugin.list_mode, in_group) {
        (AccessControlMode::WhiteList, true) => access_list
            .groups
            .contains(event.group_id.as_ref().unwrap()),
        (AccessControlMode::WhiteList, false) => {
            access_list.friends.contains(&event.sender.user_id)
        }
        (AccessControlMode::BlackList, true) => !access_list
            .groups
            .contains(event.group_id.as_ref().unwrap()),
        (AccessControlMode::BlackList, false) => {
            !access_list.friends.contains(&event.sender.user_id)
        }
    }
}
