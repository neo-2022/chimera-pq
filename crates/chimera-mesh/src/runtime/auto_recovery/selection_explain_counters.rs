use super::*;

pub(crate) fn append_selection_counters_explain(
    explain: &mut Vec<String>,
    selected_metrics: &SelectedPeerMetrics,
    candidates_selected: usize,
    candidates_rejected_total: usize,
    stats: &CandidateStats,
) {
    explain.push(format!(
        "candidates_considered={}",
        selected_metrics.candidates_considered
    ));
    explain.push(format!("candidates_selected={candidates_selected}"));
    explain.push(format!(
        "candidates_rejected_total={candidates_rejected_total}"
    ));
    explain.push(format!(
        "candidates_skipped_due_to_max_peers={}",
        selected_metrics.candidates_skipped_due_to_max_peers
    ));
    explain.push(format!(
        "candidates_skipped_due_to_limit={}",
        selected_metrics.candidates_skipped_due_to_limit
    ));
    explain.push(format!(
        "selection_utilization_pct={}",
        selected_metrics.selection_utilization_pct
    ));
    explain.push(format!(
        "selection_headroom={}",
        selected_metrics.selection_headroom
    ));
    explain.push(format_candidate_summary(stats));
}

pub(crate) fn format_candidate_summary(stats: &CandidateStats) -> String {
    format!(
        "candidate_summary=accepted:{},rejected_blocked:{},rejected_health:{},rejected_region:{},rejected_reliability:{},rejected_load:{}",
        stats.accepted_count,
        stats.rejected_blocked,
        stats.rejected_health,
        stats.rejected_region,
        stats.rejected_reliability,
        stats.rejected_load
    )
}
