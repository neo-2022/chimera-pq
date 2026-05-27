use super::preemptive_antiflap::apply_preemptive_antiflap;
use super::*;

pub(super) fn append_plan_path_preemptive_shadow_explain(
    explain: &mut Vec<String>,
    context: &PlanPathPreemptiveContext<'_>,
    pri_tuning: &ShadowPriTuning,
) {
    let unhealthy_nodes = runtime_unhealthy_nodes(context.health_state);
    let health_blocked_count = unhealthy_nodes.len();
    let mut shadow = evaluate_shadow_runtime_decision(
        context.path_profile,
        context.avg_load_score,
        context.avg_reliability_score,
        health_blocked_count,
        context.peers,
        &unhealthy_nodes,
        pri_tuning,
    );
    let anti_flap = apply_preemptive_antiflap(
        &mut shadow,
        context.peer_meta,
        context.tick,
        context.table_policy,
    );
    let summaries = shadow_summary_bundle(&shadow);
    append_preemptive_shadow_explain(
        explain,
        &shadow,
        &summaries,
        pri_tuning,
        context.path_profile,
    );
    explain.push(format!(
        "preemptive_shadow_antiflap_blocked={}",
        anti_flap.blocked
    ));
    explain.push(format!(
        "preemptive_shadow_antiflap_reason={}",
        anti_flap.reason
    ));
    explain.push(format!(
        "preemptive_shadow_antiflap_replacements_window={}",
        anti_flap.replacements_window
    ));
    explain.push(format!(
        "preemptive_shadow_antiflap_replacements_limit={}",
        anti_flap.replacements_limit
    ));
}
