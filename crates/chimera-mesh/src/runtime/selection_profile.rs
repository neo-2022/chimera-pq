use std::collections::BTreeMap;

use crate::model::{MeshPeerState, peer_priority};
use crate::policy::MeshPathProfile;

use super::MeshProfileState;

#[path = "selection_profile_core.rs"]
mod core;
#[path = "selection_profile_effective.rs"]
mod effective;

pub(super) fn resolve_path_profile(
    override_profile: Option<MeshPathProfile>,
    profile_state: &MeshProfileState,
    current_tick: u64,
    profile_hysteresis_ticks: u64,
    avg_load_score: u8,
    avg_reliability_score: u8,
) -> (MeshPathProfile, &'static str) {
    core::resolve_path_profile(
        override_profile,
        profile_state,
        current_tick,
        profile_hysteresis_ticks,
        avg_load_score,
        avg_reliability_score,
    )
}

pub(super) fn score_for_profile(peer: &MeshPeerState, profile: MeshPathProfile) -> i32 {
    core::score_for_profile(peer, profile)
}

pub(super) fn profile_label(profile: MeshPathProfile) -> &'static str {
    core::profile_label(profile)
}

pub(super) fn runtime_peer_signal_averages(peers: &BTreeMap<String, MeshPeerState>) -> (u8, u8) {
    core::runtime_peer_signal_averages(peers)
}

pub(super) fn effective_target_distinct_regions(
    configured_target: usize,
    max_entries: usize,
    profile: MeshPathProfile,
) -> (usize, &'static str) {
    effective::effective_target_distinct_regions(configured_target, max_entries, profile)
}

pub(super) fn effective_min_reliability(
    requested: u8,
    profile: MeshPathProfile,
    avg_reliability: u8,
    auto_mode: bool,
) -> u8 {
    effective::effective_min_reliability(requested, profile, avg_reliability, auto_mode)
}

pub(super) fn effective_max_peers(
    requested: usize,
    profile: MeshPathProfile,
    auto_mode: bool,
) -> usize {
    effective::effective_max_peers(requested, profile, auto_mode)
}

pub(super) fn effective_min_distinct_regions(
    requested: usize,
    effective_max_peers: usize,
    profile: MeshPathProfile,
    auto_mode: bool,
) -> usize {
    effective::effective_min_distinct_regions(requested, effective_max_peers, profile, auto_mode)
}

pub(super) fn effective_prefer_region_diversity(
    requested: bool,
    profile: MeshPathProfile,
    auto_mode: bool,
) -> bool {
    effective::effective_prefer_region_diversity(requested, profile, auto_mode)
}

pub(super) fn effective_max_selected_per_region(
    requested: usize,
    effective_max_peers: usize,
    profile: MeshPathProfile,
    auto_mode: bool,
) -> usize {
    effective::effective_max_selected_per_region(requested, effective_max_peers, profile, auto_mode)
}

pub(super) fn effective_max_load(
    requested: u8,
    profile: MeshPathProfile,
    avg_load: u8,
    auto_mode: bool,
) -> u8 {
    effective::effective_max_load(requested, profile, avg_load, auto_mode)
}
