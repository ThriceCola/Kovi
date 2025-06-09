use super::{plugin_builder::Listen, AccessControlMode, AccessList, KoviAsyncFn};
use crate::{
    bot::{PLUGIN_BUILDER, PLUGIN_NAME},
    task::TASK_MANAGER,
    Bot, PluginBuilder, RT,
};
use std::sync::Arc;
use tokio::{sync::watch, task::JoinHandle};

#[derive(Clone)]
pub struct Plugin {
    pub(crate) enable_on_startup: bool,
    pub(crate) enabled: watch::Sender<bool>,

    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) main: Arc<KoviAsyncFn>,
    pub(crate) listen: Listen,

    #[cfg(feature = "plugin-access-control")]
    pub(crate) access_control: bool,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) list_mode: AccessControlMode,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) access_list: AccessList,
}

impl Plugin {
    pub fn new(name: String, version: String, main: Arc<KoviAsyncFn>) -> Self {
        let (tx, _rx) = watch::channel(true);
        Self {
            enable_on_startup: true,
            enabled: tx,
            name,
            version,
            main,
            listen: Listen::default(),

            #[cfg(feature = "plugin-access-control")]
            access_control: false,
            #[cfg(feature = "plugin-access-control")]
            list_mode: AccessControlMode::WhiteList,
            #[cfg(feature = "plugin-access-control")]
            access_list: AccessList::default(),
        }
    }

    pub(crate) fn shutdown(&mut self) -> JoinHandle<()> {
        log::debug!("Plugin '{}' is dropping.", self.name,);

        let plugin_name_ = Arc::new(self.name.clone());

        let mut task_vec = Vec::new();

        for listen in &self.listen.drop {
            let listen_clone = listen.clone();
            let plugin_name_ = plugin_name_.clone();
            let task = RT.get().unwrap().spawn(async move {
                PLUGIN_NAME
                    .scope(plugin_name_, Bot::handler_drop(listen_clone))
                    .await;
            });
            task_vec.push(task);
        }

        TASK_MANAGER.disable_plugin(&self.name);

        self.enabled.send_modify(|v| {
            *v = false;
        });
        self.listen.clear();
        RT.get().unwrap().spawn(async move {
            for task in task_vec {
                let _ = task.await;
            }
        })
    }

    // 运行插件的main()
    pub fn run_plugin_main(&self, plugin_builder: PluginBuilder) {
        println!("Running plugin main");
        let plugin_name = plugin_builder.runtime_bot.plugin_name.clone();

        let mut enabled = self.enabled.subscribe();
        let main = self.main.clone();

        RT.get().unwrap().spawn(async move {
            tokio::select! {
                _ = PLUGIN_NAME.scope(
                        Arc::new(plugin_name),
                        PLUGIN_BUILDER.scope(plugin_builder, main()),
                ) => {}
                _ = async {
                        loop {
                            enabled.changed().await.unwrap();
                            if !*enabled.borrow_and_update() {
                                break;
                            }
                        }
                } => {}
            }
        });
    }
}
