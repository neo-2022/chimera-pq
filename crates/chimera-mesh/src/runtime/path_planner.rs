use super::join_mode::evaluate_join_mode;
use super::path_planner_finalize::{SelectionFinalizeInput, finalize_selection};
use super::path_planner_recovery::{AutoRecoveryInput, run_auto_recovery};
use super::path_planner_setup::build_plan_setup;
use super::standby_shadow_explain::append_standby_shadow_explain;
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PlanningExplainMode {
    Full,
    None,
}

impl PlanningExplainMode {
    pub(super) fn enabled(self) -> bool {
        matches!(self, Self::Full)
    }
}

impl MeshRuntime {
    pub fn plan_path(
        &self,
        request: &MeshJoinRequest,
        policy: &MeshPathPolicy,
    ) -> Result<MeshPathPlan, String> {
        if request.namespace.trim() != self.namespace {
            return Err("mesh request namespace does not match runtime".to_string());
        }
        let join_mode = evaluate_join_mode(request)?;
        policy.validate()?;
        self.rebuild_plan_snapshot_from_runtime_state(join_mode, policy)
    }

    pub fn plan_path_core(
        &self,
        request: &MeshJoinRequest,
        policy: &MeshPathPolicy,
    ) -> Result<MeshPathPlanCore, String> {
        if request.namespace.trim() != self.namespace {
            return Err("mesh request namespace does not match runtime".to_string());
        }
        let join_mode = evaluate_join_mode(request)?;
        policy.validate()?;
        self.rebuild_plan_snapshot_core_from_runtime_state(join_mode, policy)
    }
}

pub(super) fn build_plan_from_runtime_state(
    runtime: &MeshRuntime,
    join_mode: MeshJoinMode,
    policy: &MeshPathPolicy,
) -> Result<MeshPathPlan, String> {
    let outcome = build_plan_outcome_from_runtime_state(
        runtime,
        join_mode,
        policy,
        PlanningExplainMode::Full,
    )?;
    Ok(MeshPathPlan {
        namespace: runtime.namespace.clone(),
        join_mode,
        selected_peers: outcome.selected_peers,
        multipath_schedule: outcome.multipath_schedule,
        explain: outcome.explain,
    })
}

pub(super) fn build_plan_core_from_runtime_state(
    runtime: &MeshRuntime,
    join_mode: MeshJoinMode,
    policy: &MeshPathPolicy,
) -> Result<MeshPathPlanCore, String> {
    let outcome = build_plan_outcome_from_runtime_state(
        runtime,
        join_mode,
        policy,
        PlanningExplainMode::None,
    )?;
    Ok(MeshPathPlanCore {
        namespace: runtime.namespace.clone(),
        join_mode,
        selected_peers: outcome.selected_peers,
        multipath_schedule: outcome.multipath_schedule,
    })
}

struct BuildPlanOutcome {
    selected_peers: Vec<MeshPeerState>,
    multipath_schedule: MeshMultipathSchedule,
    explain: Vec<String>,
}

fn build_plan_outcome_from_runtime_state(
    runtime: &MeshRuntime,
    join_mode: MeshJoinMode,
    policy: &MeshPathPolicy,
    explain_mode: PlanningExplainMode,
) -> Result<BuildPlanOutcome, String> {
    let mut explain = if explain_mode.enabled() {
        Vec::with_capacity(128 + runtime.peers.len().saturating_mul(4))
    } else {
        Vec::new()
    };
    let setup = build_plan_setup(runtime, join_mode, policy, &mut explain, explain_mode);
    let blocked: BTreeSet<&str> = setup
        .blocked_node_ids
        .iter()
        .map(std::string::String::as_str)
        .collect();
    let health_blocked_all: BTreeSet<&str> = setup
        .health_blocked_all
        .iter()
        .map(std::string::String::as_str)
        .collect();
    let empty_health_blocked = BTreeSet::new();
    let health_blocked = if setup.auto_mode {
        &health_blocked_all
    } else {
        &empty_health_blocked
    };
    let (candidates, stats) = collect_candidates(
        &runtime.peers,
        &CandidateFilter {
            blocked: &blocked,
            health_blocked,
            allowed_regions: &setup.allowed_regions,
            min_reliability: setup.effective_reliability,
            max_load: setup.effective_load,
            profile: setup.path_profile,
        },
        &mut explain,
        explain_mode,
    );
    if explain_mode.enabled() {
        explain.push(format!(
            "effective_min_reliability={}",
            setup.effective_reliability
        ));
        explain.push(format!("effective_max_load={}", setup.effective_load));
        explain.push(format!("effective_max_peers={}", setup.effective_max_peers));
        explain.push(format!(
            "effective_min_distinct_regions={}",
            setup.effective_min_distinct_regions
        ));
        explain.push(format!(
            "effective_prefer_region_diversity={}",
            setup.effective_prefer_region_diversity
        ));
        explain.push(format!(
            "effective_max_selected_per_region={}",
            setup.effective_max_selected_per_region
        ));
        explain.push(format!(
            "effective_filter_source={}",
            if setup.auto_mode {
                "auto_profile"
            } else {
                "manual_override"
            }
        ));
        explain.push(format!(
            "effective_health_filter_source={}",
            if setup.auto_mode {
                "auto"
            } else {
                "manual_disabled"
            }
        ));
    }
    let recovery = run_auto_recovery(
        &runtime.peers,
        candidates,
        stats,
        AutoRecoveryInput {
            blocked: &blocked,
            health_blocked_all: &health_blocked_all,
            health_blocked,
            allowed_regions: &setup.allowed_regions,
            effective_reliability: setup.effective_reliability,
            effective_load: setup.effective_load,
            effective_max_peers: setup.effective_max_peers,
            auto_mode: setup.auto_mode,
            path_profile: setup.path_profile,
            spread_bonus_weight: runtime.table_policy.resilient_region_spread_bonus_weight,
        },
        &mut explain,
        explain_mode,
    );
    let candidates = recovery.candidates;
    let stats = recovery.stats;

    let selected_peers = finalize_selection(
        runtime,
        SelectionFinalizeInput {
            policy,
            stats,
            candidates,
            effective_prefer_region_diversity: setup.effective_prefer_region_diversity,
            effective_max_peers: setup.effective_max_peers,
            effective_max_selected_per_region: setup.effective_max_selected_per_region,
            effective_min_distinct_regions: setup.effective_min_distinct_regions,
        },
        &mut explain,
        explain_mode,
    )?;
    if explain_mode.enabled() {
        append_standby_shadow_explain(&selected_peers, &mut explain);
    }
    let multipath_mode = policy
        .multipath_mode
        .map(schedule_mode_from_multipath_hint)
        .unwrap_or(MeshMultipathMode::Off);
    let multipath_schedule = build_multipath_schedule(
        &selected_peers,
        multipath_mode,
        None,
        policy.multipath_demand,
        &[],
    )?;
    if explain_mode.enabled() {
        multipath_schedule::append_multipath_schedule_explain(&mut explain, &multipath_schedule);
    }

    Ok(BuildPlanOutcome {
        selected_peers,
        multipath_schedule,
        explain,
    })
}
