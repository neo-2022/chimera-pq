use super::route_explain_contract::{MeshRouteExplainTextView, format_mesh_route_explain_text};
use super::route_explain_fields::collect_mesh_route_explain_fields;
use super::route_explain_json::build_json_string;
use super::route_explain_types::MeshRouteExplainRender;

pub(crate) struct MeshRouteExplainOutput {
    pub(crate) json: String,
    pub(crate) text: String,
}

pub(crate) fn build_mesh_route_explain_output(
    render: MeshRouteExplainRender<'_>,
) -> MeshRouteExplainOutput {
    let fields = collect_mesh_route_explain_fields(render);
    let json = build_json_string(&fields);
    let text = format_mesh_route_explain_text(&MeshRouteExplainTextView {
        contract_version: &fields.contract_version,
        namespace: &fields.namespace,
        node_name: &fields.node_name,
        selected_peer: &fields.initial_selected,
        join_mode: fields.join_mode.clone(),
        consistency_gate: &fields.table_runtime_consistency_gate,
        degraded_path: &fields.preemptive_degraded_path,
        confidence_summary: &fields.preemptive_shadow_switch_confidence_summary,
        readiness_summary: &fields.preemptive_shadow_candidate_readiness_summary,
        selection_pressure_summary: &fields.selection_pressure_summary,
        selection_pressure_level: &fields.selection_pressure_level,
        selection_pressure_score: &fields.selection_pressure_score,
        selection_pressure_dominant: &fields.selection_pressure_dominant,
        selection_pressure_action_hint: &fields.selection_pressure_action_hint,
        selection_pressure_compact: &fields.selection_pressure_compact,
        selection_pressure_reason: &fields.selection_pressure_reason,
        reason_chain: &fields.preemptive_shadow_switch_block_reason_chain,
        setup_compact: &fields.plan_setup_discovery_table_compact,
        setup_compact_consistency: &fields.dps_payload_plan_setup_compact_consistency,
        plan_setup_compact_consistency: &fields.plan_setup_discovery_table_compact_consistency,
        plan_setup_compact_consistency_match: &fields
            .plan_setup_discovery_table_compact_consistency_match,
        plan_setup_compact_consistency_match_source: &fields
            .plan_setup_discovery_table_compact_consistency_match_source,
        setup_compact_consistency_match: &fields.setup_compact_consistency_match,
        setup_compact_consistency_match_source: &fields.setup_compact_consistency_match_source,
        status_shadow_setup_match_source_from_compact: &fields
            .status_shadow_setup_match_source_from_compact,
        status_shadow_plan_setup_match_source_from_compact: &fields
            .status_shadow_plan_setup_match_source_from_compact,
        dps_plan_setup_compact_consistency_match: &fields
            .dps_payload_plan_setup_compact_consistency_match,
        dps_plan_setup_compact_consistency_match_source: &fields
            .dps_payload_plan_setup_compact_consistency_match_source,
        dps_setup_compact_consistency_match: &fields.dps_payload_setup_compact_consistency_match,
        dps_setup_compact_consistency_match_source: &fields
            .dps_payload_setup_compact_consistency_match_source,
        dps_shadow_setup_match_source_from_compact: &fields
            .dps_shadow_setup_match_source_from_compact,
        dps_shadow_plan_setup_match_source_from_compact: &fields
            .dps_shadow_plan_setup_match_source_from_compact,
        dps_selection_pressure_summary: &fields.dps_payload_selection_pressure_summary,
        dps_selection_pressure_level: &fields.dps_payload_selection_pressure_level,
        dps_selection_pressure_score: &fields.dps_payload_selection_pressure_score,
        dps_selection_pressure_dominant: &fields.dps_payload_selection_pressure_dominant,
        dps_selection_pressure_action_hint: &fields.dps_payload_selection_pressure_action_hint,
        dps_selection_pressure_compact: &fields.dps_payload_selection_pressure_compact,
        dps_selection_pressure_reason: &fields.dps_payload_selection_pressure_reason,
        selection_pressure_projection_consistency: &fields
            .selection_pressure_projection_consistency,
        selection_pressure_projection_gate: &fields.selection_pressure_projection_gate,
        route_explain_health_gate: &fields.route_explain_health_gate,
        route_explain_health_summary: &fields.route_explain_health_summary,
        route_explain_operator_summary: &fields.route_explain_operator_summary,
        route_explain_contract_integrity: &fields.route_explain_contract_integrity,
        consistency_source_matrix: &fields.consistency_source_matrix,
        dps_shadow_compact: &fields.dps_payload_preemptive_shadow_compact,
    });
    MeshRouteExplainOutput { json, text }
}
