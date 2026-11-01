use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    window::{Color, Effect, EffectState, EffectsBuilder},
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindowBuilder,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
};

use crate::config::ThemeMode;
use crate::state::{AppState, MonitorCaptureDto};
use crate::win32;

pub fn create_widget(app: &AppHandle) -> anyhow::Result<()> {
    if app.get_webview_window("widget").is_some() {
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(app, "widget", WebviewUrl::App("widget.html".into()))
        .title("流萤")
        .inner_size(114.0, 114.0)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focusable(false)
        .no_redirection_bitmap(true)
        .visible(false)
        .build()?;

    let outer = window
        .outer_size()
        .unwrap_or(PhysicalSize::new(170, 143));
    let saved = {
        let state = app.state::<AppState>();
        let config = state.config.lock().unwrap();
        config.widget_x.zip(config.widget_y)
    };
    let (x, y) = if let Some((saved_x, saved_y)) = saved {
        let (left, top, right, bottom) = win32::monitor_bounds_at(saved_x, saved_y);
        let max_x = (right - outer.width as i32).max(left);
        let max_y = (bottom - outer.height as i32).max(top);
        (saved_x.clamp(left, max_x), saved_y.clamp(top, max_y))
    } else if let Some(monitor) = window.primary_monitor()? {
        let scale = monitor.scale_factor();
        let margin = (16.0 * scale).round() as i32;
        let (area_x, area_y, area_width, area_height) = win32::primary_work_area();
        (
            area_x + area_width as i32 - outer.width as i32 - margin,
            area_y + area_height as i32 - outer.height as i32 - margin,
        )
    } else {
        (0, 0)
    };
    window.set_position(PhysicalPosition::new(x, y))?;

    window.set_ignore_cursor_events(true)?;
    window.show()?;
    bring_to_top(&window);

    let state = app.state::<AppState>();
    state.widget_visible.store(true, Ordering::SeqCst);
    Ok(())
}

fn bring_to_top(window: &tauri::WebviewWindow) {
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }
}

pub fn create_bubble(app: &AppHandle) -> anyhow::Result<()> {
    if app.get_webview_window("bubble").is_some() {
        return Ok(());
    }

    let _window = WebviewWindowBuilder::new(app, "bubble", WebviewUrl::App("bubble.html".into()))
        .title("流萤")
        .inner_size(360.0, 140.0)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focusable(true)
        .no_redirection_bitmap(true)
        .visible(false)
        .build()?;

    Ok(())
}

pub fn create_tray(app: &AppHandle) -> anyhow::Result<()> {
    let capture = MenuItem::with_id(app, "capture", "截图", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle_widget", "显示/隐藏挂件", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&capture, &toggle, &settings, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("缺少默认窗口图标"))?;

    let tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("流萤 · FireflyChan")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_widget(tray.app_handle());
            }
        })
        .build(app)?;

    // Keep the tray icon alive for the whole application lifetime.
    std::mem::forget(tray);
    Ok(())
}

pub fn widget_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let capture = MenuItem::with_id(app, "capture", "截图", true, None::<&str>)?;
    let chat = MenuItem::with_id(app, "open_chat", "打开对话", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide_widget", "隐藏挂件", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    Menu::with_items(app, &[&capture, &chat, &hide, &settings, &quit])
}

pub fn hide_for_capture(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("widget") {
        let _ = window.hide();
    }
    if let Some(window) = app.get_webview_window("bubble") {
        let _ = window.hide();
    }
    let state = app.state::<AppState>();
    state.widget_visible.store(false, Ordering::SeqCst);
    state.bubble_visible.store(false, Ordering::SeqCst);
}

pub fn show_widget(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("widget") {
        let _ = window.show();
        let _ = window.set_always_on_top(true);
        bring_to_top(&window);
    }
    let state = app.state::<AppState>();
    state.widget_visible.store(true, Ordering::SeqCst);
    state.manually_hidden.store(false, Ordering::SeqCst);
    state.auto_hidden.store(false, Ordering::SeqCst);
}

pub fn show_widget_no_flag(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("widget") {
        let _ = window.show();
        let _ = window.set_always_on_top(true);
        bring_to_top(&window);
    }
    let state = app.state::<AppState>();
    state.widget_visible.store(true, Ordering::SeqCst);
    state.auto_hidden.store(false, Ordering::SeqCst);
}

