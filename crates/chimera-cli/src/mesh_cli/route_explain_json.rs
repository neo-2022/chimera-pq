use super::route_explain_envelope::insert_route_explain_envelope;
use super::route_explain_json_insert::insert_json;
use super::route_explain_meta::{
    ROUTE_EXPLAIN_KIND_OK, ROUTE_EXPLAIN_NETWORK_STATE_NOT_MODIFIED, ROUTE_EXPLAIN_STATUS_OK,
};
use super::route_explain_types::MeshRouteExplainFields;

pub(crate) fn build_json_string(fields: &MeshRouteExplainFields) -> String {
    let mut object = serde_json::Map::with_capacity(66);
    insert_route_explain_envelope(
        &mut object,
        ROUTE_EXPLAIN_STATUS_OK,
        ROUTE_EXPLAIN_KIND_OK,
        &fields.contract_version,
        &fields.namespace,
        &fields.node_name,
    );
    insert_json(&mut object, "join_mode", &format!("{:?}", fields.join_mode));
    insert_json(
        &mut object,
        "initial_selected_peer",
        &fields.initial_selected,
    );
    insert_json(
        &mut object,
        "failover_selected_peer",
        &fields.failover_selected,
    );
    insert_json(
        &mut object,
        "cooldown_selected_peer",
        &fields.cooldown_selected,
    );
    insert_json(
        &mut object,
        "table_runtime_consistency_gate",
        &fields.table_runtime_consistency_gate,
    );
    insert_json(
        &mut object,
        "table_runtime_consistency_all_true",
        &fields.table_runtime_consistency_all_true,
    );
    insert_json(
        &mut object,
        "table_runtime_consistency_summary",
        &fields.table_runtime_consistency_summary,
    );
    insert_json(
        &mut object,
        "plan_setup_discovery_table_compact",
        &fields.plan_setup_discovery_table_compact,
    );
    insert_json(
        &mut object,
        "plan_setup_discovery_table_compact_consistency",
        &fields.plan_setup_discovery_table_compact_consistency,
    );
    insert_json(
        &mut object,
        "plan_setup_discovery_table_compact_consistency_match",
        &fields.plan_setup_discovery_table_compact_consistency_match,
    );
    insert_json(
        &mut object,
        "plan_setup_discovery_table_compact_consistency_match_source",
        &fields.plan_setup_discovery_table_compact_consistency_match_source,
    );
    insert_json(
        &mut object,
        "status_plan_setup_compact_consistency_match_source",
        &fields.status_plan_setup_compact_consistency_match_source,
    );
    insert_json(
        &mut object,
        "status_setup_compact_consistency_match_source",
        &fields.status_setup_compact_consistency_match_source,
    );
    insert_json(
        &mut object,
        "status_shadow_setup_match_source_from_compact",
        &fields.status_shadow_setup_match_source_from_compact,
    );
    insert_json(
        &mut object,
        "status_shadow_plan_setup_match_source_from_compact",
        &fields.status_shadow_plan_setup_match_source_from_compact,
    );
    insert_json(
        &mut object,
        "dps_payload_plan_setup_compact_consistency",
        &fields.dps_payload_plan_setup_compact_consistency,
    );
    insert_json(
        &mut object,
        "setup_compact_consistency_match",
        &fields.setup_compact_consistency_match,
    );
    insert_json(
        &mut object,
        "setup_compact_consistency_match_source",
        &fields.setup_compact_consistency_match_source,
    );
    insert_json(
        &mut object,
        "dps_payload_plan_setup_compact_consistency_match",
        &fields.dps_payload_plan_setup_compact_consistency_match,
    );
    insert_json(
        &mut object,
        "dps_payload_plan_setup_compact_consistency_match_source",
        &fields.dps_payload_plan_setup_compact_consistency_match_source,
    );
    insert_json(
        &mut object,
        "dps_payload_setup_compact_consistency_match",
        &fields.dps_payload_setup_compact_consistency_match,
    );
    insert_json(
        &mut object,
        "dps_payload_setup_compact_consistency_match_source",
        &fields.dps_payload_setup_compact_consistency_match_source,
    );
    insert_json(
        &mut object,
        "dps_shadow_setup_match_source_from_compact",
        &fields.dps_shadow_setup_match_source_from_compact,
    );
    insert_json(
        &mut object,
        "dps_shadow_plan_setup_match_source_from_compact",
        &fields.dps_shadow_plan_setup_match_source_from_compact,
    );
    insert_json(
        &mut object,
        "dps_payload_selection_pressure_summary",
        &fields.dps_payload_selection_pressure_summary,
    );
    insert_json(
        &mut object,
        "dps_payload_selection_pressure_level",
        &fields.dps_payload_selection_pressure_level,
    );
    insert_json(
        &mut object,
        "dps_payload_selection_pressure_reason",
        &fields.dps_payload_selection_pressure_reason,
    );
    insert_json(
        &mut object,
        "dps_payload_selection_pressure_score",
        &fields.dps_payload_selection_pressure_score,
    );
    insert_json(
        &mut object,
        "dps_payload_selection_pressure_dominant",
        &fields.dps_payload_selection_pressure_dominant,
    );
    insert_json(
        &mut object,
        "dps_payload_selection_pressure_action_hint",
        &fields.dps_payload_selection_pressure_action_hint,
    );
    insert_json(
        &mut object,
        "dps_payload_selection_pressure_compact",
        &fields.dps_payload_selection_pressure_compact,
    );
    insert_json(
        &mut object,
        "selection_pressure_projection_consistency",
        &fields.selection_pressure_projection_consistency,
    );
    insert_json(
        &mut object,
        "selection_pressure_projection_gate",
        &fields.selection_pressure_projection_gate,
    );
    insert_json(
        &mut object,
        "route_explain_health_gate",
        &fields.route_explain_health_gate,
    );
    insert_json(
        &mut object,
        "route_explain_health_summary",
        &fields.route_explain_health_summary,
    );
    insert_json(
        &mut object,
        "route_explain_operator_summary",
        &fields.route_explain_operator_summary,
    );
    insert_json(
        &mut object,
        "route_explain_operator_signature",
        &fields.route_explain_operator_signature,
    );
    insert_json(
        &mut object,
        "route_explain_operator_route_key",
        &fields.route_explain_operator_route_key,
    );
    insert_json(
        &mut object,
        "route_explain_operator_health",
        &fields.route_explain_operator_health,
    );
    insert_json(
        &mut object,
        "route_explain_operator_selected",
        &fields.route_explain_operator_selected,
    );
    insert_json(
        &mut object,
        "route_explain_operator_pressure",
        &fields.route_explain_operator_pressure,
    );
    insert_json(
        &mut object,
        "route_explain_operator_action",
        &fields.route_explain_operator_action,
    );
    insert_json(
        &mut object,
        "route_explain_operator_reason",
        &fields.route_explain_operator_reason,
    );
    insert_json(
        &mut object,
        "route_explain_contract_integrity",
        &fields.route_explain_contract_integrity,
    );
    insert_json(
        &mut object,
        "consistency_source_matrix",
        &fields.consistency_source_matrix,
    );
    insert_json(
        &mut object,
        "dps_payload_preemptive_shadow_compact",
        &fields.dps_payload_preemptive_shadow_compact,
    );
    insert_json(
        &mut object,
        "preemptive_shadow_degraded_path",
        &fields.preemptive_degraded_path,
    );
    insert_json(
        &mut object,
        "preemptive_shadow_degraded_reason",
        &fields.preemptive_degraded_reason,
    );
    insert_json(
        &mut object,
        "preemptive_shadow_degraded_summary",
        &fields.preemptive_shadow_degraded_summary,
    );
    insert_json(
        &mut object,
        "preemptive_shadow_switch_candidate_confidence",
        &fields.preemptive_shadow_switch_candidate_confidence,
    );
    insert_json(
        &mut object,
        "preemptive_shadow_switch_confidence_gate_min",
        &fields.preemptive_shadow_switch_confidence_gate_min,
    );
    insert_json(
        &mut object,
        "preemptive_shadow_switch_confidence_gate_passed",
        &fields.preemptive_shadow_switch_confidence_gate_passed,
    );
    insert_json(
        &mut object,
        "preemptive_shadow_switch_candidate_sample_age_ticks",
        &fields.preemptive_shadow_switch_candidate_sample_age_ticks,
    );
    insert_json(
        &mut object,
        "preemptive_shadow_switch_confidence_summary",
        &fields.preemptive_shadow_switch_confidence_summary,
    );
    insert_json(
        &mut object,
        "preemptive_shadow_switch_block_reason_chain",
        &fields.preemptive_shadow_switch_block_reason_chain,
    );
    insert_json(
        &mut object,
        "preemptive_shadow_candidate_readiness_summary",
        &fields.preemptive_shadow_candidate_readiness_summary,
    );
    insert_json(
        &mut object,
        "auto_recovery_attempts",
        &fields.auto_recovery_attempts,
    );
    insert_json(
        &mut object,
        "auto_recovery_final_result",
        &fields.auto_recovery_final_result,
    );
    insert_json(
        &mut object,
        "connect_retry_budget_exhausted",
        &fields.connect_retry_budget_exhausted,
    );
    insert_json(
        &mut object,
        "connect_recovery_needed",
        &fields.connect_recovery_needed,
    );
    insert_json(
        &mut object,
        "connect_recovery_strategy",
        &fields.connect_recovery_strategy,
    );
    insert_json(
        &mut object,
        "connect_recovery_projection_consistency",
        &fields.connect_recovery_projection_consistency,
    );
    insert_json(
        &mut object,
        "connect_recovery_projection_key",
        &fields.connect_recovery_projection_key,
    );
    insert_json(
        &mut object,
        "route_explain_recovery_schema_version",
        &fields.route_explain_recovery_schema_version,
    );
    insert_json(
        &mut object,
        "route_explain_recovery_fields_checksum",
        &fields.route_explain_recovery_fields_checksum,
    );
    insert_json(
        &mut object,
        "selection_pressure_summary",
        &fields.selection_pressure_summary,
    );
    insert_json(
        &mut object,
        "selection_pressure_level",
        &fields.selection_pressure_level,
    );
    insert_json(
        &mut object,
        "selection_pressure_score",
        &fields.selection_pressure_score,
    );
    insert_json(
        &mut object,
        "selection_pressure_dominant",
        &fields.selection_pressure_dominant,
    );
    insert_json(
        &mut object,
        "selection_pressure_action_hint",
        &fields.selection_pressure_action_hint,
    );
    insert_json(
        &mut object,
        "selection_pressure_compact",
        &fields.selection_pressure_compact,
    );
    insert_json(
        &mut object,
        "selection_pressure_reason",
        &fields.selection_pressure_reason,
    );
    insert_json(
        &mut object,
        "selected_peer_connect_retry_plan",
        &fields.selected_peer_connect_retry_plan,
    );
    insert_json(
        &mut object,
        "selected_peer_connect_backoff_profile",
        &fields.selected_peer_connect_backoff_profile,
    );
    insert_json(&mut object, "explain", &fields.explain);
    insert_json(
        &mut object,
        "network_state",
        ROUTE_EXPLAIN_NETWORK_STATE_NOT_MODIFIED,
    );
    serde_json::Value::Object(object).to_string()
}
