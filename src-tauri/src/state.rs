use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::config::{self, AppConfig};
use crate::conversation::{self, Conversation};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorCaptureDto {
    pub index: usize,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RectDto {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CropResultDto {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedImage {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

pub struct AppState {
    pub session_active: AtomicBool,
    pub closing_overlays: AtomicBool,
    pub widget_visible: AtomicBool,
    pub manually_hidden: AtomicBool,
    pub auto_hidden: AtomicBool,
    pub bubble_visible: AtomicBool,
    pub bubble_height: Mutex<f64>,
    pub bubble_side: Mutex<String>,
    pub captures: Mutex<Vec<MonitorCaptureDto>>,
    pub session_dir: Mutex<Option<PathBuf>>,
    pub config: Mutex<AppConfig>,
    pub conversation: Mutex<Conversation>,
    pub staged: Mutex<Option<StagedImage>>,
    pub chat_running: AtomicBool,
    pub chat_abort: AtomicBool,
}

impl AppState {
    pub fn load(app: &AppHandle) -> Self {
        Self {
            session_active: AtomicBool::new(false),
            closing_overlays: AtomicBool::new(false),
            widget_visible: AtomicBool::new(false),
            manually_hidden: AtomicBool::new(false),
            auto_hidden: AtomicBool::new(false),
            bubble_visible: AtomicBool::new(false),
            bubble_height: Mutex::new(120.0),
            bubble_side: Mutex::new("left".to_string()),
            captures: Mutex::new(Vec::new()),
            session_dir: Mutex::new(None),
            config: Mutex::new(config::load(app)),
            conversation: Mutex::new(conversation::load(app)),
            staged: Mutex::new(None),
            chat_running: AtomicBool::new(false),
            chat_abort: AtomicBool::new(false),
        }
    }
}
