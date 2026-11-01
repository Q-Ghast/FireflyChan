use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const KEYRING_SERVICE: &str = "FireflyChan";
const KEYRING_USER: &str = "deepseek";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn to_tauri(self) -> Option<tauri::Theme> {
        match self {
            Self::System => None,
            Self::Light => Some(tauri::Theme::Light),
            Self::Dark => Some(tauri::Theme::Dark),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub theme: ThemeMode,
    #[serde(default)]
    pub widget_x: Option<i32>,
    #[serde(default)]
    pub widget_y: Option<i32>,
    #[serde(default = "default_true")]
    pub web_search: bool,
    #[serde(default)]
    pub quick_prompts: Vec<String>,
}

pub fn normalize_prompts(prompts: Vec<String>) -> Vec<String> {
    let mut normalized: Vec<String> = prompts
        .into_iter()
        .take(9)
        .map(|prompt| prompt.trim().to_string())
        .collect();
    while normalized.len() < 9 {
        normalized.push(String::new());
    }
    normalized
}

fn default_true() -> bool {
    true
}

fn default_base_url() -> String {
    "https://api.deepseek.com".to_string()
}

fn default_model() -> String {
    "deepseek-flash".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            model: default_model(),
            theme: ThemeMode::System,
            widget_x: None,
            widget_y: None,
            web_search: true,
            quick_prompts: normalize_prompts(Vec::new()),
        }
    }
}

pub fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("config.json"))
        .map_err(|error| error.to_string())
}

pub fn load(app: &AppHandle) -> AppConfig {
    let Ok(path) = config_path(app) else {
        return AppConfig::default();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return AppConfig::default();
    };
    let text = text.trim_start_matches('\u{feff}');
    serde_json::from_str(text).unwrap_or_default()
}

pub fn save(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(config).map_err(|error| error.to_string())?;
    std::fs::write(path, text).map_err(|error| error.to_string())
}

pub fn get_api_key() -> Option<String> {
    if let Ok(key) = std::env::var("FIREFLY_API_KEY") {
        if !key.trim().is_empty() {
            return Some(key);
        }
    }
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()?;
    entry.get_password().ok().filter(|value| !value.trim().is_empty())
}

pub fn has_api_key() -> bool {
    get_api_key().is_some()
}

pub fn set_api_key(key: &str) -> Result<(), String> {
    let entry =
        keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|error| error.to_string())?;
    entry
        .set_password(key)
        .map_err(|error| format!("保存 API Key 失败：{error}"))
}
