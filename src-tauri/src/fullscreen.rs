use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::state::AppState;
use crate::windows;

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last: Option<bool> = None;
        loop {
            std::thread::sleep(Duration::from_millis(500));

            let state = app.state::<AppState>();
            if state.session_active.load(Ordering::SeqCst) {
                last = None;
                continue;
            }

            let fullscreen = crate::win32::is_foreground_fullscreen();
            if last == Some(fullscreen) {
                continue;
            }
            last = Some(fullscreen);

            if fullscreen {
                windows::hide_widget_auto(&app);
                windows::hide_bubble(&app);
            } else if !state.manually_hidden.load(Ordering::SeqCst) {
                windows::show_widget_no_flag(&app);
            }
        }
    });
}
