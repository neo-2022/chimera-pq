use crate::nodes_model::{MeshNode, MeshNodeReasonCode, MeshNodeScoreBreakdown};
use crate::nodes_policy::MeshNodesPolicy;
use crate::nodes_scoring::compute_mesh_node_score;

#[derive(Debug, Clone, PartialEq)]
pub struct MeshNodeExplain {
    pub node_id: String,
    pub status: String,
    pub country_code: String,
    pub country_name: String,
    pub country_source: String,
    pub country_confidence: String,
    pub country_conflict: bool,
    pub country_conflict_reason: Option<String>,
    pub score_breakdown: MeshNodeScoreBreakdown,
    pub reason_codes: Vec<MeshNodeReasonCode>,
    pub explain_reason: String,
}

pub fn build_mesh_node_explain(node: &MeshNode, policy: &MeshNodesPolicy) -> MeshNodeExplain {
    let mut reason_codes = Vec::new();
    if node.country.is_unknown() {
        reason_codes.push(MeshNodeReasonCode::CountryUnknown);
    }
    if node.country.country_conflict {
        reason_codes.push(MeshNodeReasonCode::GeoIpConflict);
    }
    if node.observation_count < policy.score.thresholds.min_observation_count {
        reason_codes.push(MeshNodeReasonCode::InsufficientObservations);
    }
    if node.status.is_down() {
        reason_codes.push(MeshNodeReasonCode::AllNodesDown);
    }
    if reason_codes.is_empty() {
        reason_codes.push(MeshNodeReasonCode::BestSelected);
    }
    MeshNodeExplain {
        node_id: node.node_id.0.clone(),
        status: node.status.as_str().to_string(),
        country_code: node.country.country_code.clone(),
        country_name: node.country.country_name.clone(),
        country_source: node.country.country_source.as_str().to_string(),
        country_confidence: node.country.country_confidence.as_str().to_string(),
        country_conflict: node.country.country_conflict,
        country_conflict_reason: node.country.country_conflict_reason.clone(),
        score_breakdown: compute_mesh_node_score(node, policy),
        reason_codes,
        explain_reason: node.explain_reason.clone(),
    }
}

pub fn render_mesh_node_explain(explain: &MeshNodeExplain) -> String {
    let reasons = explain
        .reason_codes
        .iter()
        .map(|reason| reason.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let mut out = String::new();
    out.push_str(&format!("node_id: {}\n", explain.node_id));
    out.push_str(&format!("status: {}\n", explain.status));
    out.push_str(&format!(
        "country: {} ({}) source={} confidence={}\n",
        explain.country_name,
        explain.country_code,
        explain.country_source,
        explain.country_confidence
    ));
    if explain.country_conflict {
        out.push_str(&format!(
            "country_conflict: true reason={}\n",
            explain
                .country_conflict_reason
                .clone()
                .unwrap_or_else(|| "geoip_conflict".to_string())
        ));
    }
    out.push_str("score_breakdown:\n");
    out.push_str(&format!(
        "  latency_q: {:.3}\n  jitter_q: {:.3}\n  loss_q: {:.3}\n",
        explain.score_breakdown.latency_q,
        explain.score_breakdown.jitter_q,
        explain.score_breakdown.loss_q
    ));
    out.push_str(&format!(
        "  observation_q: {:.3}\n  failure_penalty: {:.3}\n  status_multiplier: {:.3}\n",
        explain.score_breakdown.observation_q,
        explain.score_breakdown.failure_penalty,
        explain.score_breakdown.status_multiplier
    ));
    out.push_str(&format!(
        "  base_score: {:.2}\n  final_score: {:.2}\n",
        explain.score_breakdown.base_score, explain.score_breakdown.final_score
    ));
    out.push_str(&format!("reason_codes: {reasons}\n"));
    out.push_str(&format!("explain_reason: {}\n", explain.explain_reason));
    out
}
