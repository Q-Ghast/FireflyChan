use std::time::Duration;

use tauri::{AppHandle, Manager};

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut ignored = true;

        loop {
            std::thread::sleep(Duration::from_millis(30));

            let Some(window) = app.get_webview_window("widget") else {
                continue;
            };

            if !window.is_visible().unwrap_or(false) {
                if !ignored {
                    let _ = window.set_ignore_cursor_events(true);
                    ignored = true;
                }
                continue;
            }

            let (Ok(position), Ok(size)) =
                (window.outer_position(), window.outer_size())
            else {
                continue;
            };

            let cursor = crate::win32::cursor_pos();
            let center_x = position.x + size.width as i32 / 2;
            let center_y = position.y + size.height as i32 / 2;
            let radius = size.width.min(size.height) as f64 * 0.42;
            let dx = (cursor.x - center_x) as f64;
            let dy = (cursor.y - center_y) as f64;
            let distance = (dx * dx + dy * dy).sqrt();

            let inside = if ignored {
                distance <= radius - 6.0
            } else {
                distance <= radius + 6.0
            };

            if inside == ignored {
                let _ = window.set_ignore_cursor_events(!inside);
                ignored = !inside;
            }
        }
    });
}
