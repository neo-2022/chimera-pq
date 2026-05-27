use super::*;

pub(super) fn run_last_chance_health_step(
    peers: &BTreeMap<String, MeshPeerState>,
    candidates: &mut Vec<MeshPeerState>,
    stats: &mut CandidateStats,
    input: &AutoRecoveryInput<'_>,
    state: &mut AutoRecoveryState,
    explain: &mut Vec<String>,
) {
    state.ensure_defaults();
    if input.auto_mode
        && candidates.len() < input.effective_max_peers
        && !input.health_blocked_all.is_empty()
    {
        state.auto_recovery_attempts = state.auto_recovery_attempts.saturating_add(1);
        state.auto_recovery_trace.push("last_chance:health");
        explain.push("auto_recovery=activated".to_string());
        explain.push("auto_recovery_relaxed_filters=health,last_chance".to_string());
        let empty_allowed_regions = BTreeSet::new();
        let empty_health_blocked = BTreeSet::new();
        let (fallback_candidates, fallback_stats) = collect_candidates(
            peers,
            &CandidateFilter {
                blocked: input.blocked,
                health_blocked: &empty_health_blocked,
                allowed_regions: &empty_allowed_regions,
                min_reliability: 0,
                max_load: 100,
                profile: input.path_profile,
            },
            explain,
        );
        if fallback_candidates.len() > candidates.len() {
            *candidates = fallback_candidates;
            *stats = fallback_stats;
            state.health_relax_applied = true;
            explain.push("auto_recovery_result=selected_from_last_chance_health".to_string());
            state.health_relax_reason = "last_chance_health_relax";
            state.health_relax_stage = "last_chance";
            state.auto_recovery_final_result = "selected_from_last_chance_health";
            state
                .auto_recovery_trace
                .push("last_chance:selected_from_last_chance_health");
        } else {
            explain.push("auto_recovery_result=no_candidate_after_last_chance_health".to_string());
            state.auto_recovery_final_result = "no_candidate_after_last_chance_health";
            state
                .auto_recovery_trace
                .push("last_chance:no_candidate_after_last_chance_health");
            if !state.health_relax_applied {
                state.health_relax_reason = "last_chance_no_gain";
            }
        }
    }
}
