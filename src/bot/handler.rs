use crate::Bot;
#[cfg(feature = "plugin-access-control")]
use crate::bot::AccessControlMode;
use crate::bot::BotInformation;
#[cfg(feature = "plugin-access-control")]
use crate::bot::runtimebot::kovi_api::AccessList;
#[cfg(feature = "plugin-access-control")]
use crate::event::MessageEventTrait;
use crate::event::{Event, InternalEvent};
use crate::plugin::PLUGIN_NAME;
use crate::plugin::plugin_builder::ListenInner;
use crate::types::ApiAndOptOneshot;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};

/// Kovi内部事件
#[derive(Clone)]
pub(crate) enum InternalInternalEvent {
    Exit(ExitEvent),
    OneBotEvent(Box<InternalEvent>),
}

#[derive(Clone)]
pub(crate) enum ExitEvent {
    FromDrive,
    FromSignal,
}

impl Bot {
    pub(crate) async fn handler_event(
        bot: Arc<RwLock<Self>>,
        event: InternalInternalEvent,
        api_tx: mpsc::Sender<ApiAndOptOneshot>,
    ) {
        match event {
            InternalInternalEvent::Exit(_) => Self::handle_kovi_exit(bot).await,
            InternalInternalEvent::OneBotEvent(msg) => {
                Self::handler_internal_event(bot, *msg, api_tx).await
            }
        }
    }

    pub(crate) async fn handle_kovi_exit(bot: Arc<RwLock<Self>>) {
        let drop_task = {
            let mut bot_write = bot.write();

            #[cfg(any(feature = "save_plugin_status", feature = "save_bot_admin"))]
            bot_write.save_bot_status();
            let mut task_vec = Vec::new();
            for plugin in bot_write.plugins.values_mut() {
                task_vec.push(plugin.shutdown());
            }
            Some(task_vec)
        };
        if let Some(drop_task) = drop_task {
            for task in drop_task {
                let _ = task.await;
            }
        }
    }

    async fn handler_internal_event(
        bot: Arc<RwLock<Self>>,
        msg: InternalEvent,
        api_tx: mpsc::Sender<ApiAndOptOneshot>,
    ) {
        let bot_read = bot.read();
        let drive = bot_read.drive.clone();
        let info = &bot_read.information;

        let plugin_iter = bot_read.plugins.iter();

        let plugin_cache = plugin_iter
            .clone()
            .map(|(name, plugin)| {
                let name = Arc::new(name.to_owned());
                (
                    name.clone(),
                    PluginCache {
                        name,
                        #[cfg(feature = "plugin-access-control")]
                        acc: AccCache::new(
                            plugin.access_control,
                            plugin.list_mode,
                            plugin.access_list.clone(),
                        ),
                        bot_info: info.clone(),
                        enabled: plugin.enabled.subscribe(),
                    },
                )
            })
            .collect::<ahash::HashMap<Arc<String>, PluginCache>>();

        let type_plugin_map = {
            let mut type_plugin_map: PluginMap = Default::default();
            for (name, plugin) in plugin_iter {
                for listen in &plugin.listen.list {
                    let plugin_map = type_plugin_map.entry(listen.type_id).or_default();

                    let plugin_vec = plugin_map
                        .plugins
                        .entry(plugin_cache[name].name.clone())
                        .or_default();

                    plugin_vec.push(listen.clone());
                }
            }
            type_plugin_map
        };

        let msg_event =
            (drive.message_event_register().type_de)(&msg, &info.read(), &api_tx).map(|e| {
                log_msg_event(&*e);
                e
            });

        drop(bot_read);

        struct SharedData {
            msg: InternalEvent,
            api_tx: mpsc::Sender<ApiAndOptOneshot>,
            plugin_cache: ahash::HashMap<Arc<String>, PluginCache>,
        }

        let shared_data = Arc::new(SharedData {
            msg,
            api_tx,
            plugin_cache,
        });

        for plugin_map in type_plugin_map.into_values() {
            tokio::spawn(type_handler(
                plugin_map,
                msg_event.clone(),
                shared_data.clone(),
            ));
        }

        async fn type_handler(
            plugin_map: EventHandler,
            msg_event: Option<Arc<dyn MessageEventTrait>>,
            shared_data: Arc<SharedData>,
        ) {
            let mut event_cache: Option<Arc<dyn Event>> = None;
            for (name, plugin_vec) in plugin_map.plugins.into_iter() {
                let plugin_cache = &shared_data.plugin_cache[&name];

                // TODO： 由于onebot拆分所以这里作为一个plugin-access-control的服务暂时注释掉
                #[cfg(feature = "plugin-access-control")]
                if let Some(event) = &msg_event {
                    // 判断是否黑白名单
                    if !is_access(&plugin_cache.acc, &**event) {
                        continue;
                    }
                }

                for listen in plugin_vec {
                    let event = match &event_cache {
                        Some(v) => v.clone(),
                        None => {
                            let event_opt = (listen.type_de)(
                                &shared_data.msg,
                                &plugin_cache.bot_info.read(),
                                &shared_data.api_tx,
                            );

                            match event_opt {
                                Some(event) => {
                                    event_cache = Some(event.clone());
                                    event
                                }
                                None => return,
                            }
                        }
                    };

                    let name = name.clone();
                    let enabled = plugin_cache.enabled.clone();

                    tokio::spawn(async move {
                        tokio::select! {
                            _ = PLUGIN_NAME.scope(name, handle_listen(listen, event)) => {}
                            _ = monitor_enabled_state(enabled) => {}
                        }
                    });
                }
            }
        }

        async fn monitor_enabled_state(mut enabled: watch::Receiver<bool>) {
            loop {
                enabled
                    .changed()
                    .await
                    .expect("The enabled signal was dropped");
                if !*enabled.borrow_and_update() {
                    break;
                }
            }
        }

        // TODO： 由于onebot拆分所以这里作为一个打印bot启动日志的服务暂时注释掉
        fn log_msg_event<T: MessageEventTrait + ?Sized>(event: &T) {
            let message_type = event.get_message_type_str().unwrap_or_default();
            let group_id = match event.get_ref_group_id() {
                Some(id) => id.to_string(),
                None => "".to_string(),
            };
            let nickname = event.get_sender_name().unwrap_or_default();
            let id = event.get_sender_id();
            let msg = event.get_message();

            log::info!(
                "[{message_type}{group_id}{nickname} {id}]: {text}",
                text = msg.to_human_string()
            );
        }

        async fn handle_listen(listen: Arc<ListenInner>, cache_event: Arc<dyn Event + 'static>) {
            (*listen.handler)(cache_event).await;
        }
    }
}

