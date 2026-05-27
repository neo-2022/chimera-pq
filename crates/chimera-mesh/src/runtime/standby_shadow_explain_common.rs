pub(super) const STANDBY_EXPLAIN_KEYS: &[&str] = &[
    "standby_shadow_mode=",
    "standby_shadow_target=",
    "standby_shadow_target_source=",
    "standby_shadow_reason=",
    "standby_shadow_source=",
    "standby_shadow_warm_ready=",
    "standby_shadow_hot_ready=",
    "standby_shadow_stage_source=",
    "standby_shadow_summary=",
];
pub(super) use super::standby_shadow::{StandbyShadowDeriveInput, derive_standby_shadow_fields};

pub(super) fn explain_value<'a>(explain: &'a [String], prefix: &str) -> Option<&'a str> {
    explain.iter().find_map(|line| line.strip_prefix(prefix))
}

pub(super) fn remove_explain_keys(explain: &mut Vec<String>, keys: &[&str]) {
    explain.retain(|line| !keys.iter().any(|key| line.starts_with(key)));
}

pub(super) fn selected_peer_ids_from_explain(explain: &[String]) -> Vec<String> {
    let Some(value) = explain_value(explain, "selected_peer_ids=") else {
        return Vec::new();
    };
    value
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .collect()
}
