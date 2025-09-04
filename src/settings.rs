use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_FONT_SIZE: f64 = 14.0;

#[derive(Serialize, Deserialize, Debug)]
pub struct AppSettings {
    pub theme: String,
    pub font: String,
    pub last_opened_directory: Option<PathBuf>,
    pub last_opened_files: Option<Vec<PathBuf>>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font: format!("Monospace {}", DEFAULT_FONT_SIZE),
            last_opened_directory: None,
            last_opened_files: None,
        }
    }
}

pub fn get_config_path() -> Option<PathBuf> {
    let mut path = dirs::config_dir()?;
    path.push("e4code");
    fs::create_dir_all(&path).ok()?;
    path.push("settings.json");
    Some(path)
}

pub fn save_settings(settings: &AppSettings) {
    if let Some(path) = get_config_path() {
        if let Ok(json) = serde_json::to_string_pretty(settings) {
            fs::write(path, json).ok();
        }
    }
}

pub fn load_settings() -> AppSettings {
    if let Some(path) = get_config_path() {
        if let Ok(json) = fs::read_to_string(path) {
            if let Ok(settings) = serde_json::from_str(&json) {
                return settings;
            }
        }
    }
    AppSettings::default()
}
