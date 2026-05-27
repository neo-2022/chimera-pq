use super::*;
use crate::preemptive::ShadowPriTuning;

pub(super) struct PreemptiveShadowExplainContext<'a> {
    pub(super) profile: MeshPathProfile,
    pub(super) avg_load_score: u8,
    pub(super) avg_reliability_score: u8,
    pub(super) peers: &'a BTreeMap<String, MeshPeerState>,
    pub(super) unhealthy_nodes: &'a BTreeSet<String>,
    pub(super) health_state_count: usize,
    pub(super) table_policy: &'a MeshPeerTablePolicy,
    pub(super) peer_meta: &'a BTreeMap<String, MeshPeerMeta>,
    pub(super) tick: u64,
}

pub(super) struct PreemptiveShadowEval {
    pub(super) shadow: crate::preemptive::ShadowRuntimeDecision,
    pub(super) tuning: ShadowPriTuning,
    pub(super) replacements_window: u64,
    pub(super) confirm_ratio: f32,
    pub(super) confirm_missing_signals: u8,
    pub(super) switch_target: String,
    pub(super) switch_candidate_sample_age_ticks: Option<u64>,
    pub(super) switch_confidence_gate_min: f64,
    pub(super) switch_confidence_gate_passed: bool,
    pub(super) health_blocked_count: usize,
}

pub(super) fn evaluate_preemptive_shadow(
    ctx: &PreemptiveShadowExplainContext<'_>,
) -> PreemptiveShadowEval {
    let tuning = shadow_pri_tuning_from_env();
    let mut shadow = evaluate_shadow_runtime_decision(
        ctx.profile,
        ctx.avg_load_score,
        ctx.avg_reliability_score,
        ctx.unhealthy_nodes.len(),
        ctx.peers,
        ctx.unhealthy_nodes,
        &tuning,
    );
    let replacements_window = ctx
        .peer_meta
        .values()
        .map(|meta| {
            let age_since_seen = ctx.tick.saturating_sub(meta.last_seen_tick);
            if age_since_seen <= ctx.table_policy.stability_window_ticks {
                meta.replacement_events
            } else {
                0
            }
        })
        .max()
        .unwrap_or(0);
    if shadow.switch.should_switch
        && replacements_window >= ctx.table_policy.max_replacements_per_window
    {
        shadow.switch.should_switch = false;
        shadow.switch.reason = "switch_budget_exceeded".to_string();
        shadow.action = crate::preemptive::ShadowAction::KeepHotStandby;
        shadow.action_reason = "switch_budget_exceeded";
    }

    let confirm_ratio = if shadow.confirmation.confirm_m == 0 {
        0.0
    } else {
        f32::from(shadow.confirmation.signal_hits) / f32::from(shadow.confirmation.confirm_m)
    };
    let confirm_missing_signals = shadow
        .confirmation
        .confirm_n
        .saturating_sub(shadow.confirmation.signal_hits);
    let switch_target = shadow
        .switch
        .target_peer
        .clone()
        .unwrap_or_else(|| "none".to_string());
    let switch_candidate_sample_age_ticks = shadow
        .switch
        .target_peer
        .as_ref()
        .and_then(|target| ctx.peer_meta.get(target))
        .map(|meta| ctx.tick.saturating_sub(meta.last_seen_tick));
    let switch_confidence_gate_min =
        crate::preemptive::profile_min_switch_conf(&tuning, ctx.profile);
    let switch_confidence_gate_passed = shadow.switch.confidence >= switch_confidence_gate_min;
    let health_blocked_count = ctx.unhealthy_nodes.len().min(ctx.health_state_count);

    PreemptiveShadowEval {
        shadow,
        tuning,
        replacements_window,
        confirm_ratio,
        confirm_missing_signals,
        switch_target,
        switch_candidate_sample_age_ticks,
        switch_confidence_gate_min,
        switch_confidence_gate_passed,
        health_blocked_count,
    }
}
