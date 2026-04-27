use ahash::{HashMap, HashMapExt as _, HashSet};
use parking_lot::RwLock;
// #[cfg(feature = "plugin-access-control")]
// use runtimebot::kovi_api::AccessList;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::fmt::Debug;
use std::fs;
use std::io::Write as _;
use std::sync::Arc;

use crate::config::kovi_conf::KoviConf;
use crate::drive::Drive;
use crate::error::BotError;

#[cfg(feature = "plugin-access-control")]
pub use crate::bot::runtimebot::kovi_api::AccessControlMode;
use crate::plugin::plugin_set::PluginSet;
use crate::plugin::{Plugin, PluginStatus};

pub(crate) mod handler;
pub(crate) mod run;

pub mod event;
pub mod runtimebot;

/// bot结构体
pub struct Bot {
    pub information: Arc<RwLock<BotInformation>>,
    pub drive: Arc<dyn Drive>,
    pub(crate) plugins: HashMap<String, Plugin>,
    pub(crate) run_abort: Vec<tokio::task::AbortHandle>,
}
impl Drop for Bot {
    fn drop(&mut self) {
        for i in self.run_abort.iter() {
            i.abort();
        }
    }
}

impl Bot {
    /// 构建一个bot实例
    /// # Examples
    /// ```
    /// use kovi::Bot;
    /// use kovi::bot::{KoviConf, Server};
    /// use std::net::{IpAddr, Ipv4Addr};
    ///
    /// let conf = KoviConf::new(
    ///     123456,
    ///     None,
    ///     Server {
    ///         host: kovi::bot::Host::IpAddr(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
    ///         port: 8081,
    ///         access_token: "".to_string(),
    ///         secure: false,
    ///     },
    ///     false,
    /// );
    /// let bot = Bot::build(conf);
    /// bot.run()
    /// ```
    pub fn build<C, D>(conf_from_template: C, drive: D) -> Bot
    where
        C: AsRef<KoviConf>,
        D: Drive + 'static,
    {
        let conf = conf_from_template.as_ref();

        Bot {
            information: Arc::new(RwLock::new(BotInformation {
                main_admin: conf.config.main_admin,
                deputy_admins: conf.config.admins.iter().cloned().collect(),
            })),
            drive: Arc::new(drive),
            plugins: HashMap::<_, _>::new(),
            run_abort: Vec::new(),
        }
    }

    /// 挂载插件。
    pub fn mount_plugin(&mut self, plugin: Plugin) {
        self.plugins.insert(plugin.name.clone(), plugin);
    }

    /// 挂载插件。
    pub fn mount_plugin_set(&mut self, plugin: PluginSet) {
        for plugin in plugin.set {
            self.mount_plugin(plugin);
        }
    }
}

