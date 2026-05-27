use super::*;

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
    let mut selected_stability = Vec::new();
    let mut stability_updates_total = 0u64;
    let mut stability_replacements_total = 0u64;
    let mut stability_holds_total = 0u64;
    let mut stability_degraded_total = 0u64;
    let mut stability_churn_blocks_total = 0u64;
    let mut stability_threshold_blocks_total = 0u64;
    let mut selected_effective_thresholds = Vec::new();
    let mut selected_replacement_decisions = Vec::new();
    let mut selected_replacement_budget = Vec::new();
    let mut effective_threshold_min: Option<i32> = None;
    let mut effective_threshold_max: Option<i32> = None;
    let mut replacement_budget_remaining_total = 0u64;

    for peer in selected_peers {
        if let Some(meta) = runtime.peer_meta.get(&peer.node_id) {
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
            selected_effective_thresholds.push(format!(
                "{}:{}",
                peer.node_id, meta.last_effective_replacement_threshold
            ));
            selected_replacement_decisions.push(format!(
                "{}:replace{}:hold{}:churn_block{}:threshold_block{}",
                peer.node_id,
                meta.replacement_events,
                meta.hold_events,
                meta.churn_block_events,
                meta.threshold_block_events
            ));
            let remaining = runtime
                .table_policy
                .max_replacements_per_window
                .saturating_sub(meta.replacement_events);
            replacement_budget_remaining_total =
                replacement_budget_remaining_total.saturating_add(remaining);
            selected_replacement_budget.push(format!("{}:{}", peer.node_id, remaining));
            effective_threshold_min = Some(match effective_threshold_min {
                Some(current) => current.min(meta.last_effective_replacement_threshold),
                None => meta.last_effective_replacement_threshold,
            });
            effective_threshold_max = Some(match effective_threshold_max {
                Some(current) => current.max(meta.last_effective_replacement_threshold),
                None => meta.last_effective_replacement_threshold,
            });
            selected_stability.push(format!(
                "{}:u{}:r{}:h{}:d{}",
                peer.node_id,
                meta.update_events,
                meta.replacement_events,
                meta.hold_events,
                meta.degraded_events
            ));
        }
    }

    let replacement_hold_ratio_pct = stability_replacements_total
        .saturating_mul(100)
        .checked_div(stability_updates_total)
        .unwrap_or(0);

    StabilityMetrics {
        selected_stability: selected_stability.join(","),
        selected_effective_thresholds: selected_effective_thresholds.join(","),
        selected_replacement_decisions: selected_replacement_decisions.join(","),
        selected_replacement_budget: selected_replacement_budget.join(","),
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
