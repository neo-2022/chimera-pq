use super::*;
#[path = "path_planner_selection_metrics_peer.rs"]
mod peer;

use peer::build_peer_selection_summary;
use std::fmt::Write;

pub(super) fn build_selection_metrics_impl(
    runtime: &MeshRuntime,
    policy: &MeshPathPolicy,
    selected_peers: &[MeshPeerState],
    stats: CandidateStats,
    region_cap_rejections: usize,
    effective_max_peers: usize,
) -> SelectionMetrics {
    let peer =
        build_peer_selection_summary(runtime, selected_peers, &policy.connect_fallback_ports);

    let candidates_selected = selected_peers.len();
    let candidates_rejected_total = stats.rejected_total();
    let candidates_considered = stats
        .accepted_count
        .saturating_add(candidates_rejected_total);
    let candidates_skipped_due_to_max_peers = stats
        .accepted_count
        .saturating_sub(candidates_selected)
        .saturating_sub(region_cap_rejections);
    let candidates_skipped_due_to_limit =
        candidates_skipped_due_to_max_peers.saturating_add(region_cap_rejections);
    let selection_utilization_pct = candidates_selected
        .saturating_mul(100)
        .checked_div(effective_max_peers)
        .unwrap_or(0);
    let selection_headroom = effective_max_peers.saturating_sub(candidates_selected);
    let selection_pressure_level = selection_pressure_level(
        candidates_selected,
        candidates_rejected_total,
        candidates_skipped_due_to_limit,
        selection_headroom,
    );
    let selection_pressure_score = selection_pressure_score(selection_pressure_level);
    let selection_pressure_dominant = selection_pressure_dominant(
        selection_pressure_level,
        stats,
        candidates_skipped_due_to_limit,
    );
    let selection_pressure_action_hint =
        selection_pressure_action_hint(selection_pressure_dominant);
    let selection_pressure_compact = selection_pressure_compact(
        selection_pressure_level,
        selection_pressure_score,
        selection_pressure_dominant,
        selection_pressure_action_hint,
    );
    let selection_pressure_reason = selection_pressure_reason(
        selection_pressure_level,
        selection_pressure_dominant,
        stats,
        candidates_skipped_due_to_limit,
        selection_headroom,
    );

    SelectionMetrics {
        selected_peer_ids: peer.selected_peer_ids,
        selected_peer_regions: peer.selected_peer_regions,
        selected_peer_endpoints: peer.selected_peer_endpoints,
        selected_peer_connect_priority: peer.selected_peer_connect_priority,
        selected_peer_connect_retry_plan: peer.selected_peer_connect_retry_plan,
        selected_peer_connect_backoff_profile: peer.selected_peer_connect_backoff_profile,
        selected_peer_scores: peer.selected_peer_scores,
        selected_score_sum: peer.selected_score_sum,
        selected_reliability_avg: peer.selected_reliability_avg,
        selected_load_avg: peer.selected_load_avg,
        selected_region_counts: peer.selected_region_counts,
        selected_stability: peer.selected_stability,
        selected_effective_thresholds: peer.selected_effective_thresholds,
        selected_replacement_decisions: peer.selected_replacement_decisions,
        selected_replacement_budget: peer.selected_replacement_budget,
        effective_threshold_min: peer.effective_threshold_min,
        effective_threshold_max: peer.effective_threshold_max,
        stability_updates_total: peer.stability_updates_total,
        stability_replacements_total: peer.stability_replacements_total,
        stability_holds_total: peer.stability_holds_total,
        stability_degraded_total: peer.stability_degraded_total,
        stability_churn_blocks_total: peer.stability_churn_blocks_total,
        stability_threshold_blocks_total: peer.stability_threshold_blocks_total,
        replacement_hold_ratio_pct: peer.replacement_hold_ratio_pct,
        replacement_budget_remaining_total: peer.replacement_budget_remaining_total,
        candidates_considered,
        candidates_selected,
        candidates_rejected_total,
        candidates_skipped_due_to_max_peers,
        candidates_skipped_due_to_limit,
        selection_utilization_pct,
        selection_headroom,
        selection_pressure_level,
        selection_pressure_score,
        selection_pressure_dominant,
        selection_pressure_action_hint,
        selection_pressure_compact,
        selection_pressure_reason,
    }
}

fn selection_pressure_level(
    selected: usize,
    rejected: usize,
    limit_skipped: usize,
    headroom: usize,
) -> &'static str {
    if selected == 0 {
        "empty"
    } else if headroom == 0 {
        "saturated"
    } else if limit_skipped > 0 {
        "limited"
    } else if rejected > 0 {
        "constrained"
    } else {
        "healthy"
    }
}

fn selection_pressure_score(level: &str) -> usize {
    match level {
        "empty" | "saturated" => 100,
        "limited" => 75,
        "constrained" => 50,
        "healthy" => 0,
        _ => 0,
    }
}

fn selection_pressure_reason(
    level: &str,
    dominant: &str,
    stats: CandidateStats,
    limit_skipped: usize,
    headroom: usize,
) -> String {
    let mut out = String::with_capacity(128);
    let _ = write!(
        out,
        "level={level};dominant={dominant};blocked={};health={};region={};reliability={};load={};limit_skipped={limit_skipped};headroom={headroom}",
        stats.rejected_blocked,
        stats.rejected_health,
        stats.rejected_region,
        stats.rejected_reliability,
        stats.rejected_load
    );
    out
}

fn selection_pressure_compact(
    level: &str,
    score: usize,
    dominant: &str,
    action_hint: &str,
) -> String {
    let mut out = String::with_capacity(64);
    let _ = write!(
        out,
        "level:{level};score:{score};dominant:{dominant};action:{action_hint}"
    );
    out
}

fn selection_pressure_dominant(
    level: &str,
    stats: CandidateStats,
    limit_skipped: usize,
) -> &'static str {
    if level == "saturated" {
        "capacity"
    } else if limit_skipped > 0 {
        "limit"
    } else {
        dominant_rejection_reason(stats)
    }
}

fn dominant_rejection_reason(stats: CandidateStats) -> &'static str {
    let mut dominant = "none";
    let mut max_count = 0usize;
    for (reason, count) in [
        ("blocked", stats.rejected_blocked),
        ("health", stats.rejected_health),
        ("region", stats.rejected_region),
        ("reliability", stats.rejected_reliability),
        ("load", stats.rejected_load),
    ] {
        if count >= max_count && count > 0 {
            dominant = reason;
            max_count = count;
        }
    }
    dominant
}

fn selection_pressure_action_hint(dominant: &str) -> &'static str {
    match dominant {
        "capacity" => "capacity_full",
        "limit" => "max_peer_limit",
        "blocked" => "unblock_peer_policy",
        "health" => "wait_or_recover_health",
        "region" => "relax_region_policy",
        "reliability" => "lower_reliability_threshold",
        "load" => "raise_load_threshold",
        _ => "none",
    }
}
