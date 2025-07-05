use crate::{
    bot::{
        plugin_builder::{ListenInner, event::Event},
        *,
    },
    event::InternalEvent,
    plugin::PLUGIN_NAME,
    types::ApiAndOneshot,
};
use log::info;
use parking_lot::RwLock;
use plugin_builder::event::MsgEvent;
use std::{any::TypeId, sync::Arc};

/// Kovi内部事件
pub(crate) enum InternalInternalEvent {
    KoviEvent(KoviEvent),
    OneBotEvent(InternalEvent),
}

pub(crate) enum KoviEvent {
    Drop,
}

impl Bot {
    pub(crate) async fn handler_event(
        bot: Arc<RwLock<Self>>,
        event: InternalInternalEvent,
        api_tx: mpsc::Sender<ApiAndOneshot>,
    ) {
        match event {
            InternalInternalEvent::KoviEvent(event) => Self::handle_kovi_event(bot, event).await,
            InternalInternalEvent::OneBotEvent(msg) => {
                Self::handler_internal_event(bot, msg, api_tx).await
            }
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

    async fn handler_internal_event(
        bot: Arc<RwLock<Self>>,
        msg: InternalEvent,
        api_tx: mpsc::Sender<ApiAndOneshot>,
    ) {
        let bot_read = bot.read();

        let info = bot_read.information.clone();

        let plugin_iter = bot_read.plugins.iter();

        let plugin_cache = plugin_iter
            .clone()
            .map(|(name, plugin)| {
                let name = Arc::new(name.to_owned());
                (name.clone(), PluginCache {
                    name,
                    #[cfg(feature = "plugin-access-control")]
                    acc: AccCache::new(
                        plugin.access_control,
                        plugin.list_mode,
                        plugin.access_list.clone(),
                    ),
                    bot_info: info.clone(),
                    enabled: plugin.enabled.subscribe(),
                })
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

        drop(bot_read);

        let msg_event = MsgEvent::de(&msg, &info.read(), &api_tx).map(|e| {
            log_msg_event(&e);
            Arc::new(e)
        });

        struct SharedData {
            msg: InternalEvent,
            api_tx: mpsc::Sender<ApiAndOneshot>,
            plugin_cache: ahash::HashMap<Arc<String>, PluginCache>,
        }

        let shared_data = Arc::new(SharedData {
            msg,
            api_tx,
            plugin_cache,
        });

        for (type_id, plugin_map) in type_plugin_map {
            tokio::spawn(type_handler(
                type_id,
                plugin_map,
                msg_event.clone(),
                shared_data.clone(),
            ));
        }

        async fn type_handler(
            type_id: TypeId,
            plugin_map: EventHandler,
            msg_event: Option<Arc<MsgEvent>>,
            shared_data: Arc<SharedData>,
        ) {
            let mut event_cache = if type_id == TypeId::of::<MsgEvent>() {
                msg_event.clone().map(|arc| arc as Arc<dyn Event>)
            } else {
                None
            };

            for (name, plugin_vec) in plugin_map.plugins.into_iter() {
                let plugin_cache = &shared_data.plugin_cache[&name];

                #[cfg(feature = "plugin-access-control")]
                if let Some(event) = &msg_event {
                    // 判断是否黑白名单
                    if !is_access(&plugin_cache.acc, event) {
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

                    RT.spawn(async move {
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

        fn log_msg_event(event: &MsgEvent) {
            info!(
                "[{message_type}{group_id}{nickname} {id}]: {text}",
                message_type = event.message_type,
                group_id = match event.group_id {
                    Some(id) => id.to_string(),
                    None => "".to_string(),
                },
                nickname = match &event.sender.nickname {
                    Some(nickname) => nickname,
                    None => "",
                },
                id = event.sender.user_id,
                text = event.message.to_human_string()
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

#[cfg(feature = "plugin-access-control")]
fn is_access(plugin: &AccCache, event: &MsgEvent) -> bool {
    if !plugin.access_control {
        return true;
    }

    let access_list = &plugin.access_list;
    let in_group = event.is_group();

    match (plugin.list_mode, in_group) {
        (AccessControlMode::WhiteList, true) => access_list
            .groups
            .contains(event.group_id.as_ref().expect("unreachable")),
        (AccessControlMode::WhiteList, false) => {
            access_list.friends.contains(&event.sender.user_id)
        }
        (AccessControlMode::BlackList, true) => !access_list
            .groups
            .contains(event.group_id.as_ref().expect("unreachable")),
        (AccessControlMode::BlackList, false) => {
            !access_list.friends.contains(&event.sender.user_id)
        }
    }
}

#[allow(dead_code)]
#[derive(Default)]
struct EventHandler {
    plugins: ahash::HashMap<Arc<String>, Vec<Arc<ListenInner>>>,
}

#[allow(warnings)]
type PluginMap<'a> =
    HashMap<std::any::TypeId, EventHandler, std::hash::BuildHasherDefault<IdHasher>>;

#[allow(warnings)]
type TypeEventCacheMap =
    HashMap<std::any::TypeId, Option<Arc<dyn Event>>, std::hash::BuildHasherDefault<IdHasher>>;

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

// if let Some(lifecycle_event) = LifecycleEvent::de(&msg, &bot_read.information, &api_tx) {
//     tokio::spawn(lifecycle_event::handler_lifecycle_log_bot_enable(
//         api_tx.clone(),
//     ));
//     cache.insert(
//         std::any::TypeId::of::<LifecycleEvent>(),
//         Some(Arc::new(lifecycle_event)),
//     );
// };

// // 这里在 没有 plugin-access-control 会警告所以用 _
// let _msg_sevent_opt = match msg_event {
//     Some(event) => {
//         let event = Arc::new(event);
//         log_msg_event(&event);
//         cache.insert(std::any::TypeId::of::<MsgEvent>(), Some(event.clone()));
//         Some(event)
//     }
//     None => None,
// };

// for (name, plugin) in bot_read.plugins.iter() {
//     let name_ = Arc::new(name.clone());

//     for listen in &plugin.listen.list {
//         let name = name_.clone();
//         let api_tx = api_tx.clone();

//         let cache_event = match cache.get(&listen.type_id) {
//             Some(event) => match event {
//                 None => {
//                     continue;
//                 }
//                 Some(event) => event.clone(),
//             },
//             None => {
//                 let event_opt = (listen.type_de)(&msg, &bot_read.information, &api_tx);
//                 cache.insert(listen.type_id, event_opt.clone());
//                 match event_opt {
//                     Some(event) => event,
//                     None => continue,
//                 }
//             }
//         };

//         let listen = listen.clone();
//         let enabled = plugin.enabled.subscribe();

//         RT.spawn(async move {
//             tokio::select! {
//                 _ = PLUGIN_NAME.scope(name, Self::handle_listen(listen, cache_event)) => {}
//                 _ = monitor_enabled_state(enabled) => {}
//             }
//         });
//     }
// }
