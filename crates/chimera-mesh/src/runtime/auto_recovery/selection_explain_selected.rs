use super::*;

pub(crate) fn append_selected_peer_metrics_explain(
    explain: &mut Vec<String>,
    metrics: &SelectedPeerMetrics,
) {
    append_selected_peer_identity_explain(explain, metrics);
    append_selected_peer_aggregate_explain(explain, metrics);
}

pub(crate) fn append_selected_peer_identity_explain(
    explain: &mut Vec<String>,
    metrics: &SelectedPeerMetrics,
) {
    explain.push(format!("selected_peer_ids={}", metrics.selected_peer_ids));
    explain.push(format!(
        "selected_peer_regions={}",
        metrics.selected_peer_regions
    ));
    explain.push(format!(
        "selected_peer_endpoints={}",
        metrics.selected_peer_endpoints
    ));
    explain.push(format!(
        "selected_peer_connect_priority={}",
        metrics.selected_peer_connect_priority
    ));
    explain.push(format!(
        "selected_peer_connect_retry_plan={}",
        metrics.selected_peer_connect_retry_plan
    ));
    explain.push(format!(
        "selected_peer_connect_backoff_profile={}",
        metrics.selected_peer_connect_backoff_profile
    ));
    explain.push(format!(
        "selected_peer_scores={}",
        metrics.selected_peer_scores
    ));
}

pub(crate) fn append_selected_peer_aggregate_explain(
    explain: &mut Vec<String>,
    metrics: &SelectedPeerMetrics,
) {
    explain.push(format!("selected_score_sum={}", metrics.selected_score_sum));
    explain.push(format!(
        "selected_reliability_avg={}",
        metrics.selected_reliability_avg
    ));
    explain.push(format!("selected_load_avg={}", metrics.selected_load_avg));
    explain.push(format!(
        "selected_region_counts={}",
        metrics.selected_region_counts
    ));
}
