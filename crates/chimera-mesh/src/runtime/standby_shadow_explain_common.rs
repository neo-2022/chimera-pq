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
use crate::model::MeshPeerState;

pub(super) fn explain_value<'a>(explain: &'a [String], prefix: &str) -> Option<&'a str> {
    explain.iter().find_map(|line| line.strip_prefix(prefix))
}

pub(super) fn remove_explain_keys(explain: &mut Vec<String>, keys: &[&str]) {
    explain.retain(|line| !keys.iter().any(|key| line.starts_with(key)));
}

pub(super) fn selected_peer_ids(selected_peers: &[MeshPeerState]) -> Vec<String> {
    selected_peers
        .iter()
        .map(|peer| peer.node_id.clone())
        .collect()
}

pub(super) fn redacted_standby_target(target: &str, selected_peer_ids: &[String]) -> String {
    if target == "none" {
        return "none".to_string();
    }
    if is_public_peer_label(target, selected_peer_ids.len()) {
        return target.to_string();
    }
    selected_peer_ids
        .iter()
        .position(|peer_id| peer_id == target)
        .map(|index| format!("peer#{}", index + 1))
        .unwrap_or_else(|| "<redacted>".to_string())
}

pub(super) fn redact_preemptive_switch_target(
    explain: &mut [String],
    selected_peer_ids: &[String],
) {
    for line in explain.iter_mut() {
        let Some(target) = line.strip_prefix("preemptive_shadow_switch_target=") else {
            continue;
        };
        *line = format!(
            "preemptive_shadow_switch_target={}",
            redacted_standby_target(target, selected_peer_ids)
        );
        break;
    }
}

fn is_public_peer_label(target: &str, selected_peer_count: usize) -> bool {
    target
        .strip_prefix("peer#")
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|index| index > 0 && index <= selected_peer_count)
}
