use super::*;

#[path = "path_planner_selection_metrics_build.rs"]
mod build;

pub(super) struct SelectionMetrics {
    pub(super) selected_peer_ids: String,
    pub(super) selected_peer_regions: String,
    pub(super) selected_peer_endpoints: String,
    pub(super) selected_peer_connect_priority: String,
    pub(super) selected_peer_connect_retry_plan: String,
    pub(super) selected_peer_connect_backoff_profile: String,
    pub(super) selected_peer_scores: String,
    pub(super) selected_score_sum: i32,
    pub(super) selected_reliability_avg: usize,
    pub(super) selected_load_avg: usize,
    pub(super) selected_region_counts: String,
    pub(super) selected_stability: String,
    pub(super) selected_effective_thresholds: String,
    pub(super) selected_replacement_decisions: String,
    pub(super) selected_replacement_budget: String,
    pub(super) effective_threshold_min: i32,
    pub(super) effective_threshold_max: i32,
    pub(super) stability_updates_total: u64,
    pub(super) stability_replacements_total: u64,
    pub(super) stability_holds_total: u64,
    pub(super) stability_degraded_total: u64,
    pub(super) stability_churn_blocks_total: u64,
    pub(super) stability_threshold_blocks_total: u64,
    pub(super) replacement_hold_ratio_pct: u64,
    pub(super) replacement_budget_remaining_total: u64,
    pub(super) candidates_considered: usize,
    pub(super) candidates_selected: usize,
    pub(super) candidates_rejected_total: usize,
    pub(super) candidates_skipped_due_to_max_peers: usize,
    pub(super) candidates_skipped_due_to_limit: usize,
    pub(super) selection_utilization_pct: usize,
    pub(super) selection_headroom: usize,
    pub(super) selection_pressure_level: &'static str,
    pub(super) selection_pressure_score: usize,
    pub(super) selection_pressure_dominant: &'static str,
    pub(super) selection_pressure_action_hint: &'static str,
    pub(super) selection_pressure_compact: String,
    pub(super) selection_pressure_reason: String,
}

pub(super) fn build_selection_metrics(
    runtime: &MeshRuntime,
    policy: &MeshPathPolicy,
    selected_peers: &[MeshPeerState],
    stats: CandidateStats,
    region_cap_rejections: usize,
    effective_max_peers: usize,
) -> SelectionMetrics {
    build::build_selection_metrics_impl(
        runtime,
        policy,
        selected_peers,
        stats,
        region_cap_rejections,
        effective_max_peers,
    )
}
