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

#[derive(Default)]
pub(super) struct StandbyShadowExplainSnapshot {
    pub(super) action: Option<String>,
    pub(super) switch_target: Option<String>,
    pub(super) should_prepare: bool,
    pub(super) should_switch: bool,
    pub(super) stage: Option<String>,
    pub(super) trigger: Option<String>,
}

impl StandbyShadowExplainSnapshot {
    pub(super) fn capture(explain: &[String]) -> Self {
        let mut snapshot = Self::default();
        for line in explain {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "preemptive_shadow_action" => Self::assign(&mut snapshot.action, value),
                "preemptive_shadow_switch_target" => {
                    Self::assign(&mut snapshot.switch_target, value)
                }
                "preemptive_shadow_switch_prepare" => {
                    snapshot.should_prepare = value == "true";
                }
                "preemptive_shadow_switch_recommend" => {
                    snapshot.should_switch = value == "true";
                }
                "preemptive_shadow_stage" => Self::assign(&mut snapshot.stage, value),
                "preemptive_shadow_trigger" => Self::assign(&mut snapshot.trigger, value),
                _ => {}
            }
        }
        snapshot
    }

    #[inline]
    fn assign(slot: &mut Option<String>, value: &str) {
        if slot.is_none() {
            *slot = Some(value.to_owned());
        }
    }
}

pub(super) fn redacted_standby_target(target: &str, selected_peers: &[MeshPeerState]) -> String {
    if target == "none" {
        return "none".to_string();
    }
    if is_public_peer_label(target, selected_peers.len()) {
        return target.to_string();
    }
    selected_peers
        .iter()
        .position(|peer| peer.node_id == target)
        .map(|index| format!("peer#{}", index + 1))
        .unwrap_or_else(|| "<redacted>".to_string())
}

pub(super) fn remove_and_redact_explain_keys(
    explain: &mut Vec<String>,
    keys: &[&str],
    selected_peers: &[MeshPeerState],
) {
    let mut redacted_switch_target = false;
    explain.retain_mut(|line| {
        if !redacted_switch_target
            && let Some(target) = line.strip_prefix("preemptive_shadow_switch_target=")
        {
            *line = format!(
                "preemptive_shadow_switch_target={}",
                redacted_standby_target(target, selected_peers)
            );
            redacted_switch_target = true;
        }
        !keys.iter().any(|key| line.starts_with(key))
    });
}

pub(super) fn redact_preemptive_switch_target(
    explain: &mut [String],
    selected_peers: &[MeshPeerState],
) {
    for line in explain.iter_mut() {
        let Some(target) = line.strip_prefix("preemptive_shadow_switch_target=") else {
            continue;
        };
        *line = format!(
            "preemptive_shadow_switch_target={}",
            redacted_standby_target(target, selected_peers)
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
