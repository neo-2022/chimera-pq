use self::path_planner_setup_effective::resolve_effective_plan_policy;
use self::path_planner_setup_explain::{
    PlanSetupPrefaceExplainContext, append_plan_setup_discovery_table_explain,
    append_plan_setup_preface_explain,
};
use super::*;
use crate::runtime::health_state_utils::unhealthy_node_ids_from_health_state;

#[path = "path_planner_setup_effective.rs"]
mod path_planner_setup_effective;
#[path = "path_planner_setup_explain.rs"]
mod path_planner_setup_explain;

pub(super) struct PlanSetup {
    pub(super) path_profile: MeshPathProfile,
    pub(super) blocked_node_ids: Vec<String>,
    pub(super) health_blocked_all: BTreeSet<String>,
    pub(super) allowed_regions: BTreeSet<String>,
    pub(super) auto_mode: bool,
    pub(super) effective_max_peers: usize,
    pub(super) effective_min_distinct_regions: usize,
    pub(super) effective_prefer_region_diversity: bool,
    pub(super) effective_max_selected_per_region: usize,
    pub(super) effective_reliability: u8,
    pub(super) effective_load: u8,
}

pub(super) fn build_plan_setup(
    runtime: &MeshRuntime,
    join_mode: MeshJoinMode,
    policy: &MeshPathPolicy,
    explain: &mut Vec<String>,
) -> PlanSetup {
    let manual_overrides = policy.manual_override_fields();
    let auto_mode = manual_overrides.is_empty();

    let (avg_load_score, avg_reliability_score) = runtime_peer_signal_averages(&runtime.peers);
    let (path_profile, path_profile_reason) = resolve_path_profile(
        policy.path_profile_override,
        &runtime.profile_state,
        runtime.tick,
        runtime.table_policy.profile_hysteresis_ticks,
        avg_load_score,
        avg_reliability_score,
    );

    append_plan_setup_preface_explain(
        runtime,
        explain,
        &PlanSetupPrefaceExplainContext {
            manual_overrides: &manual_overrides,
            path_profile,
            path_profile_reason,
            path_profile_overridden: policy.path_profile_override.is_some(),
            avg_load_score,
            avg_reliability_score,
        },
    );
    append_plan_setup_discovery_table_explain(runtime, explain, join_mode);

    let blocked_node_ids = policy.blocked_node_ids.clone();
    let health_blocked_all = unhealthy_node_ids_from_health_state(&runtime.health_state);
    explain.push(format!(
        "effective_health_blocked_candidates={}",
        health_blocked_all.len()
    ));
    let health_blocked_node_ids =
        crate::runtime::health_state_utils::format_node_set(&health_blocked_all);
    explain.push(format!(
        "effective_health_blocked_node_ids={health_blocked_node_ids}"
    ));

    let allowed_regions: BTreeSet<String> = policy
        .allowed_regions
        .iter()
        .map(|value| normalize_region_key(value))
        .collect();

    let effective = resolve_effective_plan_policy(
        policy,
        path_profile,
        auto_mode,
        avg_reliability_score,
        avg_load_score,
    );

    PlanSetup {
        path_profile,
        blocked_node_ids,
        health_blocked_all,
        allowed_regions,
        auto_mode,
        effective_max_peers: effective.effective_max_peers,
        effective_min_distinct_regions: effective.effective_min_distinct_regions,
        effective_prefer_region_diversity: effective.effective_prefer_region_diversity,
        effective_max_selected_per_region: effective.effective_max_selected_per_region,
        effective_reliability: effective.effective_reliability,
        effective_load: effective.effective_load,
    }
}
