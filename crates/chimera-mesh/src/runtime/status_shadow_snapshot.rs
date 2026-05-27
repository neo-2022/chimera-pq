use super::preemptive_antiflap::ShadowAntiFlapGuard;
use super::preemptive_helpers::{
    shadow_hints_meta_from_payload, shadow_switch_guard_meta, tuning_source_label,
};
use super::standby_shadow::{StandbyShadowStatus, build_standby_shadow_status};
use super::*;

pub(super) struct ShadowStatusSnapshot {
    pub(super) shadow: crate::preemptive::ShadowRuntimeDecision,
    pub(super) health_blocked_count: usize,
    pub(super) antiflap: ShadowAntiFlapGuard,
    pub(super) confirm_ratio: f32,
    pub(super) confirm_missing_signals: u8,
    pub(super) switch_guard: String,
    pub(super) switch_guard_source: String,
    pub(super) switch_target: String,
    pub(super) switch_candidate_sample_age_ticks: Option<u64>,
    pub(super) switch_confidence_gate_min: f64,
    pub(super) switch_confidence_gate_passed: bool,
    pub(super) switch_mode: String,
    pub(super) hints_status: String,
    pub(super) hints_reason: String,
    pub(super) hints_present: bool,
    pub(super) hints_multipath_mode: String,
    pub(super) hints_continuity_policy: String,
    pub(super) standby: StandbyShadowStatus,
    pub(super) tuning_source: String,
    pub(super) tuning_confirmation: String,
    pub(super) tuning_weights: String,
    pub(super) tuning_thresholds: String,
}

pub(super) fn build_shadow_status_snapshot(
    runtime: &MeshRuntime,
    payload: Option<&str>,
) -> ShadowStatusSnapshot {
    let (avg_load_score, avg_reliability_score) = runtime_peer_signal_averages(&runtime.peers);
    let unhealthy_nodes = runtime.unhealthy_node_ids();
    let health_blocked_count = unhealthy_nodes.len();
    let tuning = shadow_pri_tuning_from_env();
    let mut shadow = evaluate_shadow_runtime_decision(
        runtime.profile_state.active_profile,
        avg_load_score,
        avg_reliability_score,
        health_blocked_count,
        &runtime.peers,
        &unhealthy_nodes,
        &tuning,
    );
    let antiflap = runtime.apply_shadow_antiflap_guard(&mut shadow);
    let confirm_ratio = if shadow.confirmation.confirm_m == 0 {
        0.0
    } else {
        f32::from(shadow.confirmation.signal_hits) / f32::from(shadow.confirmation.confirm_m)
    };
    let confirm_missing_signals = shadow
        .confirmation
        .confirm_n
        .saturating_sub(shadow.confirmation.signal_hits);
    let (switch_guard, switch_guard_source) =
        shadow_switch_guard_meta(shadow.switch.reason.as_str());
    let switch_target = shadow
        .switch
        .target_peer
        .clone()
        .unwrap_or_else(|| "none".to_string());
    let switch_candidate_sample_age_ticks = shadow
        .switch
        .target_peer
        .as_ref()
        .and_then(|target| runtime.peer_meta.get(target))
        .map(|meta| runtime.tick.saturating_sub(meta.last_seen_tick));
    let switch_confidence_gate_min =
        crate::preemptive::profile_min_switch_conf(&tuning, runtime.profile_state.active_profile);
    let switch_confidence_gate_passed = shadow.switch.confidence >= switch_confidence_gate_min;
    let (
        switch_mode,
        hints_status,
        hints_reason,
        hints_present,
        hints_multipath_mode,
        hints_continuity_policy,
    ) = shadow_hints_meta_from_payload(payload);
    let standby = build_standby_shadow_status(
        shadow.report.stage,
        shadow.report.trigger,
        format_shadow_action(shadow.action),
        &switch_target,
        shadow.switch.should_prepare,
        shadow.switch.should_switch,
        &switch_mode,
    );

    ShadowStatusSnapshot {
        shadow,
        health_blocked_count,
        antiflap,
        confirm_ratio,
        confirm_missing_signals,
        switch_guard: switch_guard.to_string(),
        switch_guard_source: switch_guard_source.to_string(),
        switch_target,
        switch_candidate_sample_age_ticks,
        switch_confidence_gate_min,
        switch_confidence_gate_passed,
        switch_mode,
        hints_status,
        hints_reason,
        hints_present,
        hints_multipath_mode,
        hints_continuity_policy,
        standby,
        tuning_source: tuning_source_label(tuning.source).to_string(),
        tuning_confirmation: format_confirmation_tuning(&tuning),
        tuning_weights: format_profile_tuning_weights(
            &tuning,
            runtime.profile_state.active_profile,
        ),
        tuning_thresholds: format_profile_tuning_thresholds(
            &tuning,
            runtime.profile_state.active_profile,
        ),
    }
}
