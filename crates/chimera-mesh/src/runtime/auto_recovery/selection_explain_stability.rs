use super::*;

pub(crate) fn append_selected_stability_metrics_explain(
    explain: &mut Vec<String>,
    metrics: &SelectedStabilityMetrics,
) {
    append_selected_stability_identity_explain(explain, metrics);
    append_selected_stability_counters_explain(explain, metrics);
}

pub(crate) fn append_selected_stability_identity_explain(
    explain: &mut Vec<String>,
    metrics: &SelectedStabilityMetrics,
) {
    explain.push(format!(
        "selected_peer_stability={}",
        metrics.selected_peer_stability
    ));
    explain.push(format!(
        "selected_effective_replacement_thresholds={}",
        metrics.selected_effective_replacement_thresholds
    ));
    explain.push(format!(
        "selected_replacement_decisions={}",
        metrics.selected_replacement_decisions
    ));
    explain.push(format!(
        "selected_replacement_budget_remaining={}",
        metrics.selected_replacement_budget_remaining
    ));
    explain.push(format!(
        "effective_replacement_threshold_min={}",
        metrics.effective_replacement_threshold_min
    ));
    explain.push(format!(
        "effective_replacement_threshold_max={}",
        metrics.effective_replacement_threshold_max
    ));
}

pub(crate) fn append_selected_stability_counters_explain(
    explain: &mut Vec<String>,
    metrics: &SelectedStabilityMetrics,
) {
    explain.push(format!(
        "stability_updates_total={}",
        metrics.stability_updates_total
    ));
    explain.push(format!(
        "stability_replacements_total={}",
        metrics.stability_replacements_total
    ));
    explain.push(format!("stability_holds_total={}", metrics.stability_holds_total));
    explain.push(format!(
        "stability_degraded_total={}",
        metrics.stability_degraded_total
    ));
    explain.push(format!(
        "stability_churn_blocks_total={}",
        metrics.stability_churn_blocks_total
    ));
    explain.push(format!(
        "stability_threshold_blocks_total={}",
        metrics.stability_threshold_blocks_total
    ));
    explain.push(format!(
        "replacement_hold_ratio_pct={}",
        metrics.replacement_hold_ratio_pct
    ));
    explain.push(format!(
        "replacement_budget_remaining_total={}",
        metrics.replacement_budget_remaining_total
    ));
}
