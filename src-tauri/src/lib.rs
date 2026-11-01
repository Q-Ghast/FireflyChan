mod ai;
mod capture;
mod commands;
mod config;
mod conversation;
mod fullscreen;
mod hit_test;
mod search;
mod state;
mod win32;
mod windows;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};

fn trigger_capture(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = commands::start_capture(app).await {
            log::error!("capture failed: {error}");
        }
    });
}

fn open_chat(app: &AppHandle) {
    if let Err(error) = windows::open_chat(app) {
        log::error!("open chat failed: {error}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::show_widget(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if shortcut.matches(Modifiers::ALT, Code::KeyC) {
                            open_chat(app);
                        } else {
                            trigger_capture(app);
                        }
                    }
                })
                .build(),
        )
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            app.manage(state::AppState::load(app.handle()));

            windows::create_widget(app.handle())?;
            windows::create_bubble(app.handle())?;
            windows::precreate_overlays(app.handle());
            windows::create_tray(app.handle())?;

            if let Err(error) = app.global_shortcut().register("Alt+Q") {
                log::error!("register Alt+Q failed: {error}");
            }
            if let Err(error) = app.global_shortcut().register("Alt+C") {
                log::error!("register Alt+C failed: {error}");
            }

            let theme = app.state::<state::AppState>().config.lock().unwrap().theme;
            windows::apply_theme(app.handle(), theme);

            if config::get_api_key().is_none() {
                let _ = windows::open_settings(app.handle());
            }

            fullscreen::start(app.handle().clone());
            hit_test::start(app.handle().clone());
            Ok(())
        })
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "capture" => trigger_capture(app),
                "open_chat" => open_chat(app),
                "toggle_widget" => windows::toggle_widget(app),
                "show_widget" => windows::show_widget(app),
                "hide_widget" => windows::hide_widget_manual(app),
                "settings" => {
                    if let Err(error) = windows::open_settings(app) {
                        log::error!("open settings failed: {error}");
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .on_window_event(|window, event| {
            if window.label() == "widget" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    windows::hide_widget_manual(&window.app_handle());
                }
            }

            if window.label().starts_with("overlay-") {
                if let tauri::WindowEvent::Focused(true) = event {
                    let app = window.app_handle().clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        windows::raise_companions(&app);
                    });
                }
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    let app = window.app_handle();
                    let state = app.state::<state::AppState>();
                    if !state.closing_overlays.load(std::sync::atomic::Ordering::SeqCst)
                        && state.session_active.load(std::sync::atomic::Ordering::SeqCst)
                    {
                        let _ = commands::session_cancel(app.clone());
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::session_start,
            commands::session_confirm,
            commands::session_cancel,
            commands::overlay_info,
            commands::overlay_ready,
            commands::raise_companions,
            commands::conversation_get,
            commands::conversation_new,
            commands::staged_clear,
            commands::chat_send,
            commands::chat_abort,
            commands::config_get,
            commands::config_save,
            commands::config_test,
            commands::theme_set,
            commands::quick_prompts_get,
            commands::open_chat,
            commands::widget_menu,
            commands::widget_drag,
            commands::widget_position_save,
            commands::web_search_get,
            commands::web_search_set,
            commands::widget_show,
            commands::widget_hide,
            commands::bubble_show,
            commands::bubble_hide,
            commands::bubble_resize,
            commands::fullscreen_test,
            commands::window_drag,
            commands::open_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running FireflyChan");
}
