use super::*;
use crate::runtime::connect_retry_profile::build_connect_backoff_profile;
use std::fmt::Write;

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
    pub(super) selected_stability: String,
    pub(super) selected_effective_thresholds: String,
    pub(super) selected_replacement_decisions: String,
    pub(super) selected_replacement_budget: String,
    pub(super) effective_threshold_min: i32,
    pub(super) effective_threshold_max: i32,
    pub(super) stability_updates_total: u64,
    pub(super) stability_replacements_total: u64,
    pub(super) stability_holds_total: u64,
    pub(super) stability_degraded_total: u64,
    pub(super) stability_churn_blocks_total: u64,
    pub(super) stability_threshold_blocks_total: u64,
    pub(super) replacement_hold_ratio_pct: u64,
    pub(super) replacement_budget_remaining_total: u64,
}

pub(super) fn build_peer_selection_summary(
    runtime: &MeshRuntime,
    selected_peers: &[MeshPeerState],
    connect_fallback_ports: &[u16],
) -> PeerSelectionSummary {
    let selected_peer_count = selected_peers.len();
    let mut selected_peer_ids = String::with_capacity(selected_peer_count.saturating_mul(16));
    let mut selected_peer_regions = String::with_capacity(selected_peer_count.saturating_mul(16));
    let mut selected_peer_endpoints = String::with_capacity(selected_peer_count.saturating_mul(24));
    let mut selected_peer_connect_priority =
        String::with_capacity(selected_peer_count.saturating_mul(24));
    let mut selected_peer_connect_retry_plan =
        String::with_capacity(selected_peer_count.saturating_mul(96));
    let mut selected_peer_scores = String::with_capacity(selected_peer_count.saturating_mul(24));
    let mut selected_region_counts = Vec::with_capacity(selected_peer_count.min(8));
    let mut selected_stability = String::with_capacity(selected_peer_count.saturating_mul(32));
    let mut selected_effective_thresholds =
        String::with_capacity(selected_peer_count.saturating_mul(16));
    let mut selected_replacement_decisions =
        String::with_capacity(selected_peer_count.saturating_mul(48));
    let mut selected_replacement_budget =
        String::with_capacity(selected_peer_count.saturating_mul(16));
    let mut selected_score_sum = 0i32;
    let mut selected_reliability_sum = 0usize;
    let mut selected_load_sum = 0usize;
    let mut effective_threshold_min: Option<i32> = None;
    let mut effective_threshold_max: Option<i32> = None;
    let mut stability_updates_total = 0u64;
    let mut stability_replacements_total = 0u64;
    let mut stability_holds_total = 0u64;
    let mut stability_degraded_total = 0u64;
    let mut stability_churn_blocks_total = 0u64;
    let mut stability_threshold_blocks_total = 0u64;
    let mut replacement_budget_remaining_total = 0u64;
    let replacements_limit = runtime.table_policy.max_replacements_per_window;
    let fallback_port_state = if connect_fallback_ports.is_empty() {
        "none"
    } else {
        "configured"
    };

    for (idx, selected_peer) in selected_peers.iter().enumerate() {
        if idx > 0 {
            selected_peer_ids.push(',');
            selected_peer_regions.push(',');
            selected_peer_endpoints.push(',');
            selected_peer_connect_priority.push(',');
            selected_peer_connect_retry_plan.push(',');
            selected_peer_scores.push(',');
        }

        super::format::push_redacted_peer_label(&mut selected_peer_ids, idx);
        selected_peer_regions.push_str(selected_peer.region.as_str());
        super::format::push_redacted_endpoint_label(&mut selected_peer_endpoints, idx);
        push_connect_priority_label(&mut selected_peer_connect_priority, idx);
        push_connect_retry_plan_entry(
            &mut selected_peer_connect_retry_plan,
            idx,
            selected_peer_count,
            fallback_port_state,
        );

        super::format::push_redacted_peer_label(&mut selected_peer_scores, idx);
        selected_peer_scores.push(':');
        let _ = write!(
            &mut selected_peer_scores,
            "{}",
            selected_peer.selection_score
        );
        selected_score_sum += selected_peer.selection_score;
        selected_reliability_sum += selected_peer.reliability_score as usize;
        selected_load_sum += selected_peer.load_score as usize;
        push_selected_region_count(&mut selected_region_counts, selected_peer.region.as_str());

        if let Some(meta) = runtime.peer_meta.get(&selected_peer.node_id) {
            if !selected_stability.is_empty() {
                selected_stability.push(',');
                selected_effective_thresholds.push(',');
                selected_replacement_decisions.push(',');
                selected_replacement_budget.push(',');
            }
            stability_updates_total = stability_updates_total.saturating_add(meta.update_events);
            stability_replacements_total =
                stability_replacements_total.saturating_add(meta.replacement_events);
            stability_holds_total = stability_holds_total.saturating_add(meta.hold_events);
            stability_degraded_total =
                stability_degraded_total.saturating_add(meta.degraded_events);
            stability_churn_blocks_total =
                stability_churn_blocks_total.saturating_add(meta.churn_block_events);
            stability_threshold_blocks_total =
                stability_threshold_blocks_total.saturating_add(meta.threshold_block_events);

            super::format::push_redacted_peer_label(&mut selected_effective_thresholds, idx);
            selected_effective_thresholds.push(':');
            let _ = write!(
                &mut selected_effective_thresholds,
                "{}",
                meta.last_effective_replacement_threshold
            );

            super::format::push_redacted_peer_label(&mut selected_replacement_decisions, idx);
            selected_replacement_decisions.push_str(":replace");
            let _ = write!(
                &mut selected_replacement_decisions,
                "{}",
                meta.replacement_events
            );
            selected_replacement_decisions.push_str(":hold");
            let _ = write!(&mut selected_replacement_decisions, "{}", meta.hold_events);
            selected_replacement_decisions.push_str(":churn_block");
            let _ = write!(
                &mut selected_replacement_decisions,
                "{}",
                meta.churn_block_events
            );
            selected_replacement_decisions.push_str(":threshold_block");
            let _ = write!(
                &mut selected_replacement_decisions,
                "{}",
                meta.threshold_block_events
            );

            let remaining = replacements_limit.saturating_sub(meta.replacement_events);
            replacement_budget_remaining_total =
                replacement_budget_remaining_total.saturating_add(remaining);
            super::format::push_redacted_peer_label(&mut selected_replacement_budget, idx);
            selected_replacement_budget.push(':');
            let _ = write!(&mut selected_replacement_budget, "{}", remaining);

            effective_threshold_min = Some(match effective_threshold_min {
                Some(current) => current.min(meta.last_effective_replacement_threshold),
                None => meta.last_effective_replacement_threshold,
            });
            effective_threshold_max = Some(match effective_threshold_max {
                Some(current) => current.max(meta.last_effective_replacement_threshold),
                None => meta.last_effective_replacement_threshold,
            });

            super::format::push_redacted_peer_label(&mut selected_stability, idx);
            selected_stability.push_str(":u");
            let _ = write!(&mut selected_stability, "{}", meta.update_events);
            selected_stability.push_str(":r");
            let _ = write!(&mut selected_stability, "{}", meta.replacement_events);
            selected_stability.push_str(":h");
            let _ = write!(&mut selected_stability, "{}", meta.hold_events);
            selected_stability.push_str(":d");
            let _ = write!(&mut selected_stability, "{}", meta.degraded_events);
        }
    }

    let replacement_hold_ratio_pct = stability_replacements_total
        .saturating_mul(100)
        .checked_div(stability_updates_total)
        .unwrap_or(0);

    PeerSelectionSummary {
        selected_peer_ids,
        selected_peer_regions,
        selected_peer_endpoints,
        selected_peer_connect_priority,
        selected_peer_connect_retry_plan,
        selected_peer_connect_backoff_profile: build_connect_backoff_profile(selected_peer_count),
        selected_peer_scores,
        selected_score_sum,
        selected_reliability_avg: average_selected_metric_from_sum(
            selected_reliability_sum,
            selected_peer_count,
        ),
        selected_load_avg: average_selected_metric_from_sum(selected_load_sum, selected_peer_count),
        selected_region_counts: format_selected_region_counts(selected_region_counts),
        selected_stability,
        selected_effective_thresholds,
        selected_replacement_decisions,
        selected_replacement_budget,
        effective_threshold_min: effective_threshold_min.unwrap_or(0),
        effective_threshold_max: effective_threshold_max.unwrap_or(0),
        stability_updates_total,
        stability_replacements_total,
        stability_holds_total,
        stability_degraded_total,
        stability_churn_blocks_total,
        stability_threshold_blocks_total,
        replacement_hold_ratio_pct,
        replacement_budget_remaining_total,
    }
}

