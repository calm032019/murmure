use log::warn;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{AppHandle, Manager};

pub const GLASS_STYLES: [&str; 2] = ["clear", "frosted"];

// Frosted glass lets Windows blur the desktop behind the main window
// (acrylic); clear glass drops the effect so the desktop shows through as-is.
// The overlay never gets a window effect: it would frost its whole bounding
// box, so it draws its own glass.
pub fn apply_glass_style(app: &AppHandle, style: &str) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let result = match style {
        "frosted" => window.set_effects(EffectsBuilder::new().effect(Effect::Acrylic).build()),
        _ => window.set_effects(None),
    };
    if let Err(e) = result {
        warn!("Could not apply {} glass style: {}", style, e);
    }
}
