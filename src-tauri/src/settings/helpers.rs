pub fn normalize_result_panel_mode(mode: &str) -> &'static str {
    match mode {
        "off" => "off",
        "command_llm" => "command_llm",
        "all" => "all",
        // "command" belonged to the removed Command Mode; it showed nothing else.
        _ => "off",
    }
}

pub fn migrate_legacy_paste_method(settings: &mut serde_json::Value) {
    let Some(object) = settings.as_object_mut() else {
        return;
    };
    if matches!(
        object
            .get("paste_method")
            .and_then(serde_json::Value::as_str),
        Some("none" | "None")
    ) {
        object.insert("paste_method".to_string(), "ctrl_v".into());
        object.insert("auto_insert".to_string(), false.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_result_panel_modes_are_kept() {
        for mode in ["off", "command_llm", "all"] {
            assert_eq!(normalize_result_panel_mode(mode), mode);
        }
    }

    #[test]
    fn unknown_result_panel_modes_fall_back_to_off() {
        for mode in ["command", "commands", "", "Commands"] {
            assert_eq!(normalize_result_panel_mode(mode), "off");
        }
    }
}
