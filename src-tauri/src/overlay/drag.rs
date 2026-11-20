use crate::settings;
use log::warn;
use tauri::AppHandle;

const OVERLAY_LABEL: &str = "recording_overlay";

pub fn reset_overlay_custom_position(app: &AppHandle) -> Result<(), String> {
    let mut s = settings::load_settings(app);
    s.overlay_custom_position = None;
    let res = settings::save_settings(app, &s);
    crate::overlay::overlay::update_overlay_position(app);
    res
}

fn save_overlay_custom_position(app: &AppHandle, origin: (i32, i32)) {
    let mut s = settings::load_settings(app);
    s.overlay_custom_position = Some(origin);
    if let Err(e) = settings::save_settings(app, &s) {
        warn!("overlay drag: could not save position: {}", e);
    }
}

#[cfg(target_os = "windows")]
pub use windows::start_overlay_drag;

#[cfg(not(target_os = "windows"))]
pub fn start_overlay_drag(app: &AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let window = app
        .get_webview_window(OVERLAY_LABEL)
        .ok_or("overlay window not found")?;
    window.start_dragging().map_err(|e| e.to_string())
}

// The overlay is WS_EX_NOACTIVATE: the native HTCAPTION drag would try to
// activate it and pull focus from the app being dictated into. Following the
// cursor while the button is held keeps focus where it is and tells us when
// the drag ends, so the position is saved once.
#[cfg(target_os = "windows")]
mod windows {
    use super::{save_overlay_custom_position, OVERLAY_LABEL};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;
    use tauri::{AppHandle, Manager, PhysicalPosition, Position};
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetSystemMetrics, SM_SWAPBUTTON,
    };

    static DRAGGING: AtomicBool = AtomicBool::new(false);

    fn cursor_pos() -> Option<(i32, i32)> {
        let mut point = POINT { x: 0, y: 0 };
        match unsafe { GetCursorPos(&mut point) } {
            0 => None,
            _ => Some((point.x, point.y)),
        }
    }

    fn primary_button_down() -> bool {
        let key = match unsafe { GetSystemMetrics(SM_SWAPBUTTON) } {
            0 => VK_LBUTTON,
            _ => VK_RBUTTON,
        };
        let state = unsafe { GetAsyncKeyState(key as i32) };
        state < 0
    }

    pub fn start_overlay_drag(app: &AppHandle) -> Result<(), String> {
        let window = app
            .get_webview_window(OVERLAY_LABEL)
            .ok_or("overlay window not found")?;
        let start = window.outer_position().map_err(|e| e.to_string())?;
        let cursor = cursor_pos().ok_or("cursor position unavailable")?;
        if DRAGGING.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let grab = (cursor.0 - start.x, cursor.1 - start.y);
        let app = app.clone();
        thread::spawn(move || {
            let mut last = (start.x, start.y);
            while primary_button_down() {
                if let Some((cx, cy)) = cursor_pos() {
                    let next = (cx - grab.0, cy - grab.1);
                    if next != last {
                        let _ = window.set_position(Position::Physical(PhysicalPosition {
                            x: next.0,
                            y: next.1,
                        }));
                        last = next;
                    }
                }
                thread::sleep(Duration::from_millis(8));
            }
            DRAGGING.store(false, Ordering::SeqCst);
            if last != (start.x, start.y) {
                save_overlay_custom_position(&app, last);
            }
        });
        Ok(())
    }
}
