use super::*;
use crate::runtime::connect_retry_profile::{
    build_connect_backoff_profile, build_connect_priority, build_connect_retry_plan,
};

pub(crate) fn build_selected_peer_metrics(
    selected_peers: &[MeshPeerState],
    stats: &CandidateStats,
    region_cap_rejections: usize,
    effective_max_peers: usize,
) -> SelectedPeerMetrics {
    let selected_peer_strings = build_selected_peer_strings(selected_peers);
    let selected_score_sum: i32 = selected_peers.iter().map(|peer| peer.selection_score).sum();
    let selected_reliability_avg =
        average_selected_metric(selected_peers, |peer| peer.reliability_score as usize);
    let selected_load_avg = average_selected_metric(selected_peers, |peer| peer.load_score as usize);
    let selected_region_counts = build_selected_region_counts(selected_peers);
    let candidates_selected = selected_peers.len();
    let counters = build_candidate_selection_counters(
        stats,
        candidates_selected,
        region_cap_rejections,
        effective_max_peers,
    );

    SelectedPeerMetrics {
        selected_peer_ids: selected_peer_strings.ids,
        selected_peer_regions: selected_peer_strings.regions,
        selected_peer_endpoints: selected_peer_strings.endpoints,
        selected_peer_connect_priority: build_connect_priority(selected_peers),
        selected_peer_connect_retry_plan: build_connect_retry_plan(
            selected_peers,
            &MeshPathPolicy::default_auto().connect_fallback_ports,
        ),
        selected_peer_connect_backoff_profile: build_connect_backoff_profile(selected_peers.len()),
        selected_peer_scores: selected_peer_strings.scores,
        selected_score_sum,
        selected_reliability_avg,
        selected_load_avg,
        selected_region_counts,
        candidates_selected,
        candidates_considered: counters.candidates_considered,
        candidates_skipped_due_to_max_peers: counters.candidates_skipped_due_to_max_peers,
        candidates_skipped_due_to_limit: counters.candidates_skipped_due_to_limit,
        selection_utilization_pct: counters.selection_utilization_pct,
        selection_headroom: counters.selection_headroom,
    }
}

pub(crate) fn build_selected_peer_strings(selected_peers: &[MeshPeerState]) -> SelectedPeerStrings {
    let ids = selected_peers
        .iter()
        .map(|peer| peer.node_id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let regions = selected_peers
        .iter()
        .map(|peer| peer.region.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let endpoints = selected_peers
        .iter()
        .map(|peer| peer.endpoint.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let scores = selected_peers
        .iter()
        .map(|peer| format!("{}:{}", peer.node_id, peer.selection_score))
        .collect::<Vec<_>>()
        .join(",");
    SelectedPeerStrings {
        ids,
        regions,
        endpoints,
        scores,
    }
}

pub(crate) fn average_selected_metric(
    selected_peers: &[MeshPeerState],
    map_value: impl Fn(&MeshPeerState) -> usize,
) -> usize {
    if selected_peers.is_empty() {
        0
    } else {
        selected_peers.iter().map(map_value).sum::<usize>() / selected_peers.len()
    }
}

pub(crate) fn build_selected_region_counts(selected_peers: &[MeshPeerState]) -> String {
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
