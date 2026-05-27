use crate::nodes_model::{MeshNode, MeshNodeReasonCode, MeshNodeScoreBreakdown};
use crate::nodes_policy::{MeshNodesPolicy, clamp_score, clamp01};

pub fn compute_mesh_node_score(
    node: &MeshNode,
    policy: &MeshNodesPolicy,
) -> MeshNodeScoreBreakdown {
    let thresholds = &policy.score.thresholds;
    let weights = &policy.score.weights;
    let latency_q = node
        .latency_ms
        .map(|value| {
            quality_lower_is_better(value, thresholds.latency_good_ms, thresholds.latency_bad_ms)
        })
        .unwrap_or(0.30);
    let jitter_q = node
        .jitter_ms
        .map(|value| {
            quality_lower_is_better(value, thresholds.jitter_good_ms, thresholds.jitter_bad_ms)
        })
        .unwrap_or(0.30);
    let loss_q = node
        .loss_pct
        .map(|value| {
            quality_lower_is_better(value, thresholds.loss_good_pct, thresholds.loss_bad_pct)
        })
        .unwrap_or(0.30);
    let success_5m_q = node.success_rate_5m.map(percent_to_q).unwrap_or(0.50);
    let success_1h_q = node.success_rate_1h.map(percent_to_q).unwrap_or(0.50);
    let observation_q =
        clamp01(node.observation_count as f64 / thresholds.min_observation_count as f64);
    let failure_penalty = clamp01(
        node.consecutive_failures as f64 / policy.score.max_consecutive_failures_for_penalty as f64,
    );
    let base_score = 100.0
        * ((weights.latency * latency_q)
            + (weights.jitter * jitter_q)
            + (weights.loss * loss_q)
            + (weights.success_5m * success_5m_q)
            + (weights.success_1h * success_1h_q));
    let status_multiplier = node.status.status_multiplier();
    let final_score = clamp_score(
        base_score
            * (0.50 + (0.50 * observation_q))
            * (1.0 - (policy.score.failure_penalty_weight * failure_penalty))
            * status_multiplier,
    );
    let reason = if node.status.is_down() {
        MeshNodeReasonCode::AllNodesDown
    } else if node.observation_count < thresholds.min_observation_count {
        MeshNodeReasonCode::InsufficientObservations
    } else {
        MeshNodeReasonCode::BestSelected
    };
    MeshNodeScoreBreakdown {
        node_id: node.node_id.clone(),
        latency_q,
        jitter_q,
        loss_q,
        success_5m_q,
        success_1h_q,
        observation_q,
        failure_penalty,
        status_multiplier,
        base_score,
        final_score,
        reason,
    }
}

pub fn refresh_mesh_node_scores(nodes: &mut [MeshNode], policy: &MeshNodesPolicy) {
    for node in nodes {
        node.score = compute_mesh_node_score(node, policy).final_score;
    }
}

fn quality_lower_is_better(value: f64, good: f64, bad: f64) -> f64 {
    if bad <= good {
        return 0.0;
    }
    clamp01(1.0 - ((value - good) / (bad - good)))
}

fn percent_to_q(value: f64) -> f64 {
    clamp01(value / 100.0)
}
