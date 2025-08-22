use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub tutorial_seen: bool,
    pub token: Option<String>,
    pub username: Option<String>,
    pub user_icon: Option<String>,
    pub current_theme_name: crate::themes::ThemeName,
}

pub fn get_config_path() -> PathBuf {
    let mut config_dir = dirs::config_dir().unwrap();
    config_dir.push("reetui");
    fs::create_dir_all(&config_dir).unwrap();
    config_dir.push("reetui.json");
    config_dir
}

pub fn load_config() -> Config {
    let config_path = get_config_path();
    if config_path.exists() {
        let config_data = fs::read_to_string(config_path).unwrap();
        serde_json::from_str(&config_data).unwrap_or_default()
    } else {
        Config::default()
    }
}

pub fn save_config(config: &Config) {
    let config_path = get_config_path();
    let config_data = serde_json::to_string_pretty(config).unwrap();
    fs::write(config_path, config_data).unwrap();
}
