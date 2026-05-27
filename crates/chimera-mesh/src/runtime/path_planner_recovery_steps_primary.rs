use super::*;

pub(super) fn run_primary_health_step(
    peers: &BTreeMap<String, MeshPeerState>,
    candidates: &mut Vec<MeshPeerState>,
    stats: &mut CandidateStats,
    input: &AutoRecoveryInput<'_>,
    state: &mut AutoRecoveryState,
    explain: &mut Vec<String>,
) {
    state.ensure_defaults();
    if input.auto_mode
        && input.path_profile == MeshPathProfile::Resilient
        && candidates.len() < input.effective_max_peers
        && !input.health_blocked_all.is_empty()
    {
        state.auto_recovery_attempts = state.auto_recovery_attempts.saturating_add(1);
        state.auto_recovery_trace.push("primary:health");
        explain.push("auto_recovery=activated".to_string());
        explain.push("auto_recovery_relaxed_filters=health".to_string());
        let empty_health_blocked = BTreeSet::new();
        let (fallback_candidates, fallback_stats) = collect_candidates(
            peers,
            &CandidateFilter {
                blocked: input.blocked,
                health_blocked: &empty_health_blocked,
                allowed_regions: input.allowed_regions,
                min_reliability: input.effective_reliability,
                max_load: input.effective_load,
                profile: input.path_profile,
            },
            explain,
        );
        if !fallback_candidates.is_empty() {
            *candidates = fallback_candidates;
            *stats = fallback_stats;
            explain.push("auto_recovery_result=selected_from_relaxed_health".to_string());
            state.health_relax_applied = true;
            state.health_relax_reason = "resilient_health_relax";
            state.health_relax_stage = "primary";
            state.auto_recovery_final_result = "selected_from_relaxed_health";
            state
                .auto_recovery_trace
                .push("primary:selected_from_relaxed_health");
        } else {
            explain.push("auto_recovery_result=no_candidate_after_relax_health".to_string());
            state.health_relax_reason = "relax_attempt_no_gain";
            state.auto_recovery_final_result = "no_candidate_after_relax_health";
            state
                .auto_recovery_trace
                .push("primary:no_candidate_after_relax_health");
        }
    } else if !input.auto_mode {
        state.health_relax_reason = "manual_override_disabled";
    } else if input.path_profile != MeshPathProfile::Resilient {
        state.health_relax_reason = "profile_not_resilient";
    } else if candidates.len() >= input.effective_max_peers {
        state.health_relax_reason = "capacity_already_satisfied";
    } else if input.health_blocked_all.is_empty() {
        state.health_relax_reason = "no_health_blocked_candidates";
    }
}
