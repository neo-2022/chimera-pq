use super::*;

pub(crate) fn append_effective_filter_explain(
    explain: &mut Vec<String>,
    context: &EffectiveFilterExplainContext,
    pri_tuning: &ShadowPriTuning,
) {
    explain.push(format!(
        "effective_min_reliability={}",
        context.effective_reliability
    ));
    explain.push(format!("effective_max_load={}", context.effective_load));
    explain.push(format!(
        "effective_max_peers={}",
        context.effective_max_peers
    ));
    explain.push(format!(
        "effective_min_distinct_regions={}",
        context.effective_min_distinct_regions
    ));
    explain.push(format!(
        "effective_prefer_region_diversity={}",
        context.effective_prefer_region_diversity
    ));
    explain.push(format!(
        "effective_max_selected_per_region={}",
        context.effective_max_selected_per_region
    ));
    explain.push(format!(
        "effective_filter_source={}",
        if context.auto_mode {
            "auto_profile"
        } else {
            "manual_override"
        }
    ));
    explain.push(format!(
        "preemptive_shadow_tuning_confirmation={}",
        format_confirmation_tuning(pri_tuning)
    ));
    explain.push(format!(
        "effective_health_filter_source={}",
        if context.auto_mode {
            "auto"
        } else {
            "manual_disabled"
        }
    ));
}

pub(crate) fn append_auto_recovery_summary_explain(
    explain: &mut Vec<String>,
    summary: &AutoRecoverySummaryContext<'_>,
    auto_recovery_trace: &[&str],
) {
    explain.push(format!(
        "effective_health_relax_applied={}",
        summary.health_relax_applied
    ));
    explain.push(format!(
        "effective_health_relax_reason={}",
        summary.health_relax_reason
    ));
    explain.push(format!(
        "effective_health_relax_stage={}",
        summary.health_relax_stage
    ));
    explain.push(format!(
        "auto_recovery_attempts={}",
        summary.auto_recovery_attempts
    ));
    explain.push(format!(
        "auto_recovery_triggered={}",
        summary.auto_recovery_attempts > 0
    ));
    explain.push(format!(
        "auto_recovery_final_result={}",
        summary.auto_recovery_final_result
    ));
    explain.push(format!(
        "auto_recovery_trace_steps={}",
        auto_recovery_trace.len()
    ));
    let auto_recovery_trace_consistent =
        auto_recovery_trace.len() == summary.auto_recovery_attempts * 2;
    explain.push(format!(
        "auto_recovery_trace_consistent={auto_recovery_trace_consistent}"
    ));
    explain.push(format!(
        "auto_recovery_trace={}",
        if auto_recovery_trace.is_empty() {
            "none".to_string()
        } else {
            auto_recovery_trace.join("->")
        }
    ));
}
