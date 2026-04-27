use std::fs;
use std::io::Write as _;
use std::net::Ipv4Addr;

use ahash::HashMap;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Select};
use serde::{Deserialize, Serialize};

/// 配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuntimeConfig {
    pub config: Config,
    #[serde(flatten)]
    pub expand: HashMap<String, toml::Value>,
    // config_template: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub main_admin: i64,
    pub admins: Vec<i64>,
    pub debug: bool,
}

impl RuntimeConfig {
    pub fn new(
        main_admin: i64,
        admins: Option<Vec<i64>>,
        debug: bool,
        expand: Option<HashMap<String, toml::Value>>,
    ) -> Self {
        RuntimeConfig {
            config: Config {
                main_admin,
                admins: admins.unwrap_or_default(),
                debug,
            },
            expand: expand.unwrap_or_default(),
        }
    }
}

impl AsRef<RuntimeConfig> for RuntimeConfig {
    fn as_ref(&self) -> &RuntimeConfig {
        self
    }
}

/// 将配置文件写入磁盘
fn config_file_write_and_return() -> Result<RuntimeConfig, std::io::Error> {
    enum HostType {
        IPv4,
        IPv6,
        Domain,
    }

    let host_type: HostType = {
        let items = ["IPv4", "IPv6", "Domain"];
        let select = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What is the type of the host of the OneBot server?")
            .items(&items)
            .default(0)
            .interact()
            .expect("unreachable");

        match select {
            0 => HostType::IPv4,
            1 => HostType::IPv6,
            2 => HostType::Domain,
            _ => panic!(), //不可能的事情
        }
    };

    let host = match host_type {
        HostType::IPv4 => {
            let ip = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("What is the IP of the OneBot server?")
                .default(Ipv4Addr::new(127, 0, 0, 1))
                .interact_text()
                .expect("unreachable");
            Host::IpAddr(IpAddr::V4(ip))
        }
        HostType::IPv6 => {
            let ip = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("What is the IP of the OneBot server?")
                .default(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))
                .interact_text()
                .expect("unreachable");
            Host::IpAddr(IpAddr::V6(ip))
        }
        HostType::Domain => {
            let domain = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("What is the domain of the OneBot server?")
                .default("localhost".to_string())
                .interact_text()
                .expect("unreachable");
            Host::Domain(domain)
        }
    };

    let port: u16 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the port of the OneBot server?")
        .default(8081)
        .interact_text()
        .expect("unreachable");

    let access_token: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the access_token of the OneBot server? (Optional)")
        .default("".to_string())
        .show_default(false)
        .interact_text()
        .expect("unreachable");

    let main_admin: i64 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the ID of the main administrator? (Not used yet)")
        .allow_empty(true)
        .interact_text()
        .expect("unreachable");

    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the route path of websocket server?")
        .default("/".to_string())
        .interact_text()
        .expect("unreachable");

    // 是否查看更多可选选项
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
    let mut all_in_one = false;
    if more {
        fn select_bool(prompt: &str) -> bool {
            let items = vec!["No", "Yes"];
            let select = Select::with_theme(&ColorfulTheme::default())
                // .with_prompt("Enable secure connection? (HTTPS/WSS)")
                .with_prompt(prompt)
                .items(&items)
                .default(0)
                .interact()
                .expect("unreachable");

            select == 1
        }
        secure = select_bool("Enable secure connection? (WSS)");
        all_in_one = select_bool("Use single ws api endpoint?");
    }

    let config = RuntimeConfig::new(
        main_admin, None,
        // Server::new(host, port, access_token, secure, path, all_in_one),
        false,
    );

    let mut doc = toml_edit::DocumentMut::new();
    doc["config"] = toml_edit::table();
    doc["config"]["main_admin"] = toml_edit::value(config.config.main_admin);
    doc["config"]["admins"] = toml_edit::Item::Value(toml_edit::Value::Array(
        config
            .config
            .admins
            .iter()
            .map(|&x| toml_edit::Value::from(x))
            .collect(),
    ));
    doc["config"]["debug"] = toml_edit::value(config.config.debug);

    doc["server"] = toml_edit::table();
    doc["server"]["host"] = match &config.server.host {
        Host::IpAddr(ip) => toml_edit::value(ip.to_string()),
        Host::Domain(domain) => toml_edit::value(domain),
    };
    doc["server"]["port"] = toml_edit::value(config.server.port as i64);
    doc["server"]["access_token"] = toml_edit::value(&config.server.access_token);
    doc["server"]["secure"] = toml_edit::value(config.server.secure);
    doc["server"]["path"] = toml_edit::value(&config.server.path);

    let file = fs::File::create("kovi.conf.toml")?;
    let mut writer = std::io::BufWriter::new(file);
    writer.write_all(doc.to_string().as_bytes())?;

    Ok(config)
}

/// 读取本地Kovi.conf.toml文件
pub fn load_local_conf() -> Result<RuntimeConfig, BotBuildError> {
    //检测文件是kovi.conf.json还是kovi.conf.toml
    let kovi_conf_file_exist = fs::metadata("kovi.conf.toml").is_ok();

    let conf_json: RuntimeConfig = if kovi_conf_file_exist {
        match fs::read_to_string("kovi.conf.toml") {
            Ok(v) => match toml::from_str(&v) {
                Ok(conf) => conf,
                Err(err) => {
                    eprintln!("Configuration file parsing error: {err}");
                    config_file_write_and_return()
                        .map_err(|e| BotBuildError::FileCreateError(e.to_string()))?
                }
            },
            Err(err) => {
                return Err(BotBuildError::FileReadError(err.to_string()));
            }
        }
    } else {
        config_file_write_and_return().map_err(|e| BotBuildError::FileCreateError(e.to_string()))?
    };

    unsafe {
        if env::var("RUST_LOG").is_err() {
            if conf_json.config.debug {
                env::set_var("RUST_LOG", "debug");
            } else {
                env::set_var("RUST_LOG", "info");
            }
        }
    }

    Ok(conf_json)
}
