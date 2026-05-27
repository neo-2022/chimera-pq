use super::*;

pub(super) fn run_secondary_relax_step(
    peers: &BTreeMap<String, MeshPeerState>,
    candidates: &mut Vec<MeshPeerState>,
    stats: &mut CandidateStats,
    input: &AutoRecoveryInput<'_>,
    state: &mut AutoRecoveryState,
    explain: &mut Vec<String>,
) {
    state.ensure_defaults();
    if candidates.is_empty() && input.auto_mode {
        state.auto_recovery_attempts = state.auto_recovery_attempts.saturating_add(1);
        state
            .auto_recovery_trace
            .push("secondary:region_reliability_load");
        explain.push("auto_recovery=activated".to_string());
        explain.push("auto_recovery_relaxed_filters=region,reliability,load".to_string());
        let empty_allowed_regions = BTreeSet::new();
        let (fallback_candidates, fallback_stats) = collect_candidates(
            peers,
            &CandidateFilter {
                blocked: input.blocked,
                health_blocked: input.health_blocked,
                allowed_regions: &empty_allowed_regions,
                min_reliability: 0,
                max_load: 100,
                profile: input.path_profile,
            },
            explain,
        );
        if !fallback_candidates.is_empty() {
            *candidates = fallback_candidates;
            *stats = fallback_stats;
            explain.push("auto_recovery_result=selected_from_relaxed_filters".to_string());
            state.auto_recovery_final_result = "selected_from_relaxed_filters";
            state
                .auto_recovery_trace
                .push("secondary:selected_from_relaxed_filters");
            if !state.health_relax_applied {
                state.health_relax_reason = "relaxed_filters_without_health";
            }
        } else {
            explain.push("auto_recovery_result=no_candidate_after_relax".to_string());
            state.auto_recovery_final_result = "no_candidate_after_relax";
            state
                .auto_recovery_trace
                .push("secondary:no_candidate_after_relax");
            if !state.health_relax_applied {
                state.health_relax_reason = "relaxed_filters_no_candidate";
            }
        }
    }
}
