use super::route_explain_contract::{
    bool_text, compact_field_value, explain_value, explain_value_any,
    format_consistency_source_matrix, infer_dps_plan_setup_match_source,
    infer_dps_setup_match_source, infer_plan_setup_match_source, infer_setup_match_source,
    infer_status_plan_setup_match_source, infer_status_setup_match_source, parse_consistency_match,
};
use super::route_explain_health::{
    build_route_explain_health, build_route_explain_operator_summary,
};
use super::route_explain_integrity::build_route_explain_contract_integrity;
use super::route_explain_pressure::collect_pressure_fields;
use super::route_explain_recovery::project_auto_recovery;
use super::route_explain_recovery_projection::build_connect_recovery_projection;
use super::route_explain_types::{MeshRouteExplainFields, MeshRouteExplainRender};

pub(crate) fn collect_mesh_route_explain_fields(
    render: MeshRouteExplainRender<'_>,
) -> MeshRouteExplainFields {
    let initial_selected = render
        .initial
        .selected_peers
        .first()
        .map(|peer| peer.node_id.as_str())
        .unwrap_or_default();
    let preemptive_degraded_path =
        explain_value(&render.initial.explain, "preemptive_shadow_degraded_path=")
            .unwrap_or("false");
    let preemptive_degraded_reason = explain_value(
        &render.initial.explain,
        "preemptive_shadow_degraded_reason=",
    )
    .unwrap_or("none");
    let preemptive_shadow_switch_candidate_confidence = explain_value(
        &render.initial.explain,
        "preemptive_shadow_switch_candidate_confidence=",
    )
    .or_else(|| {
        explain_value(
            &render.initial.explain,
            "preemptive_shadow_switch_confidence=",
        )
    })
    .unwrap_or("0.0000");
    let preemptive_shadow_switch_confidence_gate_min = explain_value(
        &render.initial.explain,
        "preemptive_shadow_switch_confidence_gate_min=",
    )
    .unwrap_or("0.0000");
    let preemptive_shadow_switch_confidence_gate_passed = explain_value(
        &render.initial.explain,
        "preemptive_shadow_switch_confidence_gate_passed=",
    )
    .unwrap_or("false");
    let preemptive_shadow_switch_candidate_sample_age_ticks = explain_value(
        &render.initial.explain,
        "preemptive_shadow_switch_candidate_sample_age_ticks=",
    )
    .unwrap_or("unknown");
    let preemptive_shadow_switch_confidence_summary = explain_value(
        &render.initial.explain,
        "preemptive_shadow_switch_confidence_summary=",
    )
    .unwrap_or("conf=0.0000;min=0.0000;passed=false;sample_age_ticks=unknown");
    let preemptive_shadow_switch_block_reason_chain = explain_value(
        &render.initial.explain,
        "preemptive_shadow_switch_block_reason_chain=",
    )
    .unwrap_or("reason=none;guard=none;source=none");
    let preemptive_shadow_candidate_readiness_summary = explain_value(
        &render.initial.explain,
        "preemptive_shadow_candidate_readiness_summary=",
    )
    .unwrap_or("eligible=0;switch_valid=false;health_blocked=0;confidence_gate_passed=false;sample_age_ticks=unknown");
    let auto_recovery = project_auto_recovery(&render.initial.explain);
    let pressure = collect_pressure_fields(&render.initial.explain);
    let plan_setup_discovery_table_compact = explain_value_any(
        &render.initial.explain,
        &[
            "plan_setup_discovery_table_compact=",
            "status_plan_setup_discovery_table_compact=",
        ],
    )
    .unwrap_or(
        "join_mode:Unknown;sources:0;entries_after:0;consistency_gate:unknown;degraded:false",
    );
    let dps_payload_plan_setup_compact_consistency = explain_value(
        &render.initial.explain,
        "dps_payload_plan_setup_compact_consistency=",
    )
    .unwrap_or("gate_match:unknown;degraded_match:unknown");
    let dps_payload_preemptive_shadow_compact = explain_value(
        &render.initial.explain,
        "dps_payload_preemptive_shadow_compact=",
    )
    .unwrap_or(
        "pri=0.00;stage=none;trigger=none;degraded=false;consistency_gate=unknown;setup_consistency=gate_match:unknown;degraded_match:unknown",
    );
    let dps_payload_setup_compact_consistency_match = explain_value(
        &render.initial.explain,
        "dps_payload_setup_compact_consistency_match=",
    )
    .unwrap_or("false");
    let dps_payload_setup_compact_consistency_match_source =
        infer_dps_setup_match_source(&render.initial.explain);
    let dps_payload_plan_setup_compact_consistency_match = explain_value(
        &render.initial.explain,
        "dps_payload_plan_setup_discovery_table_compact_consistency_match=",
    )
    .unwrap_or("false");
    let dps_payload_plan_setup_compact_consistency_match_source =
        infer_dps_plan_setup_match_source(&render.initial.explain);
    let plan_setup_discovery_table_compact_consistency = explain_value_any(
        &render.initial.explain,
        &[
            "plan_setup_discovery_table_compact_consistency=",
            "status_plan_setup_discovery_table_compact_consistency=",
        ],
    )
    .unwrap_or("gate_match:unknown;degraded_match:unknown");
    let plan_setup_discovery_table_compact_consistency_match = explain_value_any(
        &render.initial.explain,
        &[
            "plan_setup_discovery_table_compact_consistency_match=",
            "status_plan_setup_discovery_table_compact_consistency_match=",
            "status_setup_compact_consistency_match=",
        ],
    )
    .unwrap_or_else(|| {
        bool_text(parse_consistency_match(
            plan_setup_discovery_table_compact_consistency,
        ))
    });
    let plan_setup_discovery_table_compact_consistency_match_source =
        infer_plan_setup_match_source(&render.initial.explain);
    let status_plan_setup_compact_consistency_match_source =
        infer_status_plan_setup_match_source(&render.initial.explain);
    let status_setup_compact_consistency_match_source =
        infer_status_setup_match_source(&render.initial.explain);
    let setup_compact_consistency_match = explain_value(
        &render.initial.explain,
        "dps_payload_setup_compact_consistency_match=",
    )
    .unwrap_or_else(|| {
        bool_text(parse_consistency_match(
            dps_payload_plan_setup_compact_consistency,
        ))
    });
    let setup_compact_consistency_match_source = infer_setup_match_source(&render.initial.explain);
    let status_preemptive_shadow_compact =
        explain_value(&render.initial.explain, "status_preemptive_shadow_compact=").unwrap_or("");
    let status_shadow_setup_match_source_from_compact =
        compact_field_value(status_preemptive_shadow_compact, "setup_match_source")
            .unwrap_or("unknown");
    let status_shadow_plan_setup_match_source_from_compact =
        compact_field_value(status_preemptive_shadow_compact, "plan_setup_match_source")
            .unwrap_or("unknown");
    let dps_shadow_setup_match_source_from_compact =
        compact_field_value(dps_payload_preemptive_shadow_compact, "setup_match_source")
            .unwrap_or("unknown");
    let dps_shadow_plan_setup_match_source_from_compact = compact_field_value(
        dps_payload_preemptive_shadow_compact,
        "plan_setup_match_source",
    )
    .unwrap_or("unknown");
    let consistency_source_matrix = format_consistency_source_matrix([
        plan_setup_discovery_table_compact_consistency_match_source,
        status_plan_setup_compact_consistency_match_source,
        status_setup_compact_consistency_match_source,
        setup_compact_consistency_match_source,
        dps_payload_plan_setup_compact_consistency_match_source,
        dps_payload_setup_compact_consistency_match_source,
        status_shadow_plan_setup_match_source_from_compact,
        status_shadow_setup_match_source_from_compact,
        dps_shadow_plan_setup_match_source_from_compact,
        dps_shadow_setup_match_source_from_compact,
    ]);
    let table_runtime_consistency_gate = explain_value_any(
        &render.initial.explain,
        &[
            "peer_table_runtime_consistency_gate=",
            "status_table_runtime_consistency_gate=",
        ],
    )
    .unwrap_or("unknown");
    let table_runtime_consistency_all_true = if table_runtime_consistency_gate == "ok" {
        "true"
    } else {
        "false"
    };
    let table_runtime_consistency_summary = format!(
        "gate={};all_true={}",
        table_runtime_consistency_gate, table_runtime_consistency_all_true
    );
    let preemptive_shadow_degraded_summary = format!(
        "path={};reason={};{}",
        preemptive_degraded_path, preemptive_degraded_reason, table_runtime_consistency_summary
    );
    let route_explain_health = build_route_explain_health(
        table_runtime_consistency_gate,
        preemptive_degraded_path,
        pressure.projection_gate,
    );
    let route_explain_operator_summary = build_route_explain_operator_summary(
        route_explain_health.gate,
        initial_selected,
        pressure.selection_level,
        pressure.selection_action_hint,
        preemptive_degraded_reason,
        auto_recovery.retry_budget_exhausted,
    );
    let route_explain_contract_integrity = build_route_explain_contract_integrity(
        &route_explain_operator_summary.line,
        &route_explain_operator_summary.signature,
        &route_explain_operator_summary.route_key,
        route_explain_health.gate,
        &route_explain_operator_summary.health,
        &route_explain_operator_summary.action,
    );
    let connect_recovery_projection =
        build_connect_recovery_projection(&route_explain_operator_summary.action);
    let selected_peer_connect_retry_plan =
        explain_value(&render.initial.explain, "selected_peer_connect_retry_plan=")
            .unwrap_or("none");
    let selected_peer_connect_backoff_profile = explain_value(
        &render.initial.explain,
        "selected_peer_connect_backoff_profile=",
    )
    .unwrap_or("none");

    MeshRouteExplainFields {
        contract_version: render.contract_version.to_string(),
        namespace: render.options.namespace.clone(),
        node_name: render.options.node_name.clone(),
        join_mode: render.initial.join_mode.clone(),
        initial_selected: initial_selected.to_string(),
        failover_selected: render.failover_selected.to_string(),
        cooldown_selected: render.cooldown_selected.to_string(),
        table_runtime_consistency_gate: table_runtime_consistency_gate.to_string(),
        table_runtime_consistency_all_true: table_runtime_consistency_all_true.to_string(),
        table_runtime_consistency_summary,
        plan_setup_discovery_table_compact: plan_setup_discovery_table_compact.to_string(),
        plan_setup_discovery_table_compact_consistency:
            plan_setup_discovery_table_compact_consistency.to_string(),
        plan_setup_discovery_table_compact_consistency_match:
            plan_setup_discovery_table_compact_consistency_match.to_string(),
        plan_setup_discovery_table_compact_consistency_match_source:
            plan_setup_discovery_table_compact_consistency_match_source.to_string(),
        status_plan_setup_compact_consistency_match_source:
            status_plan_setup_compact_consistency_match_source.to_string(),
        status_setup_compact_consistency_match_source:
            status_setup_compact_consistency_match_source.to_string(),
        status_shadow_setup_match_source_from_compact:
            status_shadow_setup_match_source_from_compact.to_string(),
        status_shadow_plan_setup_match_source_from_compact:
            status_shadow_plan_setup_match_source_from_compact.to_string(),
        dps_payload_plan_setup_compact_consistency: dps_payload_plan_setup_compact_consistency
            .to_string(),
        setup_compact_consistency_match: setup_compact_consistency_match.to_string(),
        setup_compact_consistency_match_source: setup_compact_consistency_match_source.to_string(),
        dps_payload_plan_setup_compact_consistency_match:
            dps_payload_plan_setup_compact_consistency_match.to_string(),
        dps_payload_plan_setup_compact_consistency_match_source:
            dps_payload_plan_setup_compact_consistency_match_source.to_string(),
        dps_payload_setup_compact_consistency_match: dps_payload_setup_compact_consistency_match
            .to_string(),
        dps_payload_setup_compact_consistency_match_source:
            dps_payload_setup_compact_consistency_match_source.to_string(),
        dps_shadow_setup_match_source_from_compact: dps_shadow_setup_match_source_from_compact
            .to_string(),
        dps_shadow_plan_setup_match_source_from_compact:
            dps_shadow_plan_setup_match_source_from_compact.to_string(),
        dps_payload_selection_pressure_summary: pressure.dps_summary.to_string(),
        dps_payload_selection_pressure_level: pressure.dps_level.to_string(),
        dps_payload_selection_pressure_score: pressure.dps_score.to_string(),
        dps_payload_selection_pressure_dominant: pressure.dps_dominant.to_string(),
        dps_payload_selection_pressure_action_hint: pressure.dps_action_hint.to_string(),
        dps_payload_selection_pressure_compact: pressure.dps_compact.to_string(),
        dps_payload_selection_pressure_reason: pressure.dps_reason.to_string(),
        selection_pressure_projection_consistency: pressure.projection_consistency,
        selection_pressure_projection_gate: pressure.projection_gate.to_string(),
        route_explain_health_gate: route_explain_health.gate.to_string(),
        route_explain_health_summary: route_explain_health.summary,
        route_explain_operator_summary: route_explain_operator_summary.line,
        route_explain_operator_signature: route_explain_operator_summary.signature,
        route_explain_operator_route_key: route_explain_operator_summary.route_key,
        route_explain_operator_health: route_explain_operator_summary.health,
        route_explain_operator_selected: route_explain_operator_summary.selected,
        route_explain_operator_pressure: route_explain_operator_summary.pressure,
        route_explain_operator_action: route_explain_operator_summary.action,
        route_explain_operator_reason: route_explain_operator_summary.reason,
        route_explain_contract_integrity,
        consistency_source_matrix,
        dps_payload_preemptive_shadow_compact: dps_payload_preemptive_shadow_compact.to_string(),
        preemptive_degraded_path: preemptive_degraded_path.to_string(),
        preemptive_degraded_reason: preemptive_degraded_reason.to_string(),
        preemptive_shadow_degraded_summary,
        preemptive_shadow_switch_candidate_confidence:
            preemptive_shadow_switch_candidate_confidence.to_string(),
        preemptive_shadow_switch_confidence_gate_min: preemptive_shadow_switch_confidence_gate_min
            .to_string(),
        preemptive_shadow_switch_confidence_gate_passed:
            preemptive_shadow_switch_confidence_gate_passed.to_string(),
        preemptive_shadow_switch_candidate_sample_age_ticks:
            preemptive_shadow_switch_candidate_sample_age_ticks.to_string(),
        preemptive_shadow_switch_confidence_summary: preemptive_shadow_switch_confidence_summary
            .to_string(),
        preemptive_shadow_switch_block_reason_chain: preemptive_shadow_switch_block_reason_chain
            .to_string(),
        preemptive_shadow_candidate_readiness_summary:
            preemptive_shadow_candidate_readiness_summary.to_string(),
        auto_recovery_attempts: auto_recovery.attempts.to_string(),
        auto_recovery_final_result: auto_recovery.final_result,
        connect_retry_budget_exhausted: bool_text(auto_recovery.retry_budget_exhausted).to_string(),
        connect_recovery_needed: bool_text(connect_recovery_projection.needed).to_string(),
        connect_recovery_strategy: connect_recovery_projection.strategy.to_string(),
        connect_recovery_projection_consistency: bool_text(connect_recovery_projection.consistency)
            .to_string(),
        connect_recovery_projection_key: connect_recovery_projection.key,
        route_explain_recovery_schema_version: auto_recovery.schema_version.to_string(),
        route_explain_recovery_fields_checksum: auto_recovery.fields_checksum.to_string(),
        selection_pressure_summary: pressure.selection_summary.to_string(),
        selection_pressure_level: pressure.selection_level.to_string(),
        selection_pressure_score: pressure.selection_score.to_string(),
        selection_pressure_dominant: pressure.selection_dominant.to_string(),
        selection_pressure_action_hint: pressure.selection_action_hint.to_string(),
        selection_pressure_compact: pressure.selection_compact.to_string(),
        selection_pressure_reason: pressure.selection_reason.to_string(),
        selected_peer_connect_retry_plan: selected_peer_connect_retry_plan.to_string(),
        selected_peer_connect_backoff_profile: selected_peer_connect_backoff_profile.to_string(),
        explain: render.initial.explain.join(" | "),
    }
}
