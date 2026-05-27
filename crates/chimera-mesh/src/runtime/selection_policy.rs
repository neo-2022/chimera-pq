#[path = "selection_policy_select.rs"]
mod select;
#[path = "selection_policy_spread.rs"]
mod spread;
#[path = "selection_policy_validate.rs"]
mod validate;

use crate::model::MeshPeerState;
use crate::policy::MeshPathProfile;

pub(super) fn select_with_region_diversity(
    candidates: Vec<MeshPeerState>,
    max_peers: usize,
    max_selected_per_region: usize,
) -> (Vec<MeshPeerState>, usize) {
    select::select_with_region_diversity(candidates, max_peers, max_selected_per_region)
}

pub(super) fn select_by_score_with_region_cap(
    candidates: Vec<MeshPeerState>,
    max_peers: usize,
    max_selected_per_region: usize,
) -> (Vec<MeshPeerState>, usize) {
    select::select_by_score_with_region_cap(candidates, max_peers, max_selected_per_region)
}

pub(super) fn normalize_region_key(value: &str) -> String {
    validate::normalize_region_key(value)
}

pub(super) fn validate_source_name(source: &str, label: &str) -> Result<(), String> {
    validate::validate_source_name(source, label)
}

pub(super) fn validate_runtime_node_id(node_id: &str, label: &str) -> Result<(), String> {
    validate::validate_runtime_node_id(node_id, label)
}

pub(super) fn apply_resilient_region_spread_bonus(
    candidates: &mut [MeshPeerState],
    profile: MeshPathProfile,
    weight: u8,
) -> (bool, i32) {
    spread::apply_resilient_region_spread_bonus(candidates, profile, weight)
}
