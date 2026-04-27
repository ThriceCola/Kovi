use dialoguer::Input;
use dialoguer::theme::ColorfulTheme;
use serde::{Deserialize, Serialize};
use std::io::Write as _;
use std::path::Path;
use std::{env, fs};

use crate::error::BotBuildError;

/// 配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KoviConf {
    pub config: Config,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub main_admin: i64,
    pub admins: Vec<i64>,
    pub debug: bool,
}

impl KoviConf {
    pub fn new(main_admin: i64, admins: Option<Vec<i64>>, debug: bool) -> Self {
        KoviConf {
            config: Config {
                main_admin,
                admins: admins.unwrap_or_default(),
                debug,
            },
        }
    }
}

impl AsRef<KoviConf> for KoviConf {
    fn as_ref(&self) -> &KoviConf {
        self
    }
}

/// 将配置文件写入磁盘
fn read_from_path_config_write_and_return(file_path: &Path) -> Result<KoviConf, std::io::Error> {
    let main_admin: i64 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the ID of the main administrator? (Not used yet)")
        .allow_empty(true)
        .interact_text()
        .expect("unreachable");

    let config = KoviConf::new(
        main_admin, None,
        // Server::new(host, port, access_token, secure, path, all_in_one),
        false,
    );

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

    // Ensure we have a config table and set values
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

    let file = fs::File::create(file_path)?;
    let mut writer = std::io::BufWriter::new(file);
    writer.write_all(doc.to_string().as_bytes())?;

    Ok(config)
}

/// 读取本地Kovi.conf.toml文件
pub fn load_local_conf() -> Result<KoviConf, BotBuildError> {
    let path = Path::new("kovi.conf.toml");
    //检测文件kovi.conf.toml
    let kovi_conf_file_exist = fs::metadata(path).is_ok();

    #[derive(Debug, Clone, Deserialize, Serialize)]
    struct TempKoviConf {
        config: Option<Config>,
    }

    let conf_json: KoviConf = if kovi_conf_file_exist {
        match fs::read_to_string(path) {
            Ok(v) => match toml::from_str::<TempKoviConf>(&v) {
                Ok(conf) => match conf.config {
                    Some(config) => KoviConf { config },
                    None => read_from_path_config_write_and_return(path)
                        .map_err(|e| BotBuildError::FileCreateError(e.to_string()))?,
                },
                Err(err) => {
                    eprintln!("Configuration file parsing error: {err}");
                    read_from_path_config_write_and_return(path)
                        .map_err(|e| BotBuildError::FileCreateError(e.to_string()))?
                }
            },
            Err(err) => {
                return Err(BotBuildError::FileReadError(err.to_string()));
            }
        }
    } else {
        read_from_path_config_write_and_return(path)
            .map_err(|e| BotBuildError::FileCreateError(e.to_string()))?
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
