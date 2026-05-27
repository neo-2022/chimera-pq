use super::common::{
    STANDBY_EXPLAIN_KEYS, StandbyShadowDeriveInput, derive_standby_shadow_fields, explain_value,
    remove_explain_keys, selected_peer_ids_from_explain,
};
use super::standby_shadow::{
    resolve_mode_from_action, standby_ready_flags, standby_stage_source,
    standby_target_for_multipath_mode,
};
pub(super) fn adapt_standby_shadow_from_dps(explain: &mut Vec<String>) {
    let action = explain_value(explain, "preemptive_shadow_action=")
        .unwrap_or("hold")
        .to_string();
    let should_prepare =
        explain_value(explain, "preemptive_shadow_switch_prepare=") == Some("true");
    let should_switch =
        explain_value(explain, "preemptive_shadow_switch_recommend=") == Some("true");
    let switch_target = explain_value(explain, "preemptive_shadow_switch_target=")
        .unwrap_or("none")
        .to_string();
    let multipath_mode = explain_value(explain, "dps_payload_multipath_mode=");
    let selected_peer_ids = selected_peer_ids_from_explain(explain);
    let (standby_target, standby_target_source) =
        standby_target_for_multipath_mode(multipath_mode, &switch_target, &selected_peer_ids);
    let standby_mode = resolve_mode_from_action(&action);
    let stage = explain_value(explain, "preemptive_shadow_stage=")
        .unwrap_or("clear")
        .to_string();
    let trigger = explain_value(explain, "preemptive_shadow_trigger=")
        .unwrap_or("none")
        .to_string();
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
    let (warm_ready, hot_ready) =
        standby_ready_flags(Some(stage.as_str()), standby_mode, &standby_target);
    let stage_source = standby_stage_source(stage.as_str(), trigger.as_str());
    let derived = derive_standby_shadow_fields(StandbyShadowDeriveInput {
        mode: standby_mode,
        target: &standby_target,
        target_source: standby_target_source,
        reason: standby_reason,
        source: standby_source,
        warm_ready,
        hot_ready,
        stage_source: &stage_source,
    });
    remove_explain_keys(explain, STANDBY_EXPLAIN_KEYS);
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
