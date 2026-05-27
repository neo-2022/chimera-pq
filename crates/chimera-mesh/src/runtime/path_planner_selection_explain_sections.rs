use super::path_planner_selection_explain::SelectionExplainInput;
use super::path_planner_selection_metrics::SelectionMetrics;

pub(super) fn append_selected_peer_lines(explain: &mut Vec<String>, metrics: &SelectionMetrics) {
    explain.push(format!("selected_peer_ids={}", metrics.selected_peer_ids));
    explain.push(format!(
        "selected_peer_regions={}",
        metrics.selected_peer_regions
    ));
    explain.push(format!(
        "selected_peer_endpoints={}",
        metrics.selected_peer_endpoints
    ));
    explain.push(format!(
        "selected_peer_connect_priority={}",
        metrics.selected_peer_connect_priority
    ));
    explain.push(format!(
        "selected_peer_connect_retry_plan={}",
        metrics.selected_peer_connect_retry_plan
    ));
    explain.push(format!(
        "selected_peer_connect_backoff_profile={}",
        metrics.selected_peer_connect_backoff_profile
    ));
    explain.push(format!(
        "selected_peer_scores={}",
        metrics.selected_peer_scores
    ));
    explain.push(format!("selected_score_sum={}", metrics.selected_score_sum));
    explain.push(format!(
        "selected_reliability_avg={}",
        metrics.selected_reliability_avg
    ));
    explain.push(format!("selected_load_avg={}", metrics.selected_load_avg));
    explain.push(format!(
        "selected_region_counts={}",
        metrics.selected_region_counts
    ));
}

pub(super) fn append_stability_lines(explain: &mut Vec<String>, metrics: &SelectionMetrics) {
    explain.push(format!(
        "selected_peer_stability={}",
        metrics.selected_stability
    ));
    explain.push(format!(
        "selected_effective_replacement_thresholds={}",
        metrics.selected_effective_thresholds
    ));
    explain.push(format!(
        "selected_replacement_decisions={}",
        metrics.selected_replacement_decisions
    ));
    explain.push(format!(
        "selected_replacement_budget_remaining={}",
        metrics.selected_replacement_budget
    ));
    explain.push(format!(
        "effective_replacement_threshold_min={}",
        metrics.effective_threshold_min
    ));
    explain.push(format!(
        "effective_replacement_threshold_max={}",
        metrics.effective_threshold_max
    ));
    explain.push(format!(
        "stability_updates_total={}",
        metrics.stability_updates_total
    ));
    explain.push(format!(
        "stability_replacements_total={}",
        metrics.stability_replacements_total
    ));
    explain.push(format!(
        "stability_holds_total={}",
        metrics.stability_holds_total
    ));
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

pub(super) fn append_candidate_lines(
    explain: &mut Vec<String>,
    metrics: &SelectionMetrics,
    input: &SelectionExplainInput,
) {
    let stats = input.stats;
    explain.push(format!(
        "candidates_considered={}",
        metrics.candidates_considered
    ));
    explain.push(format!(
        "candidates_selected={}",
        metrics.candidates_selected
    ));
    explain.push(format!(
        "candidates_rejected_total={}",
        metrics.candidates_rejected_total
    ));
    explain.push(format!(
        "candidates_skipped_due_to_max_peers={}",
        metrics.candidates_skipped_due_to_max_peers
    ));
    explain.push(format!(
        "candidates_skipped_due_to_limit={}",
        metrics.candidates_skipped_due_to_limit
    ));
    explain.push(format!(
        "selection_utilization_pct={}",
        metrics.selection_utilization_pct
    ));
    explain.push(format!("selection_headroom={}", metrics.selection_headroom));
    explain.push(format!(
        "selection_pressure_summary=considered:{};selected:{};rejected:{};limit_skipped:{};utilization_pct:{};headroom:{}",
        metrics.candidates_considered,
        metrics.candidates_selected,
        metrics.candidates_rejected_total,
        metrics.candidates_skipped_due_to_limit,
        metrics.selection_utilization_pct,
        metrics.selection_headroom
    ));
    explain.push(format!(
        "selection_pressure_level={}",
        metrics.selection_pressure_level
    ));
    explain.push(format!(
        "selection_pressure_score={}",
        metrics.selection_pressure_score
    ));
    explain.push(format!(
        "selection_pressure_dominant={}",
        metrics.selection_pressure_dominant
    ));
    explain.push(format!(
        "selection_pressure_action_hint={}",
        metrics.selection_pressure_action_hint
    ));
    explain.push(format!(
        "selection_pressure_compact={}",
        metrics.selection_pressure_compact
    ));
    explain.push(format!(
        "selection_pressure_reason={}",
        metrics.selection_pressure_reason
    ));
    explain.push(format!(
        "candidate_summary=accepted:{},rejected_blocked:{},rejected_health:{},rejected_region:{},rejected_reliability:{},rejected_load:{}",
        stats.accepted_count,
        stats.rejected_blocked,
        stats.rejected_health,
        stats.rejected_region,
        stats.rejected_reliability,
        stats.rejected_load
    ));
}
