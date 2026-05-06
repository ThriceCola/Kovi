use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Select};
use kovi::error::BotBuildError;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs;
use std::io::Write as _;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MilkyDriverConfig {
    pub server: Server,
}

/// server信息
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Server {
    pub host: Host,
    pub port: u16,
    pub access_token: String,
    pub secure: bool,
    /// path route to ws
    #[serde(default = "default_path")]
    pub path: String,
}

/// when not specified, use "/" instead.
fn default_path() -> String {
    "/".into()
}

impl Server {
    pub fn new(host: Host, port: u16, access_token: String, secure: bool, path: String) -> Self {
        Server {
            host,
            port,
            access_token,
            secure,
            path,
        }
    }
}

impl Server {
    /// 根据 path 后缀构建 WebSocket URL，例如 `ws_url("api")` → `ws://host:port/api`
    /// 如果启用了 all_in_one 模式，path 将被忽略
    pub fn ws_url(&self, path: &str) -> String {
        let protocol = if self.secure { "wss" } else { "ws" };
        let host = match &self.host {
            Host::IpAddr(std::net::IpAddr::V6(ip)) => format!("[{ip}]"),
            Host::IpAddr(ip) => ip.to_string(),
            Host::Domain(d) => d.clone(),
        };

        format!(
            "{protocol}://{host}:{self_port}{self_path}/{path}",
            self_port = self.port,
            self_path = match self.path.as_str() {
                "" => String::new(),
                p => format!("{p}"),
            },
        )
    }

    pub fn api_url(&self, path: &str) -> String {
        let protocol = if self.secure { "https" } else { "http" };
        let host = match &self.host {
            Host::IpAddr(std::net::IpAddr::V6(ip)) => format!("[{ip}]"),
            Host::IpAddr(ip) => ip.to_string(),
            Host::Domain(d) => d.clone(),
        };

        format!(
            "{protocol}://{host}:{self_port}{self_path}/api/{path}",
            self_port = self.port,
            self_path = match self.path.as_str() {
                "" => String::new(),
                p => format!("{p}"),
            },
        )
    }
}

impl AsRef<MilkyDriverConfig> for MilkyDriverConfig {
    fn as_ref(&self) -> &MilkyDriverConfig {
        self
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Host {
    IpAddr(IpAddr),
    Domain(String),
}

impl Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Host::IpAddr(ip) => write!(f, "{ip}"),
            Host::Domain(domain) => write!(f, "{domain}"),
        }
    }
}

/// 将配置文件写入磁盘
fn config_file_write_and_return(file_path: &Path) -> Result<MilkyDriverConfig, std::io::Error> {
    enum HostType {
        IPv4,
        IPv6,
        Domain,
    }

    let host_type: HostType = {
        let items = ["IPv4", "IPv6", "Domain"];
        let select = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What is the type of the host of the Milky server?")
            .items(&items)
            .default(0)
            .interact()
            .expect("unreachable");

        match select {
            0 => HostType::IPv4,
            1 => HostType::IPv6,
            2 => HostType::Domain,
            _ => panic!(), // 不可能的事情
        }
    };

    let host = match host_type {
        HostType::IPv4 => {
            let ip = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("What is the IP of the Milky server?")
                .default(Ipv4Addr::new(127, 0, 0, 1))
                .interact_text()
                .expect("unreachable");
            Host::IpAddr(IpAddr::V4(ip))
        }
        HostType::IPv6 => {
            let ip = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("What is the IP of the Milky server?")
                .default(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))
                .interact_text()
                .expect("unreachable");
            Host::IpAddr(IpAddr::V6(ip))
        }
        HostType::Domain => {
            let domain = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("What is the domain of the Milky server?")
                .default("localhost".to_string())
                .interact_text()
                .expect("unreachable");
            Host::Domain(domain)
        }
    };

    let port: u16 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the port of the Milky server?")
        .default(8081)
        .interact_text()
        .expect("unreachable");

    let access_token: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the access_token of the Milky server? (Optional)")
        .default("".to_string())
        .show_default(false)
        .interact_text()
        .expect("unreachable");

    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the route path of websocket server?")
        .default("/".to_string())
        .interact_text()
        .expect("unreachable");

    let more: bool = {
        let items = ["No", "Yes"];
        let select = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to view more optional options?")
            .items(&items)
            .default(0)
            .interact()
            .expect("unreachable");

        match select {
            0 => false,
            1 => true,
            _ => unreachable!(),
        }
    };

    let mut secure = false;

    if more {
        fn select_bool(prompt: &str) -> bool {
            let items = ["No", "Yes"];
            let select = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(prompt)
                .items(&items)
                .default(0)
                .interact()
                .expect("unreachable");

            select == 1
        }
        secure = select_bool("Enable secure connection? (WSS)");
    }

    let config = MilkyDriverConfig {
        server: Server {
            host,
            port,
            access_token,
            secure,
            path,
        },
    };

    let mut doc = match fs::read_to_string(file_path) {
        Ok(content) => match content.parse::<toml_edit::DocumentMut>() {
            Ok(d) => d,
            Err(err) => {
                eprintln!(
                    "Failed to parse existing config, creating new document: {}",
                    err
                );
                toml_edit::DocumentMut::new()
            }
        },
        Err(_) => toml_edit::DocumentMut::new(),
    };

    doc["server"] = toml_edit::table();
    doc["server"]["host"] = match &config.server.host {
        Host::IpAddr(ip) => toml_edit::value(ip.to_string()),
        Host::Domain(domain) => toml_edit::value(domain),
    };
    doc["server"]["port"] = toml_edit::value(config.server.port as i64);
    doc["server"]["access_token"] = toml_edit::value(&config.server.access_token);
    doc["server"]["secure"] = toml_edit::value(config.server.secure);
    doc["server"]["path"] = toml_edit::value(&config.server.path);

    let file = fs::File::create(file_path)?;
    let mut writer = std::io::BufWriter::new(file);
    writer.write_all(doc.to_string().as_bytes())?;

    Ok(config)
}

/// 读取本地Kovi.conf.toml文件
pub fn load_local_conf() -> Result<MilkyDriverConfig, BotBuildError> {
    let path = Path::new("kovi.conf.toml");
    let kovi_conf_file_exist = fs::metadata(path).is_ok();

    #[derive(Deserialize, Serialize, Debug, Clone)]
    struct TempKoviConfig {
        server: Option<Server>,
    }

    let conf_json: MilkyDriverConfig = if kovi_conf_file_exist {
        match fs::read_to_string(path) {
            Ok(v) => match toml::from_str::<TempKoviConfig>(&v) {
                Ok(conf) => match conf.server {
                    Some(server) => MilkyDriverConfig { server },
                    None => config_file_write_and_return(path)
                        .map_err(|e| BotBuildError::FileCreateError(e.to_string()))?,
                },
                Err(err) => {
                    eprintln!("Configuration file parsing error: {err}");
                    config_file_write_and_return(path)
                        .map_err(|e| BotBuildError::FileCreateError(e.to_string()))?
                }
            },
            Err(err) => {
                return Err(BotBuildError::FileReadError(err.to_string()));
            }
        }
    } else {
        config_file_write_and_return(path)
            .map_err(|e| BotBuildError::FileCreateError(e.to_string()))?
    };

    Ok(conf_json)
}