pub fn raise_widget(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("widget") {
        if !window.is_visible().unwrap_or(false) {
            let _ = window.show();
        }
        bring_to_top(&window);
    }
}

pub fn raise_bubble(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("bubble") {
        if window.is_visible().unwrap_or(false) {
            bring_to_top(&window);
        }
    }
}

pub fn raise_companions(app: &AppHandle) {
    raise_widget(app);
    if app
        .state::<AppState>()
        .bubble_visible
        .load(Ordering::SeqCst)
    {
        raise_bubble(app);
    }
}

pub fn apply_theme(app: &AppHandle, mode: ThemeMode) {
    let theme = mode.to_tauri();
    for (_, window) in app.webview_windows() {
        let _ = window.set_theme(theme);
    }
    let _ = app.emit("theme-changed", json!({ "theme": mode.as_str() }));
}

pub fn apply_theme_to_window(app: &AppHandle, window: &tauri::WebviewWindow) {
    let mode = app.state::<AppState>().config.lock().unwrap().theme;
    let _ = window.set_theme(mode.to_tauri());
}

fn apply_acrylic(window: &tauri::WebviewWindow) {
    let effects = EffectsBuilder::new()
        .effect(Effect::Acrylic)
        .state(EffectState::Active)
        .build();
    let _ = window.set_effects(effects);
}

pub fn open_chat(app: &AppHandle) -> anyhow::Result<()> {
    if app.state::<AppState>().session_active.load(Ordering::SeqCst) {
        return Ok(());
    }

    if let Some(window) = app.get_webview_window("chat") {
        let _ = window.show();
        let _ = window.set_focus();
        apply_theme_to_window(app, &window);
        apply_acrylic(&window);
        raise_widget(app);
        return Ok(());
    }

    let (area_x, area_y, _, area_height) = win32::primary_work_area();
    let window = WebviewWindowBuilder::new(app, "chat", WebviewUrl::App("chat.html".into()))
        .title("流萤 · 对话")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .resizable(false)
        .no_redirection_bitmap(true)
        .visible(false)
        .build()?;

    let scale = window.scale_factor().unwrap_or(1.0);
    let width = (440.0 * scale).round() as u32;
    window.set_position(PhysicalPosition::new(area_x, area_y))?;
    window.set_size(PhysicalSize::new(width, area_height))?;
    apply_theme_to_window(app, &window);
    apply_acrylic(&window);
    window.show()?;
    window.set_focus()?;
    raise_widget(app);
    Ok(())
}

pub fn close_chat_window(app: &AppHandle) -> bool {
    if let Some(window) = app.get_webview_window("chat") {
        let _ = window.close();
        true
    } else {
        false
    }
}

pub fn hide_widget_manual(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("widget") {
        let _ = window.hide();
    }
    let state = app.state::<AppState>();
    state.widget_visible.store(false, Ordering::SeqCst);
    state.manually_hidden.store(true, Ordering::SeqCst);
}

pub fn hide_widget_auto(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("widget") {
        let _ = window.hide();
    }
    let state = app.state::<AppState>();
    state.widget_visible.store(false, Ordering::SeqCst);
    state.auto_hidden.store(true, Ordering::SeqCst);
}

pub fn toggle_widget(app: &AppHandle) {
    let visible = app
        .get_webview_window("widget")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);
    if visible {
        hide_widget_manual(app);
    } else {
        show_widget(app);
    }
}

pub fn hide_bubble(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("bubble") {
        let _ = window.hide();
    }
    app.state::<AppState>()
        .bubble_visible
        .store(false, Ordering::SeqCst);
}

