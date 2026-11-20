use crate::appearance::{apply_glass_style, GLASS_STYLES};
use crate::settings;
use tauri::{command, AppHandle, Emitter};

#[command]
pub fn set_glass_style(app: AppHandle, style: String) -> Result<(), String> {
    if !GLASS_STYLES.contains(&style.as_str()) {
        return Err("Invalid glass style".to_string());
    }
    let mut s = settings::load_settings(&app);
    s.glass_style = style.clone();
    let res = settings::save_settings(&app, &s);
    apply_glass_style(&app, &style);
    let _ = app.emit("glass-style-changed", &style);
    res
}
