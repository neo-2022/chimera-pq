use super::*;
use crate::runtime::connect_retry_profile::{
    build_connect_backoff_profile, build_connect_priority, build_connect_retry_plan,
};

pub(super) struct PeerSelectionSummary {
    pub(super) selected_peer_ids: String,
    pub(super) selected_peer_regions: String,
    pub(super) selected_peer_endpoints: String,
    pub(super) selected_peer_connect_priority: String,
    pub(super) selected_peer_connect_retry_plan: String,
    pub(super) selected_peer_connect_backoff_profile: String,
    pub(super) selected_peer_scores: String,
    pub(super) selected_score_sum: i32,
    pub(super) selected_reliability_avg: usize,
    pub(super) selected_load_avg: usize,
    pub(super) selected_region_counts: String,
}

pub(super) fn build_peer_selection_summary(
    selected_peers: &[MeshPeerState],
    connect_fallback_ports: &[u16],
) -> PeerSelectionSummary {
    PeerSelectionSummary {
        selected_peer_ids: join_selected(selected_peers, |peer| peer.node_id.as_str().to_string()),
        selected_peer_regions: join_selected(selected_peers, |peer| {
            peer.region.as_str().to_string()
        }),
        selected_peer_endpoints: join_selected(selected_peers, |peer| {
            peer.endpoint.as_str().to_string()
        }),
        selected_peer_connect_priority: build_connect_priority(selected_peers),
        selected_peer_connect_retry_plan: build_connect_retry_plan(
            selected_peers,
            connect_fallback_ports,
        ),
        selected_peer_connect_backoff_profile: build_connect_backoff_profile(selected_peers.len()),
        selected_peer_scores: join_selected(selected_peers, |peer| {
            format!("{}:{}", peer.node_id, peer.selection_score)
        }),
        selected_score_sum: selected_peers.iter().map(|peer| peer.selection_score).sum(),
        selected_reliability_avg: average_selected_metric(selected_peers, |peer| {
            peer.reliability_score as usize
        }),
        selected_load_avg: average_selected_metric(selected_peers, |peer| peer.load_score as usize),
        selected_region_counts: build_selected_region_counts(selected_peers),
    }
}

fn join_selected(
    selected_peers: &[MeshPeerState],
    map_value: impl Fn(&MeshPeerState) -> String,
) -> String {
    selected_peers
        .iter()
        .map(map_value)
        .collect::<Vec<_>>()
        .join(",")
}

fn average_selected_metric(
    selected_peers: &[MeshPeerState],
    map_value: impl Fn(&MeshPeerState) -> usize,
) -> usize {
    if selected_peers.is_empty() {
        0
    } else {
        selected_peers.iter().map(map_value).sum::<usize>() / selected_peers.len()
    }
}

fn build_selected_region_counts(selected_peers: &[MeshPeerState]) -> String {
    let mut selected_region_counts_map: BTreeMap<String, usize> = BTreeMap::new();
    for peer in selected_peers {
        *selected_region_counts_map
            .entry(normalize_region_key(&peer.region))
            .or_insert(0) += 1;
    }
    selected_region_counts_map
        .into_iter()
        .map(|(region, count)| format!("{region}:{count}"))
        .collect::<Vec<_>>()
        .join(",")
}
