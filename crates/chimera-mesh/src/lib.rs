#![forbid(unsafe_code)]

mod model;
mod nodes_explain;
mod nodes_grouping;
mod nodes_model;
mod nodes_policy;
mod nodes_runtime;
mod nodes_scoring;
mod policy;
mod policy_parse;
mod preemptive;
mod runtime;

pub use model::{
    MeshDiscoveryRecord, MeshFailoverEvent, MeshJoinMode, MeshJoinRequest, MeshPathPlan,
    MeshPeerHealth, MeshPeerState, PreemptiveRisk, SwitchDecision,
};
pub use nodes_explain::{build_mesh_node_explain, render_mesh_node_explain};
pub use nodes_grouping::{
    MeshNodeCountryGroup, MeshNodeListFilter, group_mesh_nodes_by_country, sort_mesh_nodes_for_list,
};
pub use nodes_model::{
    MeshNode, MeshNodeCountry, MeshNodeCountryConfidence, MeshNodeCountrySource, MeshNodeId,
    MeshNodeReasonCode, MeshNodeScoreBreakdown, MeshNodeStatus, MeshNodeSwitchAction,
    MeshNodeSwitchDecision,
};
pub use nodes_policy::{
    MeshNodeAntiFlapPolicy, MeshNodeGeoPolicy, MeshNodeScorePolicy, MeshNodeScoreThresholds,
    MeshNodeScoreWeights, MeshNodesPolicy,
};
pub use nodes_runtime::{MeshNodeRuntime, MeshNodeRuntimeState, select_best_mesh_node};
pub use nodes_scoring::{compute_mesh_node_score, refresh_mesh_node_scores};
pub use policy::{
    ContinuityPolicy, MeshPathPolicy, MeshPathProfile, MeshPeerTablePolicy, MeshTrafficHints,
    MultipathMode, ShadowSwitchMode, TrafficClass, TrafficClassProfile,
    continuity_policy_from_dps_payload, multipath_mode_from_dps_payload,
    traffic_class_from_dps_payload, traffic_hints_from_dps_payload,
};
pub use runtime::{
    MeshConnectAttempt, MeshConnectProbeReport, MeshPeerTableEnforcementReport, MeshRuntime,
    MeshRuntimeStatusReport, evaluate_join_mode,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_auto_profile;
#[cfg(test)]
mod tests_discovery_merge;
#[cfg(test)]
mod tests_dps_explain;
#[cfg(test)]
mod tests_dps_policy;
#[cfg(test)]
mod tests_dps_runtime_flow;
#[cfg(test)]
mod tests_failover_health;
#[cfg(test)]
mod tests_mesh_nodes;
#[cfg(test)]
mod tests_peer_table_policy;
#[cfg(test)]
mod tests_peer_table_runtime;
#[cfg(test)]
mod tests_policy_parsers;
#[cfg(test)]
mod tests_preemptive_status;
#[cfg(test)]
mod tests_selection_behavior;
#[cfg(test)]
mod tests_selection_policy;
#[cfg(test)]
mod tests_standby_shadow;
#[cfg(test)]
mod tests_validation;
