use super::*;

#[path = "path_planner_recovery_steps_last_chance.rs"]
mod last_chance;
#[path = "path_planner_recovery_steps_primary.rs"]
mod primary;
#[path = "path_planner_recovery_steps_secondary.rs"]
mod secondary;

#[derive(Default)]
pub(super) struct AutoRecoveryState {
    pub(super) health_relax_applied: bool,
    pub(super) health_relax_reason: &'static str,
    pub(super) health_relax_stage: &'static str,
    pub(super) auto_recovery_attempts: usize,
    pub(super) auto_recovery_final_result: &'static str,
    pub(super) auto_recovery_trace: Vec<&'static str>,
}

impl AutoRecoveryState {
    pub(super) fn ensure_defaults(&mut self) {
        if self.health_relax_reason.is_empty() {
            self.health_relax_reason = "not_needed";
        }
        if self.health_relax_stage.is_empty() {
            self.health_relax_stage = "none";
        }
        if self.auto_recovery_final_result.is_empty() {
            self.auto_recovery_final_result = "not_triggered";
        }
    }
}

pub(super) fn run_primary_health_step(
    peers: &BTreeMap<String, MeshPeerState>,
    candidates: &mut Vec<MeshPeerState>,
    stats: &mut CandidateStats,
    input: &AutoRecoveryInput<'_>,
    state: &mut AutoRecoveryState,
    explain: &mut Vec<String>,
) {
    primary::run_primary_health_step(peers, candidates, stats, input, state, explain);
}

pub(super) fn run_secondary_relax_step(
    peers: &BTreeMap<String, MeshPeerState>,
    candidates: &mut Vec<MeshPeerState>,
    stats: &mut CandidateStats,
    input: &AutoRecoveryInput<'_>,
    state: &mut AutoRecoveryState,
    explain: &mut Vec<String>,
) {
    secondary::run_secondary_relax_step(peers, candidates, stats, input, state, explain);
}

pub(super) fn run_last_chance_health_step(
    peers: &BTreeMap<String, MeshPeerState>,
    candidates: &mut Vec<MeshPeerState>,
    stats: &mut CandidateStats,
    input: &AutoRecoveryInput<'_>,
    state: &mut AutoRecoveryState,
    explain: &mut Vec<String>,
) {
    last_chance::run_last_chance_health_step(peers, candidates, stats, input, state, explain);
}
