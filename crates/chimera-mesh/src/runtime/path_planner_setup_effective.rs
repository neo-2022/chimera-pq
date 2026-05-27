use super::selection_profile::{
    effective_max_load, effective_max_peers, effective_max_selected_per_region,
    effective_min_distinct_regions, effective_min_reliability, effective_prefer_region_diversity,
};
use super::*;

pub(super) struct EffectivePlanPolicy {
    pub(super) effective_max_peers: usize,
    pub(super) effective_min_distinct_regions: usize,
    pub(super) effective_prefer_region_diversity: bool,
    pub(super) effective_max_selected_per_region: usize,
    pub(super) effective_reliability: u8,
    pub(super) effective_load: u8,
}

pub(super) fn resolve_effective_plan_policy(
    policy: &MeshPathPolicy,
    path_profile: MeshPathProfile,
    auto_mode: bool,
    avg_reliability_score: u8,
    avg_load_score: u8,
) -> EffectivePlanPolicy {
    let effective_max_peers = effective_max_peers(policy.max_peers, path_profile, auto_mode);
    let effective_min_distinct_regions = effective_min_distinct_regions(
        policy.min_distinct_regions,
        effective_max_peers,
        path_profile,
        auto_mode,
    );
    let effective_prefer_region_diversity =
        effective_prefer_region_diversity(policy.prefer_region_diversity, path_profile, auto_mode);
    let effective_max_selected_per_region = effective_max_selected_per_region(
        policy.max_selected_per_region,
        effective_max_peers,
        path_profile,
        auto_mode,
    );
    let effective_reliability = effective_min_reliability(
        policy.require_min_reliability,
        path_profile,
        avg_reliability_score,
        auto_mode,
    );
    let effective_load = effective_max_load(
        policy.max_load_score,
        path_profile,
        avg_load_score,
        auto_mode,
    );

    EffectivePlanPolicy {
        effective_max_peers,
        effective_min_distinct_regions,
        effective_prefer_region_diversity,
        effective_max_selected_per_region,
        effective_reliability,
        effective_load,
    }
}