struct PluginCache {
    name: Arc<String>,
    #[cfg(feature = "plugin-access-control")]
    acc: AccCache,
    bot_info: Arc<RwLock<BotInformation>>,
    enabled: watch::Receiver<bool>,
}

#[cfg(feature = "plugin-access-control")]
struct AccCache {
    pub(crate) access_control: bool,
    pub(crate) list_mode: AccessControlMode,
    pub(crate) access_list: AccessList,
}

#[cfg(feature = "plugin-access-control")]
impl AccCache {
    pub fn new(
        access_control: bool,
        list_mode: AccessControlMode,
        access_list: AccessList,
    ) -> Self {
        Self {
            access_control,
            list_mode,
            access_list,
        }
    }
}

//TODO： 由于onebot拆分所以这里作为一个plugin-access-control的服务暂时注释掉
#[cfg(feature = "plugin-access-control")]
fn is_access<T: MessageEventTrait + ?Sized>(plugin: &AccCache, event: &T) -> bool {
    if !plugin.access_control {
        return true;
    }

    let access_list = &plugin.access_list;
    let in_group = event.is_group_message();

    match (plugin.list_mode, in_group) {
        (AccessControlMode::WhiteList, true) => {
            let id = event.get_ref_group_id().expect("unreachable");
            access_list.groups.iter().any(|v| *v == id)
        }

        (AccessControlMode::WhiteList, false) => {
            let id = event.get_sender_id();
            access_list.friends.iter().any(|v| *v == id)
        }
        (AccessControlMode::BlackList, true) => {
            let id = event.get_ref_group_id().expect("unreachable");
            !access_list.groups.iter().any(|v| *v == id)
        }
        (AccessControlMode::BlackList, false) => {
            let id = event.get_sender_id();
            !access_list.friends.iter().any(|v| *v == id)
        }
    }
}

#[allow(dead_code)]
#[derive(Default)]
struct EventHandler {
    plugins: ahash::HashMap<Arc<String>, Vec<Arc<ListenInner>>>,
}

#[allow(warnings)]
type PluginMap<'a> = std::collections::HashMap<
    std::any::TypeId,
    EventHandler,
    std::hash::BuildHasherDefault<IdHasher>,
>;

#[allow(warnings)]
type TypeEventCacheMap = std::collections::HashMap<
    std::any::TypeId,
    Option<Arc<dyn Event>>,
    std::hash::BuildHasherDefault<IdHasher>,
>;

/// With TypeIds as keys, there's no need to hash them. They are already hashes
/// themselves, coming from the compiler. The IdHasher holds the u64 of
/// the TypeId, and then returns it, instead of doing any bit fiddling.
#[derive(Default, Debug)]
struct IdHasher(u64);

impl std::hash::Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}
