pub(crate) fn resolve_mode_from_action(action: &str) -> &'static str {
    match action {
        "recommend_switch" => "switch_ready",
        "keep_hot_standby" => "hot",
        "prepare_standby" => "warm",
        _ => "off",
    }
}

pub(crate) fn standby_ready_flags(stage: Option<&str>, mode: &str, target: &str) -> (bool, bool) {
    if target == "none" || mode == "off" {
        return (false, false);
    }
    match stage.unwrap_or("clear") {
        "switch" | "hard" => (true, true),
        "hot_standby" => (true, true),
        "prepare" => (true, false),
        _ => (false, false),
    }
}

pub(crate) fn standby_stage_source(stage: &str, trigger: &str) -> String {
    format!("stage:{stage};trigger:{trigger}")
}
