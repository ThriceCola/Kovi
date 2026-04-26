use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

fn save_data(data: &[u8], file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = file_path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)?;
    }

    let mut file = File::create(file_path)?;
    file.write_all(data)?;

    Ok(())
}

/// 加载本地json数据，如果没有则保存传入的数据进指定路径
pub fn load_json_data<T, P>(data: T, file_path: P) -> Result<T, Box<dyn std::error::Error>>
where
    T: Serialize + DeserializeOwned,
    P: AsRef<Path>,
{
    if !file_path.as_ref().exists() {
        let serialized_data = serde_json::to_string(&data)?;
        save_data(serialized_data.as_bytes(), file_path.as_ref())?;
        return Ok(data);
    }

    let mut file = File::open(&file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let deserialized_data = serde_json::from_str(&contents)?;

    Ok(deserialized_data)
}

/// 加载本地toml数据，如果没有则保存传入的数据进指定路径
pub fn load_toml_data<T, P>(data: T, file_path: P) -> Result<T, Box<dyn std::error::Error>>
where
    T: Serialize + DeserializeOwned,
    P: AsRef<Path>,
{
    if !file_path.as_ref().exists() {
        let serialized_data = toml::to_string(&data)?;
        save_data(serialized_data.as_bytes(), file_path.as_ref())?;
        return Ok(data);
    }

    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let deserialized_data = toml::from_str(&contents)?;

    Ok(deserialized_data)
}

/// 将json数据保存在传入的地址
pub fn save_json_data<T, P>(data: &T, file_path: P) -> Result<(), Box<dyn std::error::Error>>
where
    T: Serialize,
    P: AsRef<Path>,
{
    let serialized_data = serde_json::to_string(data)?;
    save_data(serialized_data.as_bytes(), file_path.as_ref())?;
    Ok(())
}

/// 将toml数据保存在传入的地址
pub fn save_toml_data<T, P>(data: &T, file_path: P) -> Result<(), Box<dyn std::error::Error>>
where
    T: Serialize,
    P: AsRef<Path>,
{
    let serialized_data = toml::to_string(data)?;
    save_data(serialized_data.as_bytes(), file_path.as_ref())?;
    Ok(())
}

/// 获取插件数据根目录
///
/// 如：`bot.get_data_path()` 是 `/path/data/plugin_name/`
///
/// 那么这个插件的数据目录就是 `/path/data/`
pub fn get_data_root_path() -> PathBuf {
    let mut current_dir = std::env::current_dir().expect("Get current directory failed");
    current_dir.push("data");
    current_dir
}

/// 计算pskey值
pub fn calculate_pskey(skey: &str) -> u32 {
    let mut hash: u32 = 5381;
    for character in skey.chars() {
        hash = (hash << 5)
            .wrapping_add(hash)
            .wrapping_add(character as u32);
    }
    hash & 0x7fffffff
}
