use super::*;
use crate::runtime::connect_retry_profile::{
    build_connect_backoff_profile, build_connect_priority, build_connect_retry_plan,
};
use std::collections::BTreeMap;
use std::fmt::Write;

pub(crate) fn build_selected_peer_metrics(
    selected_peers: &[MeshPeerState],
    stats: &CandidateStats,
    region_cap_rejections: usize,
    effective_max_peers: usize,
) -> SelectedPeerMetrics {
    let selected_peer_parts = build_selected_peer_parts(selected_peers);
    let selected_score_sum = selected_peer_parts.selected_score_sum;
    let selected_reliability_avg = average_selected_metric_from_sum(
        selected_peer_parts.selected_reliability_sum,
        selected_peers.len(),
    );
    let selected_load_avg = average_selected_metric_from_sum(
        selected_peer_parts.selected_load_sum,
        selected_peers.len(),
    );
    let selected_region_counts =
        format_selected_region_counts(selected_peer_parts.selected_region_counts);
    let candidates_selected = selected_peers.len();
    let counters = build_candidate_selection_counters(
        stats,
        candidates_selected,
        region_cap_rejections,
        effective_max_peers,
    );

    SelectedPeerMetrics {
        selected_peer_ids: selected_peer_parts.selected_peer_ids,
        selected_peer_regions: selected_peer_parts.selected_peer_regions,
        selected_peer_endpoints: selected_peer_parts.selected_peer_endpoints,
        selected_peer_connect_priority: build_connect_priority(selected_peers),
        selected_peer_connect_retry_plan: build_connect_retry_plan(
            selected_peers,
            &MeshPathPolicy::default_auto().connect_fallback_ports,
        ),
        selected_peer_connect_backoff_profile: build_connect_backoff_profile(selected_peers.len()),
        selected_peer_scores: selected_peer_parts.selected_peer_scores,
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
    let selected_peer_parts = build_selected_peer_parts(selected_peers);
    SelectedPeerStrings {
        ids: selected_peer_parts.selected_peer_ids,
        regions: selected_peer_parts.selected_peer_regions,
        endpoints: selected_peer_parts.selected_peer_endpoints,
        scores: selected_peer_parts.selected_peer_scores,
    }
}

fn build_selected_peer_parts(selected_peers: &[MeshPeerState]) -> SelectedPeerParts {
    let mut selected_peer_ids = String::with_capacity(selected_peers.len().saturating_mul(16));
    let mut selected_peer_regions = String::with_capacity(selected_peers.len().saturating_mul(16));
    let mut selected_peer_endpoints =
        String::with_capacity(selected_peers.len().saturating_mul(24));
    let mut selected_peer_scores = String::with_capacity(selected_peers.len().saturating_mul(24));
    let mut selected_region_counts_map: BTreeMap<String, usize> = BTreeMap::new();
    let mut selected_score_sum = 0;
    let mut selected_reliability_sum = 0;
    let mut selected_load_sum = 0;

    for (idx, peer) in selected_peers.iter().enumerate() {
        if idx > 0 {
            selected_peer_ids.push(',');
            selected_peer_regions.push(',');
            selected_peer_endpoints.push(',');
            selected_peer_scores.push(',');
        }

        push_redacted_peer_label(&mut selected_peer_ids, idx);
        selected_peer_regions.push_str(peer.region.as_str());
        push_redacted_endpoint_label(&mut selected_peer_endpoints, idx);
        push_redacted_peer_label(&mut selected_peer_scores, idx);
        selected_peer_scores.push(':');
        let _ = write!(&mut selected_peer_scores, "{}", peer.selection_score);

        selected_score_sum += peer.selection_score;
        selected_reliability_sum += peer.reliability_score as usize;
        selected_load_sum += peer.load_score as usize;
        *selected_region_counts_map
            .entry(normalize_region_key(&peer.region))
            .or_insert(0) += 1;
    }

    SelectedPeerParts {
        selected_peer_ids,
        selected_peer_regions,
        selected_peer_endpoints,
        selected_peer_scores,
        selected_score_sum,
        selected_reliability_sum,
        selected_load_sum,
        selected_region_counts: selected_region_counts_map,
    }
}

fn format_selected_region_counts(selected_region_counts_map: BTreeMap<String, usize>) -> String {
    let mut out = String::with_capacity(selected_region_counts_map.len().saturating_mul(16));
    for (idx, (region, count)) in selected_region_counts_map.into_iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(&region);
        out.push(':');
        let _ = write!(&mut out, "{}", count);
    }
    out
}

fn average_selected_metric_from_sum(selected_sum: usize, selected_peer_count: usize) -> usize {
    selected_sum.checked_div(selected_peer_count).unwrap_or(0)
}

pub(crate) fn build_selected_region_counts(selected_peers: &[MeshPeerState]) -> String {
    let mut selected_region_counts_map: BTreeMap<String, usize> = BTreeMap::new();
    for peer in selected_peers {
        *selected_region_counts_map
            .entry(normalize_region_key(&peer.region))
            .or_insert(0) += 1;
    }
    format_selected_region_counts(selected_region_counts_map)
}

struct SelectedPeerParts {
    selected_peer_ids: String,
    selected_peer_regions: String,
    selected_peer_endpoints: String,
    selected_peer_scores: String,
    selected_score_sum: i32,
    selected_reliability_sum: usize,
    selected_load_sum: usize,
    selected_region_counts: BTreeMap<String, usize>,
}

fn push_redacted_peer_label(out: &mut String, index: usize) {
    out.push_str("peer#");
    let _ = write!(out, "{}", index + 1);
}

fn push_redacted_endpoint_label(out: &mut String, index: usize) {
    out.push_str("endpoint#");
    let _ = write!(out, "{}", index + 1);
    out.push_str(":<redacted>");
}
