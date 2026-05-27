use super::*;

pub(crate) fn build_candidate_selection_counters(
    stats: &CandidateStats,
    candidates_selected: usize,
    region_cap_rejections: usize,
    effective_max_peers: usize,
) -> CandidateSelectionCounters {
    let candidates_rejected_total = stats.rejected_total();
    let candidates_considered = stats
        .accepted_count
        .saturating_add(candidates_rejected_total);
    let candidates_skipped_due_to_max_peers = stats
        .accepted_count
        .saturating_sub(candidates_selected)
        .saturating_sub(region_cap_rejections);
    let candidates_skipped_due_to_limit =
        candidates_skipped_due_to_max_peers.saturating_add(region_cap_rejections);
    let selection_utilization_pct = candidates_selected
        .saturating_mul(100)
        .checked_div(effective_max_peers)
        .unwrap_or(0);
    let selection_headroom = effective_max_peers.saturating_sub(candidates_selected);
    CandidateSelectionCounters {
        candidates_considered,
        candidates_skipped_due_to_max_peers,
        candidates_skipped_due_to_limit,
        selection_utilization_pct,
        selection_headroom,
    }
}
