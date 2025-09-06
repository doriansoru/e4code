//! Module for application settings management
//!
//! This module provides functionality for loading, saving, and managing
//! application settings such as theme, font, and last opened files.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Default font size for the editor
pub const DEFAULT_FONT_SIZE: f64 = 14.0;

/// Application settings structure
///
/// This struct holds all the configurable settings for the application,
/// including theme preferences, font settings, and file history.
#[derive(Serialize, Deserialize, Debug)]
pub struct AppSettings {
    /// Theme setting ("light" or "dark")
    pub theme: String,
    /// Font setting in Pango format (e.g., "Monospace 14")
    pub font: String,
    /// Last opened directory path
    pub last_opened_directory: Option<PathBuf>,
    /// List of last opened files
    pub last_opened_files: Option<Vec<PathBuf>>,
}

impl Default for AppSettings {
    /// Creates default application settings
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font: format!("Monospace {}", DEFAULT_FONT_SIZE),
            last_opened_directory: None,
            last_opened_files: None,
        }
    }
}

/// Gets the configuration file path
///
/// Returns the path to the configuration file in the user's config directory.
/// Creates the directory structure if it doesn't exist.
fn get_config_path() -> Option<PathBuf> {
    let mut path = dirs::config_dir()?;
    path.push("e4code");
    fs::create_dir_all(&path).ok()?;
    path.push("settings.json");
    Some(path)
}

/// Saves application settings to disk
///
/// Serializes the settings to JSON and writes them to the configuration file.
///
/// # Arguments
///
/// * `settings` - Reference to the settings to save
pub fn save_settings(settings: &AppSettings) {
    if let Some(path) = get_config_path() {
        if let Ok(json) = serde_json::to_string_pretty(settings) {
            fs::write(path, json).ok();
        }
    }
}

/// Loads application settings from disk
///
/// Reads and deserializes settings from the configuration file.
/// Returns default settings if the file doesn't exist or can't be read.
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