fn ensure_overlay(
    app: &AppHandle,
    capture: &MonitorCaptureDto,
) -> anyhow::Result<tauri::WebviewWindow> {
    let label = format!("overlay-{}", capture.index);
    if let Some(window) = app.get_webview_window(&label) {
        return Ok(window);
    }

    let window =
        WebviewWindowBuilder::new(app, &label, WebviewUrl::App("overlay.html".into()))
            .title("流萤 · 截图")
            .decorations(false)
            .transparent(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .background_color(Color(5, 15, 20, 255))
            .visible(false)
            .build()?;

    window.set_position(PhysicalPosition::new(capture.x, capture.y))?;
    window.set_size(PhysicalSize::new(capture.width, capture.height))?;
    Ok(window)
}

pub fn precreate_overlays(app: &AppHandle) {
    if let Ok(layout) = crate::capture::monitor_layout() {
        for capture in layout {
            let _ = ensure_overlay(app, &capture);
        }
    }
}

pub fn create_overlays(app: &AppHandle, captures: &[MonitorCaptureDto]) -> anyhow::Result<()> {
    let labels: Vec<String> = captures
        .iter()
        .map(|capture| format!("overlay-{}", capture.index))
        .collect();

    for (label, window) in app.webview_windows() {
        if label.starts_with("overlay-") && !labels.contains(&label) {
            let _ = window.close();
        }
    }

    for capture in captures {
        let window = ensure_overlay(app, capture)?;
        let _ = window.hide();
        window.set_position(PhysicalPosition::new(capture.x, capture.y))?;
        window.set_size(PhysicalSize::new(capture.width, capture.height))?;
        let label = format!("overlay-{}", capture.index);
        let _ = app.emit_to(&label, "overlay-update", capture.clone());
    }

    let fallback = app.clone();
    let fallback_labels = labels.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(2500));
        if !fallback
            .state::<AppState>()
            .session_active
            .load(Ordering::SeqCst)
        {
            return;
        }
        for (label, window) in fallback.webview_windows() {
            if fallback_labels.contains(&label) && !window.is_visible().unwrap_or(false) {
                let _ = window.show();
                let _ = window.set_fullscreen(true);
                let _ = window.set_focus();
            }
        }
    });

    start_escape_watch(app);
    Ok(())
}

pub fn reveal_overlay(window: &tauri::WebviewWindow) {
    let _ = window.show();
    let _ = window.set_fullscreen(true);
    let _ = window.set_focus();
    let app = window.app_handle();
    let captures = app.state::<AppState>().captures.lock().unwrap().clone();
    focus_cursor_monitor(app, &captures);
}

static ESCAPE_WATCH_RUNNING: AtomicBool = AtomicBool::new(false);

fn start_escape_watch(app: &AppHandle) {
    if ESCAPE_WATCH_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let mut was_down = win32::escape_pressed();
        loop {
            let state = app.state::<AppState>();
            let active = state.session_active.load(Ordering::SeqCst);
            let has_overlay = app
                .webview_windows()
                .keys()
                .any(|label| label.starts_with("overlay-"));
            if !active || !has_overlay {
                break;
            }
            let down = win32::escape_pressed();
            if down && !was_down {
                let _ = crate::commands::session_cancel(app.clone());
                break;
            }
            was_down = down;
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        ESCAPE_WATCH_RUNNING.store(false, Ordering::SeqCst);
    });
}

fn focus_cursor_monitor(app: &AppHandle, captures: &[MonitorCaptureDto]) {
    let cursor = win32::cursor_pos();
    let target = captures.iter().find(|capture| {
        cursor.x >= capture.x
            && cursor.x < capture.x + capture.width as i32
            && cursor.y >= capture.y
            && cursor.y < capture.y + capture.height as i32
    });

    let Some(target) = target else {
        return;
    };
    if let Some(window) = app.get_webview_window(&format!("overlay-{}", target.index)) {
        let _ = window.set_focus();
        let webview: &tauri::Webview = window.as_ref();
        let _ = webview.set_focus();
        let delayed = window.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(120));
            let _ = delayed.set_focus();
            let webview: &tauri::Webview = delayed.as_ref();
            let _ = webview.set_focus();
        });
    }
}

pub fn close_overlays(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.closing_overlays.store(true, Ordering::SeqCst);

    let windows: Vec<_> = app
        .webview_windows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("overlay-"))
        .collect();
    for (_label, window) in windows {
        let _ = window.emit("overlay-reset", ());
        let _ = window.hide();
    }

    state.closing_overlays.store(false, Ordering::SeqCst);
}