impl Bot {
    /// 使用KoviConf设置插件在Bot启动时的状态
    ///
    /// 如果配置文件中没有对应的插件，将会被忽略，保留插件默认状态
    ///
    /// 如果配置文件读取失败或者解析toml失败，将会保留插件默认状态
    pub fn set_plugin_startup_use_file(mut self) -> Self {
        let file_path = "kovi.plugin.toml";
        let content = match fs::read_to_string(file_path) {
            Ok(v) => {
                log::debug!("Set plugin startup use file successfully");
                v
            }
            Err(e) => {
                log::debug!("Failed to read file: {e}");
                return self;
            }
        };
        let mut plugin_status_map: HashMap<String, PluginStatus> = match toml::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                log::debug!("Failed to parse toml: {e}");
                return self;
            }
        };

        for (name, plugin) in self.plugins.iter_mut() {
            if let Some(plugin_status) = plugin_status_map.remove(name) {
                plugin.enable_on_startup = plugin_status.enable_on_startup;
                plugin.enabled.send_modify(|v| {
                    *v = plugin_status.enable_on_startup;
                });
                #[cfg(feature = "plugin-access-control")]
                {
                    plugin.access_control = plugin_status.access_control;
                    plugin.list_mode = plugin_status.list_mode;
                    plugin.access_list = plugin_status.access_list;
                }
            }
        }

        self
    }

    /// 使用KoviConf设置插件在Bot启动时的状态
    ///
    /// 如果配置文件中没有对应的插件，将会被忽略，保留插件默认状态
    ///
    /// 如果配置文件读取失败或者解析toml失败，将会保留插件默认状态
    pub fn set_plugin_startup_use_file_ref(&mut self) {
        let file_path = "kovi.plugin.toml";
        let content = match fs::read_to_string(file_path) {
            Ok(v) => {
                log::debug!("Set plugin startup use file successfully");
                v
            }
            Err(e) => {
                log::debug!("Failed to read file: {e}");
                return;
            }
        };
        let mut plugin_status_map: HashMap<String, PluginStatus> = match toml::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                log::debug!("Failed to parse toml: {e}");
                return;
            }
        };

        for (name, plugin) in self.plugins.iter_mut() {
            if let Some(plugin_status) = plugin_status_map.remove(name) {
                plugin.enable_on_startup = plugin_status.enable_on_startup;
                plugin.enabled.send_modify(|v| {
                    *v = plugin_status.enable_on_startup;
                });
                #[cfg(feature = "plugin-access-control")]
                {
                    plugin.access_control = plugin_status.access_control;
                    plugin.list_mode = plugin_status.list_mode;
                    plugin.access_list = plugin_status.access_list;
                }
            }
        }
    }

    /// 设置全部插件在Bot启动时的状态
    pub fn set_all_plugin_startup(mut self, enabled: bool) -> Self {
        for plugin in self.plugins.values_mut() {
            plugin.enable_on_startup = enabled;
            plugin.enabled.send_modify(|v| {
                *v = enabled;
            });
        }
        self
    }

    /// 设置全部插件在Bot启动时的状态
    pub fn set_all_plugin_startup_ref(&mut self, enabled: bool) {
        for plugin in self.plugins.values_mut() {
            plugin.enable_on_startup = enabled;
            plugin.enabled.send_modify(|v| {
                *v = enabled;
            });
        }
    }

    /// 设置单个插件在Bot启动时的状态
    pub fn set_plugin_startup<T: AsRef<str>>(
        mut self,
        name: T,
        enabled: bool,
    ) -> Result<Self, BotError> {
        let name = name.as_ref();
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.enable_on_startup = enabled;
            plugin.enabled.send_modify(|v| {
                *v = enabled;
            });
            Ok(self)
        } else {
            Err(BotError::PluginNotFound(format!("Plugin {name} not found")))
        }
    }

    /// 设置单个插件在Bot启动时的状态
    pub fn set_plugin_startup_ref<T: AsRef<str>>(
        &mut self,
        name: T,
        enabled: bool,
    ) -> Result<(), BotError> {
        let name = name.as_ref();
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.enable_on_startup = enabled;
            plugin.enabled.send_modify(|v| {
                *v = enabled;
            });
            Ok(())
        } else {
            Err(BotError::PluginNotFound(format!("Plugin {name} not found")))
        }
    }

    #[cfg(any(feature = "save_plugin_status", feature = "save_bot_admin"))]
    pub(crate) fn save_bot_status(&self) {
        #[cfg(feature = "save_plugin_status")]
        {
            let _file_path = "kovi.plugin.toml";

            let mut plugin_status = HashMap::new();
            for (name, plugin) in self.plugins.iter() {
                plugin_status.insert(
                    name.clone(),
                    PluginStatus {
                        enable_on_startup: *plugin.enabled.borrow(),
                        #[cfg(feature = "plugin-access-control")]
                        access_control: plugin.access_control,
                        #[cfg(feature = "plugin-access-control")]
                        list_mode: plugin.list_mode,
                        #[cfg(feature = "plugin-access-control")]
                        access_list: plugin.access_list.clone(),
                    },
                );
            }

            let serialized = match toml::to_string(&plugin_status) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to serialize plugin status: {e}");
                    return;
                }
            };
            if let Err(e) = fs::write(_file_path, serialized) {
                log::error!("Failed to write plugin status to file: {e}");
            }
        }

        #[cfg(feature = "save_bot_admin")]
        {
            let file_path = "kovi.conf.toml";
            let existing_content = fs::read_to_string(file_path).unwrap_or_default();

            let mut doc = existing_content
                .parse::<toml_edit::DocumentMut>()
                .unwrap_or_else(|_| toml_edit::DocumentMut::new());

            // 确保 "config" 存在
            if !doc.contains_key("config") {
                doc["config"] = toml_edit::table();
            }

            let (main_admin, deputy_admins) = {
                let info = self.information.read();
                (info.main_admin, info.deputy_admins.clone())
            };

            // 更新 "config" 中的 admin 信息
            doc["config"]["main_admin"] = toml_edit::value(main_admin);
            doc["config"]["admins"] = toml_edit::Item::Value(toml_edit::Value::Array(
                deputy_admins
                    .iter()
                    .map(|&x| toml_edit::Value::from(x))
                    .collect(),
            ));

            match fs::File::create(file_path) {
                Ok(file) => {
                    let mut writer = std::io::BufWriter::new(file);
                    if let Err(e) = writer.write_all(doc.to_string().as_bytes()) {
                        log::error!("Failed to write to file: {e}");
                    }
                }
                Err(e) => {
                    log::error!("Failed to create file: {e}");
                }
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SendApi {
    pub action: String,
    pub params: Value,
    // echo: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiReturn {
    pub status: String,
    pub retcode: i32,
    pub data: Value,
}

/// bot信息结构体
#[derive(Debug, Clone)]
pub struct BotInformation {
    pub main_admin: i64,
    pub deputy_admins: HashSet<i64>,
}

impl std::fmt::Display for ApiReturn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status: {}, retcode: {}, data: {}",
            self.status, self.retcode, self.data
        )
    }
}

impl std::fmt::Display for SendApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).expect("unreachable"))
    }
}

impl SendApi {
    pub fn new(action: &str, params: Value) -> Self {
        SendApi {
            action: action.to_string(),
            params,
            // echo: Self::rand_echo(),
        }
    }
}

// #[macro_export]
// macro_rules! build_bot {
//     ($( $plugin:ident ),* $(,)* ) => {
//         {
//             let conf = match kovi::bot::Bot::load_local_conf() {
//                 Ok(c) => c,
//                 Err(e) => {
//                     eprintln!("Error loading config: {}", e);
//                     panic!("Failed to load config");
//                 }
//             };
//             kovi::logger::try_set_logger();
//             let mut bot = kovi::bot::Bot::build(&conf);

//             $(
//                 let plugin = $plugin::__kovi_build_plugin();
//                 kovi::log::info!("Mounting plugin: {}", &plugin.name);
//                 bot.mount_plugin(plugin);
//             )*

//             bot.set_plugin_startup_use_file_ref();
//             bot
//         }
//     };
// }
