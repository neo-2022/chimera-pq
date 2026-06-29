use super::*;
use std::fmt::Write;

pub(super) struct StabilityMetrics {
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

pub(super) fn build_stability_metrics(
    runtime: &MeshRuntime,
    selected_peers: &[MeshPeerState],
) -> StabilityMetrics {
    let mut selected_stability = String::with_capacity(selected_peers.len().saturating_mul(32));
    let replacements_limit = runtime.table_policy.max_replacements_per_window;
    let mut stability_updates_total = 0u64;
    let mut stability_replacements_total = 0u64;
    let mut stability_holds_total = 0u64;
    let mut stability_degraded_total = 0u64;
    let mut stability_churn_blocks_total = 0u64;
    let mut stability_threshold_blocks_total = 0u64;
    let mut selected_effective_thresholds =
        String::with_capacity(selected_peers.len().saturating_mul(16));
    let mut selected_replacement_decisions =
        String::with_capacity(selected_peers.len().saturating_mul(48));
    let mut selected_replacement_budget =
        String::with_capacity(selected_peers.len().saturating_mul(16));
    let mut effective_threshold_min: Option<i32> = None;
    let mut effective_threshold_max: Option<i32> = None;
    let mut replacement_budget_remaining_total = 0u64;

    for (idx, peer) in selected_peers.iter().enumerate() {
        if let Some(meta) = runtime.peer_meta.get(&peer.node_id) {
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
            let remaining = replacements_limit.saturating_sub(meta.replacement_events);
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
            push_redacted_peer_label(&mut selected_stability, idx);
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

    StabilityMetrics {
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

fn push_redacted_peer_label(out: &mut String, index: usize) {
    out.push_str("peer#");
    let _ = write!(out, "{}", index + 1);
}
