pub mod plugin_builder;
pub mod plugin_set;

use crate::PluginBuilder;
use crate::bot::plugin_builder::Listen;
#[cfg(feature = "plugin-access-control")]
use crate::bot::runtimebot::kovi_api::AccessList;
use crate::types::KoviAsyncFn;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::watch;
use tokio::task::JoinHandle;

#[cfg(feature = "plugin-access-control")]
pub use crate::bot::runtimebot::kovi_api::AccessControlMode;

use crate::task::TASK_MANAGER;

tokio::task_local! {
    pub static PLUGIN_BUILDER: crate::PluginBuilder;
}

tokio::task_local! {
    pub(crate) static PLUGIN_NAME: Arc<String>;
}

#[derive(Clone)]
pub struct Plugin {
    pub(crate) enable_on_startup: bool,
    pub(crate) enabled: watch::Sender<bool>,

    pub name: String,
    pub version: String,
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
    pub fn new<S>(name: S, version: S, main: Arc<KoviAsyncFn>) -> Self
    where
        S: Into<String>,
    {
        Self {
            enable_on_startup: true,
            enabled: watch::channel(true).0,
            name: name.into(),
            version: version.into(),
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

    /// 运行单个插件的main()
    pub(crate) fn run(&self, plugin_builder: PluginBuilder) {
        let plugin_name = plugin_builder.runtime_bot.plugin_name.clone();

        let mut enabled = self.enabled.subscribe();
        let main = self.main.clone();

        tokio::spawn(async move {
            tokio::select! {
                _ = PLUGIN_NAME.scope(
                        Arc::new(plugin_name),
                        PLUGIN_BUILDER.scope(plugin_builder, main()),
                ) =>{}
                _ = async {
                        loop {
                            enabled.changed().await.expect("Failed to change enabled status");
                            if !*enabled.borrow_and_update() {
                                break;
                            }
                        }
                } => {}
            }
        });
    }

    pub(crate) fn shutdown(&mut self) -> JoinHandle<()> {
        log::debug!("Plugin '{}' is dropping.", self.name,);

        let plugin_name_ = Arc::new(self.name.clone());

        let mut task_vec = Vec::new();

        for listen in &self.listen.drop {
            let listen_clone = listen.clone();
            let plugin_name_ = plugin_name_.clone();
            let task = tokio::spawn(async move {
                PLUGIN_NAME.scope(plugin_name_, listen_clone()).await;
            });
            task_vec.push(task);
        }

        TASK_MANAGER.disable_plugin(&self.name);

        self.enabled.send_modify(|v| {
            *v = false;
        });
        self.listen.clear();
        tokio::spawn(async move {
            for task in task_vec {
                let _ = task.await;
            }
        })
    }
}

/// 黑白名单
#[cfg(feature = "plugin-access-control")]
impl Plugin {
    /// 启动名单
    pub fn set_access_control(&mut self, enable: bool) {
        self.access_control = enable;
    }

    /// 更改名单为其他模式，插件默认为白名单模式
    pub fn set_access_control_mode(&mut self, access_control_mode: AccessControlMode) {
        self.list_mode = access_control_mode;
    }

    /// 添加名单
    pub fn set_access_control_list(&mut self, is_group: bool, change: SetAccessControlList) {
        match (change, is_group) {
            // 添加一个群组到名单
            (SetAccessControlList::Add(id), true) => {
                self.access_list.groups.insert(id);
            }
            // 添加多个群组到名单
            (SetAccessControlList::Adds(ids), true) => {
                for id in ids {
                    self.access_list.groups.insert(id);
                }
            }
            // 从名单中移除一个群组
            (SetAccessControlList::Remove(id), true) => {
                self.access_list.groups.remove(&id);
            }
            // 从名单中移除多个群组
            (SetAccessControlList::Removes(ids), true) => {
                for id in ids {
                    self.access_list.groups.remove(&id);
                }
            }
            // 替换名单为新的群组列表
            (SetAccessControlList::Changes(ids), true) => {
                self.access_list.groups = ids.into_iter().collect();
            }
            // 添加一个用户到名单
            (SetAccessControlList::Add(id), false) => {
                self.access_list.friends.insert(id);
            }
            // 添加多个用户到名单
            (SetAccessControlList::Adds(ids), false) => {
                self.access_list.friends.extend(ids);
            }
            // 从名单中移除一个用户
            (SetAccessControlList::Remove(id), false) => {
                self.access_list.friends.remove(&id);
            }
            // 从名单中移除多个用户
            (SetAccessControlList::Removes(ids), false) => {
                self.access_list.friends.retain(|&x| !ids.contains(&x));
            }
            // 替换名单为新的用户列表
            (SetAccessControlList::Changes(ids), false) => {
                self.access_list.friends = ids.into_iter().collect();
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct PluginStatus {
    pub(crate) enable_on_startup: bool,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) access_control: bool,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) list_mode: AccessControlMode,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) access_list: AccessList,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    /// 插件是否启用
    pub enabled: bool,
    /// 插件是否在Bot启动时启用
    pub enable_on_startup: bool,
    /// 插件是否启用框架级访问控制
    #[cfg(feature = "plugin-access-control")]
    pub access_control: bool,
    /// 插件的访问控制模式
    #[cfg(feature = "plugin-access-control")]
    pub list_mode: AccessControlMode,
    /// 插件的访问控制列表
    #[cfg(feature = "plugin-access-control")]
    pub access_list: AccessList,
}

#[cfg(feature = "plugin-access-control")]
#[derive(Debug, Clone)]
pub enum SetAccessControlList {
    /// 增加一个名单
    Add(i64),
    /// 增加多个名单
    Adds(Vec<i64>),
    /// 移除一个名单
    Remove(i64),
    /// 移除多个名单
    Removes(Vec<i64>),
    /// 替换名单成此名单
    Changes(Vec<i64>),
}
