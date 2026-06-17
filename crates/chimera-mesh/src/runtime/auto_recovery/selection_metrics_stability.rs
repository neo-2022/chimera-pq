use super::*;

pub(crate) fn build_selected_stability_metrics(
    selected_peers: &[MeshPeerState],
    peer_meta: &BTreeMap<String, MeshPeerMeta>,
    max_replacements_per_window: u64,
) -> SelectedStabilityMetrics {
    let mut aggregate = StabilityAggregate {
        selected_stability: Vec::new(),
        selected_effective_thresholds: Vec::new(),
        selected_replacement_decisions: Vec::new(),
        selected_replacement_budget: Vec::new(),
        effective_threshold_min: None,
        effective_threshold_max: None,
        stability_updates_total: 0,
        stability_replacements_total: 0,
        stability_holds_total: 0,
        stability_degraded_total: 0,
        stability_churn_blocks_total: 0,
        stability_threshold_blocks_total: 0,
        replacement_budget_remaining_total: 0,
    };

    for (idx, peer) in selected_peers.iter().enumerate() {
        if let Some(meta) = peer_meta.get(&peer.node_id) {
            accumulate_selected_peer_stability(
                &mut aggregate,
                idx,
                meta,
                max_replacements_per_window,
            );
        }
    }

    let replacement_hold_ratio_pct = aggregate
        .stability_replacements_total
        .saturating_mul(100)
        .checked_div(aggregate.stability_updates_total)
        .unwrap_or(0);

    SelectedStabilityMetrics {
        selected_peer_stability: aggregate.selected_stability.join(","),
        selected_effective_replacement_thresholds: aggregate
            .selected_effective_thresholds
            .join(","),
        selected_replacement_decisions: aggregate.selected_replacement_decisions.join(","),
        selected_replacement_budget_remaining: aggregate.selected_replacement_budget.join(","),
        effective_replacement_threshold_min: aggregate.effective_threshold_min.unwrap_or(0),
        effective_replacement_threshold_max: aggregate.effective_threshold_max.unwrap_or(0),
        stability_updates_total: aggregate.stability_updates_total,
        stability_replacements_total: aggregate.stability_replacements_total,
        stability_holds_total: aggregate.stability_holds_total,
        stability_degraded_total: aggregate.stability_degraded_total,
        stability_churn_blocks_total: aggregate.stability_churn_blocks_total,
        stability_threshold_blocks_total: aggregate.stability_threshold_blocks_total,
        replacement_hold_ratio_pct,
        replacement_budget_remaining_total: aggregate.replacement_budget_remaining_total,
    }
}

pub(crate) fn accumulate_selected_peer_stability(
    aggregate: &mut StabilityAggregate,
    peer_index: usize,
    meta: &MeshPeerMeta,
    max_replacements_per_window: u64,
) {
    aggregate.stability_updates_total = aggregate
        .stability_updates_total
        .saturating_add(meta.update_events);
    aggregate.stability_replacements_total = aggregate
        .stability_replacements_total
        .saturating_add(meta.replacement_events);
    aggregate.stability_holds_total = aggregate
        .stability_holds_total
        .saturating_add(meta.hold_events);
    aggregate.stability_degraded_total = aggregate
        .stability_degraded_total
        .saturating_add(meta.degraded_events);
    aggregate.stability_churn_blocks_total = aggregate
        .stability_churn_blocks_total
        .saturating_add(meta.churn_block_events);
    aggregate.stability_threshold_blocks_total = aggregate
        .stability_threshold_blocks_total
        .saturating_add(meta.threshold_block_events);
    aggregate
        .selected_effective_thresholds
        .push(format_selected_effective_threshold(peer_index, meta));
    aggregate
        .selected_replacement_decisions
        .push(format_selected_replacement_decision(peer_index, meta));
    let remaining = max_replacements_per_window.saturating_sub(meta.replacement_events);
    aggregate.replacement_budget_remaining_total = aggregate
        .replacement_budget_remaining_total
        .saturating_add(remaining);
    aggregate
        .selected_replacement_budget
        .push(format_selected_replacement_budget(peer_index, remaining));
    aggregate.effective_threshold_min = Some(match aggregate.effective_threshold_min {
        Some(current) => current.min(meta.last_effective_replacement_threshold),
        None => meta.last_effective_replacement_threshold,
    });
    aggregate.effective_threshold_max = Some(match aggregate.effective_threshold_max {
        Some(current) => current.max(meta.last_effective_replacement_threshold),
        None => meta.last_effective_replacement_threshold,
    });
    aggregate
        .selected_stability
        .push(format_selected_stability(peer_index, meta));
}

pub(crate) fn format_selected_effective_threshold(
    peer_index: usize,
    meta: &MeshPeerMeta,
) -> String {
    format!(
        "{}:{}",
        redacted_peer_label(peer_index),
        meta.last_effective_replacement_threshold
    )
}

pub(crate) fn format_selected_replacement_decision(
    peer_index: usize,
    meta: &MeshPeerMeta,
) -> String {
    format!(
        "{}:replace{}:hold{}:churn_block{}:threshold_block{}",
        redacted_peer_label(peer_index),
        meta.replacement_events,
        meta.hold_events,
        meta.churn_block_events,
        meta.threshold_block_events
    )
}

pub(crate) fn format_selected_replacement_budget(peer_index: usize, remaining: u64) -> String {
    format!("{}:{}", redacted_peer_label(peer_index), remaining)
}

pub(crate) fn format_selected_stability(peer_index: usize, meta: &MeshPeerMeta) -> String {
    format!(
        "{}:u{}:r{}:h{}:d{}",
        redacted_peer_label(peer_index),
        meta.update_events,
        meta.replacement_events,
        meta.hold_events,
        meta.degraded_events
    )
}

fn redacted_peer_label(index: usize) -> String {
    format!("peer#{}", index + 1)
}
