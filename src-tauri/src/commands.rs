use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

use crate::ai;
use crate::config::{self, AppConfig};
use crate::conversation::{self, ChatMessage};
use crate::state::{AppState, CropResultDto, MonitorCaptureDto, RectDto, StagedImage};
use crate::{capture, windows, win32};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationView {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub staged_image: Option<StagedImage>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionChatDto {
    pub crop: CropResultDto,
    pub conversation: ConversationView,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicConfig {
    pub base_url: String,
    pub model: String,
    pub has_api_key: bool,
    pub theme: String,
    pub quick_prompts: Vec<String>,
}

fn conversation_view(app: &AppHandle) -> ConversationView {
    let state = app.state::<AppState>();
    let conversation = state.conversation.lock().unwrap().clone();
    let staged = state.staged.lock().unwrap().clone();
    ConversationView {
        id: conversation.id,
        messages: conversation.messages,
        staged_image: staged,
    }
}

fn reset_session(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.session_active.store(false, Ordering::SeqCst);
    state.captures.lock().unwrap().clear();
}

pub async fn start_capture(app: AppHandle) -> Result<(), String> {
    if windows::close_chat_window(&app) {
        let state = app.state::<AppState>();
        state.chat_abort.store(true, Ordering::SeqCst);
        for _ in 0..20 {
            if !state.chat_running.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    let was_active = {
        let state = app.state::<AppState>();
        let active = state.session_active.swap(true, Ordering::SeqCst);
        if active {
            state.chat_abort.store(true, Ordering::SeqCst);
            *state.staged.lock().unwrap() = None;
        }
        active
    };

    if was_active {
        for _ in 0..20 {
            if !app
                .state::<AppState>()
                .chat_running
                .load(Ordering::SeqCst)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        windows::close_overlays(&app);
        {
            let state = app.state::<AppState>();
            state.captures.lock().unwrap().clear();
        }
    }

    {
        let state = app.state::<AppState>();
        let old_dir = { state.session_dir.lock().unwrap().take() };
        if let Some(old_dir) = old_dir {
            let _ = std::fs::remove_dir_all(old_dir);
        }
    }

    windows::hide_for_capture(&app);
    tokio::time::sleep(Duration::from_millis(30)).await;

    let capture_app = app.clone();
    let captured = tokio::task::spawn_blocking(move || capture::capture_all(&capture_app))
        .await
        .map_err(|error| error.to_string())?;

    let (session_id, captures, dir) = match captured {
        Ok(value) => value,
        Err(error) => {
            reset_session(&app);
            windows::show_widget(&app);
            return Err(error);
        }
    };

    {
        let state = app.state::<AppState>();
        *state.captures.lock().unwrap() = captures.clone();
        *state.session_dir.lock().unwrap() = Some(dir);
    }

    if let Err(error) = windows::create_overlays(&app, &captures) {
        windows::close_overlays(&app);
        reset_session(&app);
        windows::show_widget(&app);
        return Err(error.to_string());
    }

    windows::show_widget(&app);

    let _ = app.emit(
        "session-ready",
        serde_json::json!({
            "sessionId": session_id,
            "monitors": captures,
        }),
    );

    Ok(())
}

#[tauri::command]
pub async fn session_start(app: AppHandle) -> Result<(), String> {
    start_capture(app).await
}

#[tauri::command]
pub async fn session_confirm(
    app: AppHandle,
    monitor_index: usize,
    rect: RectDto,
) -> Result<SessionChatDto, String> {
    let capture = {
        let state = app.state::<AppState>();
        let captures = state.captures.lock().unwrap();
        captures
            .iter()
            .find(|item| item.index == monitor_index)
            .cloned()
    }
    .ok_or_else(|| "截图数据不存在，请重新框选".to_string())?;

    let crop = tokio::task::spawn_blocking(move || capture::crop_capture(&capture, &rect))
        .await
        .map_err(|error| error.to_string())??;

    let conversation_id = {
        let state = app.state::<AppState>();
        let conversation = state.conversation.lock().unwrap();
        conversation.id.clone()
    };
    let image_dir = conversation::image_dir(&app, &conversation_id)?;
    std::fs::create_dir_all(&image_dir).map_err(|error| error.to_string())?;
    let final_path = image_dir.join(format!("{}.png", uuid::Uuid::new_v4()));
    std::fs::copy(&crop.path, &final_path).map_err(|error| error.to_string())?;

    let staged = StagedImage {
        path: final_path.to_string_lossy().into_owned(),
        width: crop.width,
        height: crop.height,
    };

    {
        let state = app.state::<AppState>();
        *state.staged.lock().unwrap() = Some(staged.clone());
    }

    windows::raise_widget(&app);

    Ok(SessionChatDto {
        crop: CropResultDto {
            path: staged.path.clone(),
            width: staged.width,
            height: staged.height,
        },
        conversation: conversation_view(&app),
    })
}

#[tauri::command]
pub fn session_cancel(app: AppHandle) -> Result<(), String> {
    {
        let state = app.state::<AppState>();
        state.session_active.store(false, Ordering::SeqCst);
        state.chat_abort.store(true, Ordering::SeqCst);
        *state.staged.lock().unwrap() = None;
    }
    windows::close_overlays(&app);
    reset_session(&app);
    windows::show_widget(&app);
    Ok(())
}

#[tauri::command]
pub fn overlay_ready(window: WebviewWindow) {
    windows::reveal_overlay(&window);
}

#[tauri::command]
pub fn overlay_info(state: State<'_, AppState>, index: usize) -> Option<MonitorCaptureDto> {
    state
        .captures
        .lock()
        .unwrap()
        .iter()
        .find(|item| item.index == index)
        .cloned()
}

#[tauri::command]
pub fn conversation_get(app: AppHandle) -> ConversationView {
    conversation_view(&app)
}

#[tauri::command]
pub fn conversation_new(app: AppHandle) -> Result<ConversationView, String> {
    let conversation = conversation::Conversation::new();
    {
        let state = app.state::<AppState>();
        state.chat_abort.store(true, Ordering::SeqCst);
        *state.conversation.lock().unwrap() = conversation.clone();
        *state.staged.lock().unwrap() = None;
    }
    conversation::save(&app, &conversation)?;
    Ok(conversation_view(&app))
}

#[tauri::command]
pub fn staged_clear(app: AppHandle) {
    let state = app.state::<AppState>();
    *state.staged.lock().unwrap() = None;
}

#[tauri::command]
pub fn chat_send(app: AppHandle, text: String, web_search: bool) -> Result<ChatMessage, String> {
    let state = app.state::<AppState>();
    if state.chat_running.swap(true, Ordering::SeqCst) {
        return Err("正在生成回复，请稍候或先停止。".to_string());
    }

    if config::get_api_key().is_none() {
        state.chat_running.store(false, Ordering::SeqCst);
        return Err("还没有配置 API Key，请先在设置中填写。".to_string());
    }

    let staged = state.staged.lock().unwrap().take();
    let mut text = text.trim().to_string();
    if text.is_empty() {
        if staged.is_some() {
            text = "请看看这张截图。".to_string();
        } else {
            state.chat_running.store(false, Ordering::SeqCst);
            return Err("请输入想说的话，或先截一张图。".to_string());
        }
    }

    let images = staged
        .as_ref()
        .map(|image| vec![image.path.clone()])
        .unwrap_or_default();
    let search_query = if web_search {
        Some(text.clone())
    } else {
        None
    };
    let message = conversation::new_message("user", text, images);

    {
        let mut conversation = state.conversation.lock().unwrap();
        conversation.messages.push(message.clone());
        conversation.updated_at = conversation::now();
        if let Err(error) = conversation::save(&app, &conversation) {
            state.chat_running.store(false, Ordering::SeqCst);
            return Err(error);
        }
    }

    state.chat_abort.store(false, Ordering::SeqCst);
    let _ = app.emit("chat-user", message.clone());

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let search_context = if let Some(query) = search_query {
            let _ = app_handle.emit("chat-status", json!({ "status": "searching" }));
            match crate::search::web_search(&query, 5).await {
                Ok(results) if !results.is_empty() => Some(crate::search::format_context(&results)),
                Ok(_) => None,
                Err(error) => {
                    log::warn!("web search failed: {error}");
                    None
                }
            }
        } else {
            None
        };
        let _ = app_handle.emit("chat-status", json!({ "status": "thinking" }));
        if let Err(error) = ai::stream_chat(app_handle, search_context).await {
            log::error!("chat failed: {error}");
        }
    });

    Ok(message)
}

#[tauri::command]
pub fn chat_abort(app: AppHandle) {
    app.state::<AppState>()
        .chat_abort
        .store(true, Ordering::SeqCst);
}

#[tauri::command]
pub fn config_get(state: State<'_, AppState>) -> PublicConfig {
    let config = state.config.lock().unwrap().clone();
    PublicConfig {
        base_url: config.base_url,
        model: config.model,
        has_api_key: config::has_api_key(),
        theme: config.theme.as_str().to_string(),
        quick_prompts: config::normalize_prompts(config.quick_prompts),
    }
}

#[tauri::command]
pub fn theme_set(app: AppHandle, theme: String) -> Result<PublicConfig, String> {
    let mode = crate::config::ThemeMode::parse(&theme);
    {
        let state = app.state::<AppState>();
        state.config.lock().unwrap().theme = mode;
        let config = state.config.lock().unwrap().clone();
        config::save(&app, &config)?;
    }
    windows::apply_theme(&app, mode);
    Ok(config_get(app.state::<AppState>()))
}

#[tauri::command]
pub fn config_save(
    app: AppHandle,
    api_key: Option<String>,
    base_url: String,
    model: String,
    theme: Option<String>,
    quick_prompts: Option<Vec<String>>,
) -> Result<PublicConfig, String> {
    let base_url = base_url.trim().trim_end_matches('/').to_string();
    let model = model.trim().to_string();
    if base_url.is_empty() {
        return Err("Base URL 不能为空".to_string());
    }
    if model.is_empty() {
        return Err("模型名不能为空".to_string());
    }
    if let Some(key) = api_key {
        let key = key.trim();
        if !key.is_empty() {
            config::set_api_key(key)?;
        }
    }

    let current = {
        let state = app.state::<AppState>();
        let config = state.config.lock().unwrap().clone();
        config
    };

    let mode = theme
        .as_deref()
        .map(crate::config::ThemeMode::parse)
        .unwrap_or(current.theme);

    let prompts = quick_prompts
        .map(crate::config::normalize_prompts)
        .unwrap_or_else(|| crate::config::normalize_prompts(current.quick_prompts.clone()));

    let config = AppConfig {
        base_url,
        model,
        theme: mode,
        widget_x: current.widget_x,
        widget_y: current.widget_y,
        web_search: current.web_search,
        quick_prompts: prompts.clone(),
    };
    {
        let state = app.state::<AppState>();
        *state.config.lock().unwrap() = config.clone();
    }
    config::save(&app, &config)?;
    windows::apply_theme(&app, mode);
    let _ = app.emit("prompts-changed", json!({ "prompts": prompts }));

    Ok(PublicConfig {
        base_url: config.base_url,
        model: config.model,
        has_api_key: config::has_api_key(),
        theme: config.theme.as_str().to_string(),
        quick_prompts: config.quick_prompts,
    })
}

#[tauri::command]
pub fn quick_prompts_get(state: State<'_, AppState>) -> Vec<String> {
    let prompts = state.config.lock().unwrap().quick_prompts.clone();
    crate::config::normalize_prompts(prompts)
}

#[tauri::command]
pub async fn config_test(app: AppHandle) -> Result<String, String> {
    ai::test_connection(&app).await
}

#[tauri::command]
pub fn raise_companions(app: AppHandle) {
    windows::raise_companions(&app);
}

#[tauri::command]
pub fn widget_menu(app: AppHandle) -> Result<(), String> {
    let menu = windows::widget_menu(&app).map_err(|error| error.to_string())?;
    let window = app
        .get_webview_window("widget")
        .ok_or_else(|| "找不到挂件窗口".to_string())?;
    window.popup_menu(&menu).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn widget_drag(window: WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn widget_show(app: AppHandle) -> Result<(), String> {
    windows::show_widget(&app);
    Ok(())
}

#[tauri::command]
pub fn widget_position_save(app: AppHandle, x: i32, y: i32) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut config = state.config.lock().unwrap().clone();
    if config.widget_x == Some(x) && config.widget_y == Some(y) {
        return Ok(());
    }
    config.widget_x = Some(x);
    config.widget_y = Some(y);
    config::save(&app, &config)?;
    *state.config.lock().unwrap() = config;
    Ok(())
}

#[tauri::command]
pub fn web_search_get(state: State<'_, AppState>) -> bool {
    state.config.lock().unwrap().web_search
}

#[tauri::command]
pub fn web_search_set(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let state = app.state::<AppState>();
    let mut config = state.config.lock().unwrap().clone();
    config.web_search = enabled;
    config::save(&app, &config)?;
    *state.config.lock().unwrap() = config;
    Ok(enabled)
}

#[tauri::command]
pub fn widget_hide(app: AppHandle) -> Result<(), String> {
    windows::hide_widget_manual(&app);
    Ok(())
}

#[tauri::command]
pub fn bubble_show(app: AppHandle, text: String) -> Result<(), String> {
    windows::show_bubble(&app, &text).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn bubble_hide(app: AppHandle) {
    windows::hide_bubble(&app);
}

#[tauri::command]
pub fn bubble_resize(app: AppHandle, height: f64) -> Result<(), String> {
    windows::resize_bubble(&app, height).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn fullscreen_test() -> bool {
    win32::is_foreground_fullscreen()
}

#[tauri::command]
pub fn window_drag(window: WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    windows::open_settings(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_chat(app: AppHandle) -> Result<(), String> {
    windows::open_chat(&app).map_err(|error| error.to_string())
}