pub fn position_bubble(app: &AppHandle) -> anyhow::Result<String> {
    let Some(bubble) = app.get_webview_window("bubble") else {
        return Ok("left".to_string());
    };
    let Some(widget) = app.get_webview_window("widget") else {
        return Ok("left".to_string());
    };

    let scale = widget.scale_factor().unwrap_or(1.0);
    let height_css = {
        let state = app.state::<AppState>();
        let value = *state.bubble_height.lock().unwrap();
        value.clamp(60.0, 360.0)
    };

    let width = (360.0 * scale).round() as u32;
    let height = ((height_css + 16.0) * scale).round() as u32;

    let widget_position = widget.outer_position()?;
    let widget_size = widget.outer_size()?;
    let monitor = widget.current_monitor()?.or(widget.primary_monitor()?);

    let Some(monitor) = monitor else {
        return Ok("left".to_string());
    };
    let area = monitor.work_area();
    let area_left = area.position.x;
    let area_top = area.position.y;
    let area_right = area.position.x + area.size.width as i32;
    let area_bottom = area.position.y + area.size.height as i32;

    let right_x = widget_position.x + widget_size.width as i32 + (12.0 * scale) as i32;
    let left_x = widget_position.x - width as i32 - (12.0 * scale) as i32;
    let preferred_x = if right_x + width as i32 <= area_right {
        right_x
    } else {
        left_x
    };
    let x = preferred_x.clamp(area_left + 4, area_right - width as i32 - 4);
    let centered_y = widget_position.y + widget_size.height as i32 / 2 - height as i32 / 2;
    let y = centered_y.clamp(area_top + 4, area_bottom - height as i32 - 4);

    let side = if x >= widget_position.x + widget_size.width as i32 {
        "right"
    } else {
        "left"
    };

    bubble.set_size(PhysicalSize::new(width, height))?;
    bubble.set_position(PhysicalPosition::new(x, y))?;

    let state = app.state::<AppState>();
    *state.bubble_side.lock().unwrap() = side.to_string();
    Ok(side.to_string())
}

pub fn show_bubble(app: &AppHandle, text: &str) -> anyhow::Result<()> {
    if let Some(bubble) = app.get_webview_window("bubble") {
        raise_widget(app);
        let side = position_bubble(app)?;
        app.emit_to(
            "bubble",
            "bubble-text",
            json!({ "text": text, "side": side }),
        )?;
        bubble.show()?;
        let _ = bubble.set_always_on_top(true);
        bring_to_top(&bubble);
        app.state::<AppState>()
            .bubble_visible
            .store(true, Ordering::SeqCst);
    }
    Ok(())
}

pub fn update_bubble_text(app: &AppHandle, text: &str) -> anyhow::Result<()> {
    let Some(bubble) = app.get_webview_window("bubble") else {
        return Ok(());
    };
    let state = app.state::<AppState>();
    if !state.bubble_visible.load(Ordering::SeqCst) {
        let side = position_bubble(app)?;
        app.emit_to(
            "bubble",
            "bubble-text",
            json!({ "text": text, "side": side }),
        )?;
        bubble.show()?;
        let _ = bubble.set_always_on_top(true);
        bring_to_top(&bubble);
        state.bubble_visible.store(true, Ordering::SeqCst);
        return Ok(());
    }

    let side = state.bubble_side.lock().unwrap().clone();
    app.emit_to(
        "bubble",
        "bubble-text",
        json!({ "text": text, "side": side }),
    )?;
    let _ = raise_bubble(app);
    Ok(())
}

pub fn resize_bubble(app: &AppHandle, height: f64) -> anyhow::Result<()> {
    {
        let state = app.state::<AppState>();
        *state.bubble_height.lock().unwrap() = height;
    }
    if app
        .state::<AppState>()
        .bubble_visible
        .load(Ordering::SeqCst)
    {
        position_bubble(app)?;
    }
    Ok(())
}

pub fn open_settings(app: &AppHandle) -> anyhow::Result<()> {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("流萤 · 设置")
        .inner_size(760.0, 700.0)
        .resizable(false)
        .always_on_top(true)
        .decorations(false)
        .shadow(false)
        .transparent(true)
        .no_redirection_bitmap(true)
        .visible(false)
        .build()?;

    let (area_x, area_y, area_width, area_height) = win32::primary_work_area();
    let scale = window.scale_factor().unwrap_or(1.0);
    let width = (760.0 * scale).round() as u32;
    let height = (700.0 * scale).round() as u32;
    window.set_size(PhysicalSize::new(width, height))?;
    window.set_position(PhysicalPosition::new(
        area_x + ((area_width as i32 - width as i32) / 2).max(0),
        area_y + ((area_height as i32 - height as i32) / 2).max(0),
    ))?;
    apply_theme_to_window(app, &window);
    window.show()?;
    window.set_focus()?;
    Ok(())
}
