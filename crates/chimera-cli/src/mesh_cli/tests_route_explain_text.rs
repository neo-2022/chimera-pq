use super::route_explain_meta::{ROUTE_EXPLAIN_CONTRACT_VERSION, ROUTE_EXPLAIN_STATUS_OK};
use super::tests_contract_constants::{
    BOOL_FALSE, BOOL_TRUE, SOURCE_PLAN_SETUP_EXPLAIN, SOURCE_SETUP_COMPUTED, SOURCE_STATUS_REPORT,
    SUCCESS_OPERATOR_ACTION, SUCCESS_OPERATOR_REASON,
};
use super::tests_json_utils::{
    expected_health_summary_ok, expected_integrity_all_true,
    expected_integrity_all_true_text_field, expected_operator_summary, expected_status_ok,
};
use super::*;

#[test]
fn mesh_route_explain_text_contains_setup_compact_and_reason_chain() {
    let setup_compact = format!(
        "join_mode:PublicDiscovery;sources:2;entries_after:2;consistency_gate:{ROUTE_EXPLAIN_STATUS_OK};degraded:false"
    );
    let dps_shadow_compact = format!(
        "pri=12.34;stage=clear;trigger=none;degraded=false;consistency_gate={ROUTE_EXPLAIN_STATUS_OK};setup_consistency=gate_match:true;degraded_match:true"
    );
    let operator_summary = expected_operator_summary(
        expected_status_ok(),
        "node-a",
        "limited",
        SUCCESS_OPERATOR_ACTION,
        SUCCESS_OPERATOR_REASON,
    );
    let consistency_source_matrix = format!(
        "plan=plan_setup;status_plan={};status_setup={};setup=dps_payload;dps_plan={};dps_setup={};status_compact_plan={};status_compact_setup={};dps_compact_plan={};dps_compact_setup={}",
        SOURCE_STATUS_REPORT,
        SOURCE_STATUS_REPORT,
        SOURCE_PLAN_SETUP_EXPLAIN,
        SOURCE_SETUP_COMPUTED,
        SOURCE_STATUS_REPORT,
        SOURCE_STATUS_REPORT,
        SOURCE_PLAN_SETUP_EXPLAIN,
        SOURCE_SETUP_COMPUTED,
    );
    let line = format_mesh_route_explain_text(&MeshRouteExplainTextView {
        contract_version: ROUTE_EXPLAIN_CONTRACT_VERSION,
        namespace: "cef-public",
        node_name: "node-client",
        selected_peer: "node-a",
        join_mode: chimera_mesh::MeshJoinMode::PublicDiscovery,
        consistency_gate: ROUTE_EXPLAIN_STATUS_OK,
        degraded_path: BOOL_FALSE,
        confidence_summary: "conf=0.9000;min=0.7000;passed=true;sample_age_ticks=2",
        readiness_summary: "eligible=1;switch_valid=true;health_blocked=0;confidence_gate_passed=true;sample_age_ticks=2",
        selection_pressure_summary: "considered:3;selected:1;rejected:1;limit_skipped:1;utilization_pct:50;headroom:1",
        selection_pressure_level: "limited",
        selection_pressure_score: "75",
        selection_pressure_dominant: "limit",
        selection_pressure_action_hint: "max_peer_limit",
        selection_pressure_compact: "level:limited;score:75;dominant:limit;action:max_peer_limit",
        selection_pressure_reason: "level=limited;dominant=limit;blocked=0;health=0;region=1;reliability=0;load=0;limit_skipped=1;headroom=1",
        reason_chain: "reason=none;guard=none;source=none",
        setup_compact: &setup_compact,
        setup_compact_consistency: "gate_match:true;degraded_match:true",
        plan_setup_compact_consistency: "gate_match:true;degraded_match:true",
        plan_setup_compact_consistency_match: BOOL_TRUE,
        plan_setup_compact_consistency_match_source: "plan_setup",
        setup_compact_consistency_match: BOOL_TRUE,
        setup_compact_consistency_match_source: "dps_payload",
        status_shadow_setup_match_source_from_compact: SOURCE_STATUS_REPORT,
        status_shadow_plan_setup_match_source_from_compact: SOURCE_STATUS_REPORT,
        dps_plan_setup_compact_consistency_match: BOOL_TRUE,
        dps_plan_setup_compact_consistency_match_source: SOURCE_PLAN_SETUP_EXPLAIN,
        dps_setup_compact_consistency_match: BOOL_TRUE,
        dps_setup_compact_consistency_match_source: SOURCE_SETUP_COMPUTED,
        dps_shadow_setup_match_source_from_compact: SOURCE_SETUP_COMPUTED,
        dps_shadow_plan_setup_match_source_from_compact: SOURCE_PLAN_SETUP_EXPLAIN,
        dps_selection_pressure_summary: "considered:3;selected:1;rejected:1;limit_skipped:1;utilization_pct:50;headroom:1",
        dps_selection_pressure_level: "limited",
        dps_selection_pressure_score: "75",
        dps_selection_pressure_dominant: "limit",
        dps_selection_pressure_action_hint: "max_peer_limit",
        dps_selection_pressure_compact: "level:limited;score:75;dominant:limit;action:max_peer_limit",
        dps_selection_pressure_reason: "level=limited;dominant=limit;blocked=0;health=0;region=1;reliability=0;load=0;limit_skipped=1;headroom=1",
        selection_pressure_projection_consistency: "summary_match:true;level_match:true;score_match:true;compact_match:true",
        selection_pressure_projection_gate: ROUTE_EXPLAIN_STATUS_OK,
        route_explain_health_gate: ROUTE_EXPLAIN_STATUS_OK,
        route_explain_health_summary: &expected_health_summary_ok(),
        route_explain_operator_summary: &operator_summary,
        route_explain_contract_integrity: expected_integrity_all_true(),
        consistency_source_matrix: &consistency_source_matrix,
        dps_shadow_compact: &dps_shadow_compact,
    });

    assert!(line.contains(&format!(
        "Mesh route explain: contract={ROUTE_EXPLAIN_CONTRACT_VERSION}"
    )));
    assert!(line.contains(
        "selection_pressure=considered:3;selected:1;rejected:1;limit_skipped:1;utilization_pct:50;headroom:1"
    ));
    assert!(line.contains("selection_pressure_level=limited"));
    assert!(line.contains("selection_pressure_score=75"));
    assert!(line.contains("selection_pressure_dominant=limit"));
    assert!(line.contains("selection_pressure_action_hint=max_peer_limit"));
    assert!(line.contains(
        "selection_pressure_compact=level:limited;score:75;dominant:limit;action:max_peer_limit"
    ));
    assert!(line.contains("selection_pressure_reason=level=limited;dominant=limit"));
    assert!(line.contains("dps_selection_pressure_dominant=limit"));
    assert!(line.contains(
        "dps_selection_pressure=considered:3;selected:1;rejected:1;limit_skipped:1;utilization_pct:50;headroom:1"
    ));
    assert!(line.contains("dps_selection_pressure_level=limited"));
    assert!(line.contains("dps_selection_pressure_score=75"));
    assert!(line.contains("dps_selection_pressure_action_hint=max_peer_limit"));
    assert!(line.contains(
        "dps_selection_pressure_compact=level:limited;score:75;dominant:limit;action:max_peer_limit"
    ));
    assert!(line.contains("dps_selection_pressure_reason=level=limited;dominant=limit"));
    assert!(line.contains(
        "selection_pressure_projection_consistency=summary_match:true;level_match:true;score_match:true;compact_match:true"
    ));
    assert!(line.contains(&format!(
        "selection_pressure_projection_gate={ROUTE_EXPLAIN_STATUS_OK}"
    )));
    assert!(line.contains(&format!(
        "route_explain_health_gate={ROUTE_EXPLAIN_STATUS_OK}"
    )));
    assert!(line.contains(&format!(
        "route_explain_health_summary={}",
        expected_health_summary_ok()
    )));
    assert!(line.contains(&format!(
        "route_explain_operator_summary={}",
        operator_summary
    )));
    assert!(line.contains(&expected_integrity_all_true_text_field()));
    assert!(line.contains("reason_chain=reason=none;guard=none;source=none"));
    assert!(line.contains(&format!("setup_compact={setup_compact}")));
    assert!(line.contains("setup_compact_consistency=gate_match:true;degraded_match:true"));
    assert!(line.contains("plan_setup_compact_consistency=gate_match:true;degraded_match:true"));
    assert!(line.contains("plan_setup_compact_consistency_match=true"));
    assert!(line.contains("plan_setup_compact_consistency_match_source=plan_setup"));
    assert!(line.contains("setup_compact_consistency_match=true"));
    assert!(line.contains("setup_compact_consistency_match_source=dps_payload"));
    assert!(line.contains(&format!(
        "status_shadow_setup_match_source_from_compact={SOURCE_STATUS_REPORT}"
    )));
    assert!(line.contains(&format!(
        "status_shadow_plan_setup_match_source_from_compact={SOURCE_STATUS_REPORT}"
    )));
    assert!(line.contains("dps_plan_setup_compact_consistency_match=true"));
    assert!(line.contains(&format!(
        "dps_plan_setup_compact_consistency_match_source={SOURCE_PLAN_SETUP_EXPLAIN}"
    )));
    assert!(line.contains("dps_setup_compact_consistency_match=true"));
    assert!(line.contains(&format!(
        "dps_setup_compact_consistency_match_source={SOURCE_SETUP_COMPUTED}"
    )));
    assert!(line.contains(&format!(
        "dps_shadow_setup_match_source_from_compact={SOURCE_SETUP_COMPUTED}"
    )));
    assert!(line.contains(&format!(
        "dps_shadow_plan_setup_match_source_from_compact={SOURCE_PLAN_SETUP_EXPLAIN}"
    )));
    assert!(line.contains(&format!(
        "consistency_source_matrix={consistency_source_matrix}"
    )));
    assert!(line.contains(&format!("dps_shadow_compact={dps_shadow_compact}")));
}
