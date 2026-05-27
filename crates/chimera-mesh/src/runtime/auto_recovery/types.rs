use super::*;

pub(crate) struct EffectiveFilterExplainContext {
    pub(crate) effective_reliability: u8,
    pub(crate) effective_load: u8,
    pub(crate) effective_max_peers: usize,
    pub(crate) effective_min_distinct_regions: usize,
    pub(crate) effective_prefer_region_diversity: bool,
    pub(crate) effective_max_selected_per_region: usize,
    pub(crate) auto_mode: bool,
}

pub(crate) struct AutoRecoverySummaryContext<'a> {
    pub(crate) health_relax_applied: bool,
    pub(crate) health_relax_reason: &'a str,
    pub(crate) health_relax_stage: &'a str,
    pub(crate) auto_recovery_attempts: usize,
    pub(crate) auto_recovery_final_result: &'a str,
}

pub(crate) struct PrimaryHealthRecoveryOutcome {
    pub(crate) candidates: Option<(Vec<MeshPeerState>, CandidateStats)>,
    pub(crate) health_relax_applied: bool,
    pub(crate) health_relax_reason: &'static str,
    pub(crate) health_relax_stage: &'static str,
    pub(crate) auto_recovery_final_result: &'static str,
    pub(crate) trace_step: &'static str,
}

pub(crate) struct SecondaryRecoveryOutcome {
    pub(crate) candidates: Option<(Vec<MeshPeerState>, CandidateStats)>,
    pub(crate) auto_recovery_final_result: &'static str,
    pub(crate) trace_step: &'static str,
    pub(crate) health_relax_reason_without_health: &'static str,
}

pub(crate) struct LastChanceRecoveryOutcome {
    pub(crate) candidates: Option<(Vec<MeshPeerState>, CandidateStats)>,
    pub(crate) health_relax_applied: bool,
    pub(crate) health_relax_reason: &'static str,
    pub(crate) health_relax_stage: &'static str,
    pub(crate) auto_recovery_final_result: &'static str,
    pub(crate) trace_step: &'static str,
}

pub(crate) struct AutoRecoveryOrchestrationContext<'a> {
    pub(crate) auto_mode: bool,
    pub(crate) path_profile: MeshPathProfile,
    pub(crate) effective_max_peers: usize,
    pub(crate) effective_reliability: u8,
    pub(crate) effective_load: u8,
    pub(crate) peers: &'a BTreeMap<String, MeshPeerState>,
    pub(crate) blocked: &'a BTreeSet<&'a str>,
    pub(crate) allowed_regions: &'a BTreeSet<String>,
    pub(crate) health_blocked: &'a BTreeSet<&'a str>,
    pub(crate) health_blocked_all: &'a BTreeSet<&'a str>,
}

pub(crate) struct AutoRecoveryOrchestrationResult {
    pub(crate) health_relax_applied: bool,
    pub(crate) health_relax_reason: &'static str,
    pub(crate) health_relax_stage: &'static str,
    pub(crate) auto_recovery_attempts: usize,
    pub(crate) auto_recovery_final_result: &'static str,
    pub(crate) auto_recovery_trace: Vec<&'static str>,
}

pub(crate) struct SelectionFeasibilityContext {
    pub(crate) candidate_distinct_regions: usize,
    pub(crate) min_distinct_regions_feasible: bool,
    pub(crate) min_distinct_regions_feasibility_gap: usize,
}

pub(crate) struct SelectionStrategyExplainContext {
    pub(crate) selection_region_cap: usize,
    pub(crate) region_cap_rejections: usize,
    pub(crate) prefer_region_diversity: bool,
}

pub(crate) struct RegionSelectionDiagnostics {
    pub(crate) min_distinct_regions_target: usize,
    pub(crate) min_distinct_regions_met: bool,
    pub(crate) distinct_region_deficit: usize,
    pub(crate) distinct_region_ratio_pct: usize,
}

pub(crate) struct WindowedPeerMetaCounters {
    pub(crate) updates: u64,
    pub(crate) replacements: u64,
    pub(crate) holds: u64,
    pub(crate) degraded: u64,
    pub(crate) churn_blocks: u64,
    pub(crate) threshold_blocks: u64,
}

pub(crate) struct SelectedPeerMetrics {
    pub(crate) selected_peer_ids: String,
    pub(crate) selected_peer_regions: String,
    pub(crate) selected_peer_endpoints: String,
    pub(crate) selected_peer_connect_priority: String,
    pub(crate) selected_peer_connect_retry_plan: String,
    pub(crate) selected_peer_connect_backoff_profile: String,
    pub(crate) selected_peer_scores: String,
    pub(crate) selected_score_sum: i32,
    pub(crate) selected_reliability_avg: usize,
    pub(crate) selected_load_avg: usize,
    pub(crate) selected_region_counts: String,
    pub(crate) candidates_selected: usize,
    pub(crate) candidates_considered: usize,
    pub(crate) candidates_skipped_due_to_max_peers: usize,
    pub(crate) candidates_skipped_due_to_limit: usize,
    pub(crate) selection_utilization_pct: usize,
    pub(crate) selection_headroom: usize,
}

pub(crate) struct SelectedStabilityMetrics {
    pub(crate) selected_peer_stability: String,
    pub(crate) selected_effective_replacement_thresholds: String,
    pub(crate) selected_replacement_decisions: String,
    pub(crate) selected_replacement_budget_remaining: String,
    pub(crate) effective_replacement_threshold_min: i32,
    pub(crate) effective_replacement_threshold_max: i32,
    pub(crate) stability_updates_total: u64,
    pub(crate) stability_replacements_total: u64,
    pub(crate) stability_holds_total: u64,
    pub(crate) stability_degraded_total: u64,
    pub(crate) stability_churn_blocks_total: u64,
    pub(crate) stability_threshold_blocks_total: u64,
    pub(crate) replacement_hold_ratio_pct: u64,
    pub(crate) replacement_budget_remaining_total: u64,
}

pub(crate) struct StabilityAggregate {
    pub(crate) selected_stability: Vec<String>,
    pub(crate) selected_effective_thresholds: Vec<String>,
    pub(crate) selected_replacement_decisions: Vec<String>,
    pub(crate) selected_replacement_budget: Vec<String>,
    pub(crate) effective_threshold_min: Option<i32>,
    pub(crate) effective_threshold_max: Option<i32>,
    pub(crate) stability_updates_total: u64,
    pub(crate) stability_replacements_total: u64,
    pub(crate) stability_holds_total: u64,
    pub(crate) stability_degraded_total: u64,
    pub(crate) stability_churn_blocks_total: u64,
    pub(crate) stability_threshold_blocks_total: u64,
    pub(crate) replacement_budget_remaining_total: u64,
}

pub(crate) struct SelectedPeerStrings {
    pub(crate) ids: String,
    pub(crate) regions: String,
    pub(crate) endpoints: String,
    pub(crate) scores: String,
}

pub(crate) struct CandidateSelectionCounters {
    pub(crate) candidates_considered: usize,
    pub(crate) candidates_skipped_due_to_max_peers: usize,
    pub(crate) candidates_skipped_due_to_limit: usize,
    pub(crate) selection_utilization_pct: usize,
    pub(crate) selection_headroom: usize,
}
