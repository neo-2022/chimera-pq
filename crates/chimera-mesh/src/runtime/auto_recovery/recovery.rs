use super::*;

pub(crate) fn run_primary_health_auto_recovery(
    peers: &BTreeMap<String, MeshPeerState>,
    blocked: &BTreeSet<&str>,
    allowed_regions: &BTreeSet<String>,
    effective_reliability: u8,
    effective_load: u8,
    path_profile: MeshPathProfile,
    explain: &mut Vec<String>,
) -> PrimaryHealthRecoveryOutcome {
    explain.push("auto_recovery=activated".to_string());
    explain.push("auto_recovery_relaxed_filters=health".to_string());
    let empty_health_blocked = BTreeSet::new();
    let (fallback_candidates, fallback_stats) = collect_candidates(
        peers,
        &CandidateFilter {
            blocked,
            health_blocked: &empty_health_blocked,
            allowed_regions,
            min_reliability: effective_reliability,
            max_load: effective_load,
            profile: path_profile,
        },
        explain,
    );
    if !fallback_candidates.is_empty() {
        explain.push("auto_recovery_result=selected_from_relaxed_health".to_string());
        PrimaryHealthRecoveryOutcome {
            candidates: Some((fallback_candidates, fallback_stats)),
            health_relax_applied: true,
            health_relax_reason: "resilient_health_relax",
            health_relax_stage: "primary",
            auto_recovery_final_result: "selected_from_relaxed_health",
            trace_step: "primary:selected_from_relaxed_health",
        }
    } else {
        explain.push("auto_recovery_result=no_candidate_after_relax_health".to_string());
        PrimaryHealthRecoveryOutcome {
            candidates: None,
            health_relax_applied: false,
            health_relax_reason: "relax_attempt_no_gain",
            health_relax_stage: "none",
            auto_recovery_final_result: "no_candidate_after_relax_health",
            trace_step: "primary:no_candidate_after_relax_health",
        }
    }
}

pub(crate) fn run_secondary_auto_recovery(
    peers: &BTreeMap<String, MeshPeerState>,
    blocked: &BTreeSet<&str>,
    health_blocked: &BTreeSet<&str>,
    path_profile: MeshPathProfile,
    explain: &mut Vec<String>,
) -> SecondaryRecoveryOutcome {
    explain.push("auto_recovery=activated".to_string());
    explain.push("auto_recovery_relaxed_filters=region,reliability,load".to_string());
    let empty_allowed_regions = BTreeSet::new();
    let (fallback_candidates, fallback_stats) = collect_candidates(
        peers,
        &CandidateFilter {
            blocked,
            health_blocked,
            allowed_regions: &empty_allowed_regions,
            min_reliability: 0,
            max_load: 100,
            profile: path_profile,
        },
        explain,
    );
    if !fallback_candidates.is_empty() {
        explain.push("auto_recovery_result=selected_from_relaxed_filters".to_string());
        SecondaryRecoveryOutcome {
            candidates: Some((fallback_candidates, fallback_stats)),
            auto_recovery_final_result: "selected_from_relaxed_filters",
            trace_step: "secondary:selected_from_relaxed_filters",
            health_relax_reason_without_health: "relaxed_filters_without_health",
        }
    } else {
        explain.push("auto_recovery_result=no_candidate_after_relax".to_string());
        SecondaryRecoveryOutcome {
            candidates: None,
            auto_recovery_final_result: "no_candidate_after_relax",
            trace_step: "secondary:no_candidate_after_relax",
            health_relax_reason_without_health: "relaxed_filters_no_candidate",
        }
    }
}

pub(crate) fn run_last_chance_health_auto_recovery(
    peers: &BTreeMap<String, MeshPeerState>,
    blocked: &BTreeSet<&str>,
    path_profile: MeshPathProfile,
    current_candidate_count: usize,
    explain: &mut Vec<String>,
) -> LastChanceRecoveryOutcome {
    explain.push("auto_recovery=activated".to_string());
    explain.push("auto_recovery_relaxed_filters=health,last_chance".to_string());
    let empty_allowed_regions = BTreeSet::new();
    let empty_health_blocked = BTreeSet::new();
    let (fallback_candidates, fallback_stats) = collect_candidates(
        peers,
        &CandidateFilter {
            blocked,
            health_blocked: &empty_health_blocked,
            allowed_regions: &empty_allowed_regions,
            min_reliability: 0,
            max_load: 100,
            profile: path_profile,
        },
        explain,
    );
    if fallback_candidates.len() > current_candidate_count {
        explain.push("auto_recovery_result=selected_from_last_chance_health".to_string());
        LastChanceRecoveryOutcome {
            candidates: Some((fallback_candidates, fallback_stats)),
            health_relax_applied: true,
            health_relax_reason: "last_chance_health_relax",
            health_relax_stage: "last_chance",
            auto_recovery_final_result: "selected_from_last_chance_health",
            trace_step: "last_chance:selected_from_last_chance_health",
        }
    } else {
        explain.push("auto_recovery_result=no_candidate_after_last_chance_health".to_string());
        LastChanceRecoveryOutcome {
            candidates: None,
            health_relax_applied: false,
            health_relax_reason: "last_chance_no_gain",
            health_relax_stage: "none",
            auto_recovery_final_result: "no_candidate_after_last_chance_health",
            trace_step: "last_chance:no_candidate_after_last_chance_health",
        }
    }
}
