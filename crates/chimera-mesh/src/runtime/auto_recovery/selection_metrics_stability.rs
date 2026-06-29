use super::*;
use std::fmt::Write;

pub(crate) fn build_selected_stability_metrics(
    selected_peers: &[MeshPeerState],
    peer_meta: &BTreeMap<String, MeshPeerMeta>,
    max_replacements_per_window: u64,
) -> SelectedStabilityMetrics {
    let mut selected_peer_stability = String::with_capacity(selected_peers.len().saturating_mul(32));
    let mut selected_effective_thresholds =
        String::with_capacity(selected_peers.len().saturating_mul(16));
    let mut selected_replacement_decisions =
        String::with_capacity(selected_peers.len().saturating_mul(48));
    let mut selected_replacement_budget =
        String::with_capacity(selected_peers.len().saturating_mul(16));
    let mut effective_threshold_min: Option<i32> = None;
    let mut effective_threshold_max: Option<i32> = None;
    let mut stability_updates_total = 0u64;
    let mut stability_replacements_total = 0u64;
    let mut stability_holds_total = 0u64;
    let mut stability_degraded_total = 0u64;
    let mut stability_churn_blocks_total = 0u64;
    let mut stability_threshold_blocks_total = 0u64;
    let mut replacement_budget_remaining_total = 0u64;

    for (idx, peer) in selected_peers.iter().enumerate() {
        if let Some(meta) = peer_meta.get(&peer.node_id) {
            if !selected_peer_stability.is_empty() {
                selected_peer_stability.push(',');
                selected_effective_thresholds.push(',');
                selected_replacement_decisions.push(',');
                selected_replacement_budget.push(',');
            }

            stability_updates_total = stability_updates_total.saturating_add(meta.update_events);
            stability_replacements_total = stability_replacements_total
                .saturating_add(meta.replacement_events);
            stability_holds_total = stability_holds_total.saturating_add(meta.hold_events);
            stability_degraded_total = stability_degraded_total.saturating_add(meta.degraded_events);
            stability_churn_blocks_total = stability_churn_blocks_total
                .saturating_add(meta.churn_block_events);
            stability_threshold_blocks_total = stability_threshold_blocks_total
                .saturating_add(meta.threshold_block_events);

            push_redacted_peer_label(&mut selected_peer_stability, idx);
            selected_peer_stability.push_str(":u");
            let _ = write!(&mut selected_peer_stability, "{}", meta.update_events);
            selected_peer_stability.push_str(":r");
            let _ = write!(&mut selected_peer_stability, "{}", meta.replacement_events);
            selected_peer_stability.push_str(":h");
            let _ = write!(&mut selected_peer_stability, "{}", meta.hold_events);
            selected_peer_stability.push_str(":d");
            let _ = write!(&mut selected_peer_stability, "{}", meta.degraded_events);

            push_redacted_peer_label(&mut selected_effective_thresholds, idx);
            selected_effective_thresholds.push(':');
            let _ = write!(
                &mut selected_effective_thresholds,
                "{}",
                meta.last_effective_replacement_threshold
            );

            push_redacted_peer_label(&mut selected_replacement_decisions, idx);
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

            let remaining = max_replacements_per_window.saturating_sub(meta.replacement_events);
            replacement_budget_remaining_total =
                replacement_budget_remaining_total.saturating_add(remaining);
            push_redacted_peer_label(&mut selected_replacement_budget, idx);
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
        }
    }

    let replacement_hold_ratio_pct = stability_replacements_total
        .saturating_mul(100)
        .checked_div(stability_updates_total)
        .unwrap_or(0);

    SelectedStabilityMetrics {
        selected_peer_stability,
        selected_effective_replacement_thresholds: selected_effective_thresholds,
        selected_replacement_decisions,
        selected_replacement_budget_remaining: selected_replacement_budget,
        effective_replacement_threshold_min: effective_threshold_min.unwrap_or(0),
        effective_replacement_threshold_max: effective_threshold_max.unwrap_or(0),
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

fn push_redacted_peer_label(out: &mut String, index: usize) {
    out.push_str("peer#");
    let _ = write!(out, "{}", index + 1);
}
