use super::path_planner_recovery_explain::{
    AutoRecoveryExplainSummary, append_auto_recovery_explain,
};
use super::selection_policy::apply_resilient_region_spread_bonus;
use super::*;

#[path = "path_planner_recovery_steps.rs"]
mod steps;

pub(super) struct AutoRecoveryInput<'a> {
    pub(super) blocked: &'a BTreeSet<&'a str>,
    pub(super) health_blocked_all: &'a BTreeSet<&'a str>,
    pub(super) health_blocked: &'a BTreeSet<&'a str>,
    pub(super) allowed_regions: &'a BTreeSet<String>,
    pub(super) effective_reliability: u8,
    pub(super) effective_load: u8,
    pub(super) effective_max_peers: usize,
    pub(super) auto_mode: bool,
    pub(super) path_profile: MeshPathProfile,
    pub(super) spread_bonus_weight: u8,
}

pub(super) struct AutoRecoveryOutcome {
    pub(super) candidates: Vec<MeshPeerState>,
    pub(super) stats: CandidateStats,
}

pub(super) fn run_auto_recovery(
    peers: &BTreeMap<String, MeshPeerState>,
    mut candidates: Vec<MeshPeerState>,
    mut stats: CandidateStats,
    input: AutoRecoveryInput<'_>,
    explain: &mut Vec<String>,
) -> AutoRecoveryOutcome {
    let mut state = steps::AutoRecoveryState::default();

    steps::run_primary_health_step(
        peers,
        &mut candidates,
        &mut stats,
        &input,
        &mut state,
        explain,
    );
    steps::run_secondary_relax_step(
        peers,
        &mut candidates,
        &mut stats,
        &input,
        &mut state,
        explain,
    );
    steps::run_last_chance_health_step(
        peers,
        &mut candidates,
        &mut stats,
        &input,
        &mut state,
        explain,
    );

    append_auto_recovery_explain(
        explain,
        AutoRecoveryExplainSummary {
            health_relax_applied: state.health_relax_applied,
            health_relax_reason: state.health_relax_reason,
            health_relax_stage: state.health_relax_stage,
            auto_recovery_attempts: state.auto_recovery_attempts,
            auto_recovery_final_result: state.auto_recovery_final_result,
            auto_recovery_trace: &state.auto_recovery_trace,
        },
    );

    let (spread_bonus_applied, spread_bonus_total) = apply_resilient_region_spread_bonus(
        &mut candidates,
        input.path_profile,
        input.spread_bonus_weight,
    );
    explain.push(format!(
        "resilient_region_spread_bonus_applied={spread_bonus_applied}"
    ));
    explain.push(format!(
        "resilient_region_spread_bonus_total={spread_bonus_total}"
    ));

    AutoRecoveryOutcome { candidates, stats }
}