fn average_selected_metric_from_sum(selected_sum: usize, selected_peer_count: usize) -> usize {
    selected_sum.checked_div(selected_peer_count).unwrap_or(0)
}

fn push_connect_priority_label(out: &mut String, index: usize) {
    let peer_number = index + 1;
    let _ = write!(out, "{peer_number}:peer#{peer_number}@<redacted>");
}

fn push_connect_retry_plan_entry(
    out: &mut String,
    index: usize,
    selected_peer_count: usize,
    fallback_port_state: &str,
) {
    out.push_str("peer#");
    let _ = write!(out, "{}", index + 1);
    out.push_str(
        "@<redacted>:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=<redacted>;fallback_ports=",
    );
    out.push_str(fallback_port_state);
    if index + 1 < selected_peer_count {
        out.push_str(";fallback:peer#");
        let _ = write!(out, "{}", index + 2);
        out.push_str("@<redacted>");
    }
}

fn push_selected_region_count(selected_region_counts: &mut Vec<(String, usize)>, region: &str) {
    let normalized_region = normalize_region_key(region);
    if let Some((_, count)) = selected_region_counts
        .iter_mut()
        .find(|(region, _)| region == &normalized_region)
    {
        *count += 1;
    } else {
        selected_region_counts.push((normalized_region, 1));
    }
}

fn format_selected_region_counts(mut selected_region_counts: Vec<(String, usize)>) -> String {
    selected_region_counts.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let mut out = String::with_capacity(selected_region_counts.len().saturating_mul(16));
    for (index, (region, count)) in selected_region_counts.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&region);
        out.push(':');
        let _ = write!(out, "{}", count);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_peer_selection_summary_sorts_region_counts_by_normalized_region_not_insertion_order() {
        let runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
            .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
        let selected_peers = vec![
            MeshPeerState {
                node_id: "node-us".to_string(),
                endpoint: "198.51.100.31:443".to_string(),
                region: "us".to_string(),
                reliability_score: 90,
                load_score: 20,
                latency_ms: None,
                throughput_mbps: None,
                selection_score: 10,
            },
            MeshPeerState {
                node_id: "node-eu".to_string(),
                endpoint: "198.51.100.32:443".to_string(),
                region: "EU".to_string(),
                reliability_score: 91,
                load_score: 21,
                latency_ms: None,
                throughput_mbps: None,
                selection_score: 11,
            },
        ];

        let summary = build_peer_selection_summary(&runtime, &selected_peers, &[]);

        assert_eq!(summary.selected_peer_regions, "us,EU");
        assert_eq!(summary.selected_region_counts, "eu:1,us:1");
    }
}
