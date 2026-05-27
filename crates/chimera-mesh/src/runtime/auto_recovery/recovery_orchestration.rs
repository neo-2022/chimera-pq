use super::*;

pub(crate) fn sort_candidates_and_build_selection_context(
    candidates: &mut [MeshPeerState],
    min_distinct_regions_target: usize,
) -> SelectionFeasibilityContext {
    candidates.sort_by(|a, b| {
        b.selection_score
            .cmp(&a.selection_score)
            .then_with(|| a.load_score.cmp(&b.load_score))
            .then_with(|| b.reliability_score.cmp(&a.reliability_score))
            .then_with(|| a.node_id.cmp(&b.node_id))
    });
    let candidate_distinct_regions = candidates
        .iter()
        .map(|peer| normalize_region_key(&peer.region))
        .collect::<BTreeSet<_>>()
        .len();
    let min_distinct_regions_feasible = candidate_distinct_regions >= min_distinct_regions_target;
    let min_distinct_regions_feasibility_gap =
        min_distinct_regions_target.saturating_sub(candidate_distinct_regions);
    SelectionFeasibilityContext {
        candidate_distinct_regions,
        min_distinct_regions_feasible,
        min_distinct_regions_feasibility_gap,
    }
}

pub(crate) fn orchestrate_auto_recovery(
    candidates: &mut Vec<MeshPeerState>,
    stats: &mut CandidateStats,
    context: &AutoRecoveryOrchestrationContext<'_>,
    explain: &mut Vec<String>,
) -> AutoRecoveryOrchestrationResult {
    let mut health_relax_applied = false;
    let mut health_relax_reason = "not_needed";
    let mut health_relax_stage = "none";
    let mut auto_recovery_attempts = 0usize;
    let mut auto_recovery_final_result = "not_triggered";
    let mut auto_recovery_trace: Vec<&'static str> = Vec::new();

    if context.auto_mode
        && context.path_profile == MeshPathProfile::Resilient
        && candidates.len() < context.effective_max_peers
        && !context.health_blocked_all.is_empty()
    {
        let outcome = run_primary_health_auto_recovery(
            context.peers,
            context.blocked,
            context.allowed_regions,
            context.effective_reliability,
            context.effective_load,
            context.path_profile,
            explain,
        );
        auto_recovery_attempts = auto_recovery_attempts.saturating_add(1);
        auto_recovery_trace.push("primary:health");
        if let Some((fallback_candidates, fallback_stats)) = outcome.candidates {
            *candidates = fallback_candidates;
            *stats = fallback_stats;
        }
        health_relax_applied = outcome.health_relax_applied;
        health_relax_reason = outcome.health_relax_reason;
        health_relax_stage = outcome.health_relax_stage;
        auto_recovery_final_result = outcome.auto_recovery_final_result;
        auto_recovery_trace.push(outcome.trace_step);
    } else if !context.auto_mode {
        health_relax_reason = "manual_override_disabled";
    } else if context.path_profile != MeshPathProfile::Resilient {
        health_relax_reason = "profile_not_resilient";
    } else if candidates.len() >= context.effective_max_peers {
        health_relax_reason = "capacity_already_satisfied";
    } else if context.health_blocked_all.is_empty() {
        health_relax_reason = "no_health_blocked_candidates";
    }

    if candidates.is_empty() && context.auto_mode {
        let outcome = run_secondary_auto_recovery(
            context.peers,
            context.blocked,
            context.health_blocked,
            context.path_profile,
            explain,
        );
        auto_recovery_attempts = auto_recovery_attempts.saturating_add(1);
        auto_recovery_trace.push("secondary:region_reliability_load");
        if let Some((fallback_candidates, fallback_stats)) = outcome.candidates {
            *candidates = fallback_candidates;
            *stats = fallback_stats;
        }
        auto_recovery_final_result = outcome.auto_recovery_final_result;
        auto_recovery_trace.push(outcome.trace_step);
        if !health_relax_applied {
            health_relax_reason = outcome.health_relax_reason_without_health;
        }
    }

    if context.auto_mode
        && candidates.len() < context.effective_max_peers
        && !context.health_blocked_all.is_empty()
    {
        let outcome = run_last_chance_health_auto_recovery(
            context.peers,
            context.blocked,
            context.path_profile,
            candidates.len(),
            explain,
        );
        auto_recovery_attempts = auto_recovery_attempts.saturating_add(1);
        auto_recovery_trace.push("last_chance:health");
        if let Some((fallback_candidates, fallback_stats)) = outcome.candidates {
            *candidates = fallback_candidates;
            *stats = fallback_stats;
        }
        if outcome.health_relax_applied {
            health_relax_applied = true;
            health_relax_reason = outcome.health_relax_reason;
            health_relax_stage = outcome.health_relax_stage;
        } else if !health_relax_applied {
            health_relax_reason = outcome.health_relax_reason;
        }
        auto_recovery_final_result = outcome.auto_recovery_final_result;
        auto_recovery_trace.push(outcome.trace_step);
    }

    AutoRecoveryOrchestrationResult {
        health_relax_applied,
        health_relax_reason,
        health_relax_stage,
        auto_recovery_attempts,
        auto_recovery_final_result,
        auto_recovery_trace,
    }
}
