use crate::{
    bot::{
        plugin_builder::{
            ListenInner,
            event::{Event, lifecycle_event::LifecycleEvent},
        },
        *,
    },
    event::InternalEvent,
    plugin::PLUGIN_NAME,
    types::ApiAndOneshot,
};
use log::info;
use parking_lot::RwLock;
use plugin_builder::event::MsgEvent;
use std::sync::Arc;

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
        // debug!("{msg_json}");

        let bot_read = bot.read();

        let mut cache: TypeEventCacheMap = Default::default();

        if let Some(lifecycle_event) = LifecycleEvent::de(&msg, &bot_read.information, &api_tx) {
            tokio::spawn(LifecycleEvent::handler_lifecycle(api_tx.clone()));
            cache.insert(
                std::any::TypeId::of::<LifecycleEvent>(),
                Some(Arc::new(lifecycle_event)),
            );
        };

        let msg_event = MsgEvent::de(&msg, &bot_read.information, &api_tx);

        // 这里在 没有 plugin-access-control 会警告所以用 _
        let _msg_sevent_opt = match msg_event {
            Some(event) => {
                let event = Arc::new(event);
                log_msg_event(&event);
                cache.insert(std::any::TypeId::of::<MsgEvent>(), Some(event.clone()));
                Some(event)
            }
            None => None,
        };

        for (name, plugin) in bot_read.plugins.iter() {
            #[cfg(feature = "plugin-access-control")]
            if let Some(event) = &_msg_sevent_opt {
                // 判断是否黑白名单
                if !is_access(plugin, event) {
                    continue;
                }
            }

            let name_ = Arc::new(name.clone());

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
                        let event_opt = (listen.type_de)(&msg, &bot_read.information, &api_tx);
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
    }

    async fn handle_listen(listen: Arc<ListenInner>, cache_event: Arc<dyn Event + 'static>) {
        (*listen.handler)(cache_event).await;
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
struct EventHandler<'a> {
    plugin: ahash::HashMap<&'a str, Arc<dyn Event>>,
}

#[allow(warnings)]
type PluginMap<'a> =
    HashMap<std::any::TypeId, EventHandler<'a>, std::hash::BuildHasherDefault<IdHasher>>;

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
