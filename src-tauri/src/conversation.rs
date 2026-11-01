use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub text: String,
    #[serde(default)]
    pub images: Vec<String>,
    pub created_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub updated_at: u64,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            messages: Vec::new(),
            updated_at: now(),
        }
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn new_message(role: &str, text: String, images: Vec<String>) -> ChatMessage {
    ChatMessage {
        id: uuid::Uuid::new_v4().to_string(),
        role: role.to_string(),
        text,
        images,
        created_at: now(),
    }
}

pub fn conversation_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("conversation.json"))
        .map_err(|error| error.to_string())
}

pub fn image_dir(app: &AppHandle, conversation_id: &str) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| {
            dir.join("conversations")
                .join(conversation_id)
                .join("images")
        })
        .map_err(|error| error.to_string())
}

pub fn load(app: &AppHandle) -> Conversation {
    let Ok(path) = conversation_path(app) else {
        return Conversation::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Conversation::new();
    };
    let mut conversation: Conversation = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => return Conversation::new(),
    };
    if conversation.id.trim().is_empty() {
        conversation.id = uuid::Uuid::new_v4().to_string();
    }
    conversation
}

pub fn save(app: &AppHandle, conversation: &Conversation) -> Result<(), String> {
    let path = conversation_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(conversation).map_err(|error| error.to_string())?;
    std::fs::write(path, text).map_err(|error| error.to_string())
}
