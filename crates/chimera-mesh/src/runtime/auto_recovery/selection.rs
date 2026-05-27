use super::*;

pub(crate) fn select_peers_by_strategy(
    candidates: Vec<MeshPeerState>,
    prefer_region_diversity: bool,
    effective_max_peers: usize,
    effective_max_selected_per_region: usize,
) -> (Vec<MeshPeerState>, usize) {
    if prefer_region_diversity {
        select_with_region_diversity(
            candidates,
            effective_max_peers,
            effective_max_selected_per_region,
        )
    } else {
        select_by_score_with_region_cap(
            candidates,
            effective_max_peers,
            effective_max_selected_per_region,
        )
    }
}
