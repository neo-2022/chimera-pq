use crate::policy::traffic_hints_from_dps_payload;

pub(crate) fn hints_reason_from_presence(has_any_hint: bool) -> &'static str {
    if has_any_hint {
        "dps_payload_parsed"
    } else {
        "dps_payload_no_hints"
    }
}

pub(crate) fn shadow_hints_meta_from_payload(
    payload: Option<&str>,
) -> (String, String, String, bool, String, String) {
    let Some(payload) = payload else {
        return (
            "unknown".to_string(),
            "unknown".to_string(),
            "no_payload_context".to_string(),
            false,
            "unknown".to_string(),
            "unknown".to_string(),
        );
    };
    match traffic_hints_from_dps_payload(payload) {
        Ok(hints) => {
            let reason = hints_reason_from_presence(hints.has_any_hint());
            (
                hints.shadow_switch_mode.as_str().to_string(),
                "ok".to_string(),
                reason.to_string(),
                hints.has_any_hint(),
                hints
                    .multipath_mode
                    .map(|value| value.as_str().to_string())
                    .unwrap_or_else(|| "none".to_string()),
                hints
                    .continuity_policy
                    .map(|value| value.as_str().to_string())
                    .unwrap_or_else(|| "none".to_string()),
            )
        }
        Err(_) => (
            "unknown".to_string(),
            "invalid".to_string(),
            "dps_payload_invalid".to_string(),
            false,
            "invalid".to_string(),
            "invalid".to_string(),
        ),
    }
}

pub(crate) fn hints_source_from_status(status: &str) -> &'static str {
    match status {
        "unknown" => "none",
        "invalid" => "invalid_payload",
        _ => "dps_payload",
    }
}

pub(crate) fn format_hints_summary(
    status: &str,
    present: bool,
    reason: &str,
    multipath_mode: &str,
    continuity_policy: &str,
) -> String {
    format!(
        "status={status};present={present};reason={reason};multipath_mode={multipath_mode};continuity_policy={continuity_policy}"
    )
}

pub(crate) fn format_hints_summary_with_source(
    status: &str,
    present: bool,
    reason: &str,
    multipath_mode: &str,
    continuity_policy: &str,
) -> String {
    let summary = format_hints_summary(status, present, reason, multipath_mode, continuity_policy);
    let source = hints_source_from_status(status);
    format!("{summary};source={source}")
}
