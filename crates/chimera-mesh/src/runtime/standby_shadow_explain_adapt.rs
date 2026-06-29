use super::MeshPeerState;
use super::common::{
    STANDBY_EXPLAIN_KEYS, StandbyShadowDeriveInput, StandbyShadowExplainSnapshot,
    derive_standby_shadow_fields, redacted_standby_target, remove_and_redact_explain_keys,
};
use super::standby_shadow::{
    resolve_mode_from_action, standby_ready_flags, standby_stage_source,
    standby_target_for_multipath_mode,
};
pub(super) fn adapt_standby_shadow_from_dps(
    selected_peers: &[MeshPeerState],
    explain: &mut Vec<String>,
    snapshot: &StandbyShadowExplainSnapshot,
    dps_multipath_mode: Option<&str>,
) {
    let action = snapshot.action.as_deref().unwrap_or("hold");
    let should_prepare = snapshot.should_prepare;
    let should_switch = snapshot.should_switch;
    let switch_target = snapshot.switch_target.as_deref().unwrap_or("none");
    let multipath_mode = dps_multipath_mode;
    let (standby_target, standby_target_source) =
        standby_target_for_multipath_mode(multipath_mode, switch_target, selected_peers);
    let public_standby_target = redacted_standby_target(&standby_target, selected_peers);
    let standby_mode = resolve_mode_from_action(action);
    let stage = snapshot.stage.as_deref().unwrap_or("clear");
    let trigger = snapshot.trigger.as_deref().unwrap_or("none");
    let standby_reason = if standby_target == "none" {
        "no_candidate"
    } else if should_switch {
        "switch_recommended"
    } else if should_prepare {
        "prepare_threshold"
    } else {
        "no_action"
    };
    let standby_source = if multipath_mode.is_some() {
        "dps_multipath_policy"
    } else {
        "preemptive_shadow"
    };
    let (warm_ready, hot_ready) = standby_ready_flags(Some(stage), standby_mode, &standby_target);
    let stage_source = standby_stage_source(stage, trigger);
    let derived = derive_standby_shadow_fields(StandbyShadowDeriveInput {
        mode: standby_mode,
        target: &public_standby_target,
        target_source: standby_target_source,
        reason: standby_reason,
        source: standby_source,
        warm_ready,
        hot_ready,
        stage_source: &stage_source,
    });
    remove_and_redact_explain_keys(explain, STANDBY_EXPLAIN_KEYS, selected_peers);
    explain.push(format!("standby_shadow_mode={}", derived.mode));
    explain.push(format!("standby_shadow_target={}", derived.target));
    explain.push(format!(
        "standby_shadow_target_source={}",
        derived.target_source
    ));
    explain.push(format!("standby_shadow_reason={}", derived.reason));
    explain.push(format!("standby_shadow_source={}", derived.source));
    explain.push(format!("standby_shadow_warm_ready={}", derived.warm_ready));
    explain.push(format!("standby_shadow_hot_ready={}", derived.hot_ready));
    explain.push(format!(
        "standby_shadow_stage_source={}",
        derived.stage_source
    ));
    explain.push(format!("standby_shadow_summary={}", derived.summary));
}
