use super::CandidateStats;
use super::path_planner_selection_explain::SelectionExplainInput;
use super::path_planner_selection_metrics::SelectionMetrics;
use std::fmt::{Display, Write};

pub(super) fn append_selected_peer_lines(explain: &mut Vec<String>, metrics: &SelectionMetrics) {
    push_line_str(
        explain,
        "selected_peer_ids",
        metrics.selected_peer_ids.as_str(),
    );
    push_line_str(
        explain,
        "selected_peer_regions",
        metrics.selected_peer_regions.as_str(),
    );
    push_line_str(
        explain,
        "selected_peer_endpoints",
        metrics.selected_peer_endpoints.as_str(),
    );
    push_line_str(
        explain,
        "selected_peer_connect_priority",
        metrics.selected_peer_connect_priority.as_str(),
    );
    push_line_str(
        explain,
        "selected_peer_connect_retry_plan",
        metrics.selected_peer_connect_retry_plan.as_str(),
    );
    push_line_str(
        explain,
        "selected_peer_connect_backoff_profile",
        metrics.selected_peer_connect_backoff_profile.as_str(),
    );
    push_line_str(
        explain,
        "selected_peer_scores",
        metrics.selected_peer_scores.as_str(),
    );
    push_line_display(explain, "selected_score_sum", metrics.selected_score_sum);
    push_line_display(
        explain,
        "selected_reliability_avg",
        metrics.selected_reliability_avg,
    );
    push_line_display(explain, "selected_load_avg", metrics.selected_load_avg);
    push_line_str(
        explain,
        "selected_region_counts",
        metrics.selected_region_counts.as_str(),
    );
}

pub(super) fn append_stability_lines(explain: &mut Vec<String>, metrics: &SelectionMetrics) {
    push_line_str(
        explain,
        "selected_peer_stability",
        metrics.selected_stability.as_str(),
    );
    push_line_str(
        explain,
        "selected_effective_replacement_thresholds",
        metrics.selected_effective_thresholds.as_str(),
    );
    push_line_str(
        explain,
        "selected_replacement_decisions",
        metrics.selected_replacement_decisions.as_str(),
    );
    push_line_str(
        explain,
        "selected_replacement_budget_remaining",
        metrics.selected_replacement_budget.as_str(),
    );
    push_line_display(
        explain,
        "effective_replacement_threshold_min",
        metrics.effective_threshold_min,
    );
    push_line_display(
        explain,
        "effective_replacement_threshold_max",
        metrics.effective_threshold_max,
    );
    push_line_display(
        explain,
        "stability_updates_total",
        metrics.stability_updates_total,
    );
    push_line_display(
        explain,
        "stability_replacements_total",
        metrics.stability_replacements_total,
    );
    push_line_display(
        explain,
        "stability_holds_total",
        metrics.stability_holds_total,
    );
    push_line_display(
        explain,
        "stability_degraded_total",
        metrics.stability_degraded_total,
    );
    push_line_display(
        explain,
        "stability_churn_blocks_total",
        metrics.stability_churn_blocks_total,
    );
    push_line_display(
        explain,
        "stability_threshold_blocks_total",
        metrics.stability_threshold_blocks_total,
    );
    push_line_display(
        explain,
        "replacement_hold_ratio_pct",
        metrics.replacement_hold_ratio_pct,
    );
    push_line_display(
        explain,
        "replacement_budget_remaining_total",
        metrics.replacement_budget_remaining_total,
    );
}

pub(super) fn append_candidate_lines(
    explain: &mut Vec<String>,
    metrics: &SelectionMetrics,
    input: &SelectionExplainInput,
) {
    let stats = input.stats;
    push_line_display(
        explain,
        "candidates_considered",
        metrics.candidates_considered,
    );
    push_line_display(explain, "candidates_selected", metrics.candidates_selected);
    push_line_display(
        explain,
        "candidates_rejected_total",
        metrics.candidates_rejected_total,
    );
    push_line_display(
        explain,
        "candidates_skipped_due_to_max_peers",
        metrics.candidates_skipped_due_to_max_peers,
    );
    push_line_display(
        explain,
        "candidates_skipped_due_to_limit",
        metrics.candidates_skipped_due_to_limit,
    );
    push_line_display(
        explain,
        "selection_utilization_pct",
        metrics.selection_utilization_pct,
    );
    push_line_display(explain, "selection_headroom", metrics.selection_headroom);
    explain.push(build_selection_pressure_summary(metrics));
    push_line_display(
        explain,
        "selection_pressure_level",
        metrics.selection_pressure_level,
    );
    push_line_display(
        explain,
        "selection_pressure_score",
        metrics.selection_pressure_score,
    );
    push_line_display(
        explain,
        "selection_pressure_dominant",
        metrics.selection_pressure_dominant,
    );
    push_line_display(
        explain,
        "selection_pressure_action_hint",
        metrics.selection_pressure_action_hint,
    );
    push_line_str(
        explain,
        "selection_pressure_compact",
        metrics.selection_pressure_compact.as_str(),
    );
    push_line_str(
        explain,
        "selection_pressure_reason",
        metrics.selection_pressure_reason.as_str(),
    );
    explain.push(build_candidate_summary(stats));
}

fn build_selection_pressure_summary(metrics: &SelectionMetrics) -> String {
    let mut out = String::with_capacity(128);
    let _ = write!(
        out,
        "selection_pressure_summary=considered:{};selected:{};rejected:{};limit_skipped:{};utilization_pct:{};headroom:{}",
        metrics.candidates_considered,
        metrics.candidates_selected,
        metrics.candidates_rejected_total,
        metrics.candidates_skipped_due_to_limit,
        metrics.selection_utilization_pct,
        metrics.selection_headroom
    );
    out
}

fn build_candidate_summary(stats: CandidateStats) -> String {
    let mut out = String::with_capacity(160);
    let _ = write!(
        out,
        "candidate_summary=accepted:{},rejected_blocked:{},rejected_health:{},rejected_region:{},rejected_reliability:{},rejected_load:{}",
        stats.accepted_count,
        stats.rejected_blocked,
        stats.rejected_health,
        stats.rejected_region,
        stats.rejected_reliability,
        stats.rejected_load
    );
    out
}

fn push_line_display<T: Display>(explain: &mut Vec<String>, key: &str, value: T) {
    let mut out = String::with_capacity(key.len().saturating_add(32));
    out.push_str(key);
    out.push('=');
    let _ = write!(&mut out, "{}", value);
    explain.push(out);
}

fn push_line_str(explain: &mut Vec<String>, key: &str, value: &str) {
    let mut out = String::with_capacity(key.len().saturating_add(value.len()).saturating_add(1));
    out.push_str(key);
    out.push('=');
    out.push_str(value);
    explain.push(out);
}
