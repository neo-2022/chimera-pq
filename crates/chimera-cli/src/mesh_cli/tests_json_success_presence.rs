use super::tests_contract_constants::{
    SUCCESS_OPERATOR_ACTION, SUCCESS_OPERATOR_REASON, SUCCESS_PRESSURE, SUCCESS_SELECTED_NODE,
};
use super::tests_json_runner_utils::run_route_explain_json;
use super::tests_json_success_utils::{
    assert_success_health_projection_consistency, assert_success_operator_health_block,
};
use super::tests_json_utils::{
    assert_operator_summary_invariants, assert_route_explain_contract_blocks_presence,
    assert_route_explain_envelope, base_route_explain_args, expected_contract_family,
    expected_contract_version, expected_health_summary_ok, expected_integrity_all_true,
    expected_kind_ok, expected_network_state, expected_operator_route_key_ok,
    expected_operator_summary, expected_recovery_fields_checksum, expected_recovery_schema_version,
    expected_status_ok,
};

const PROJECTION_CONSISTENCY_ALL_TRUE: &str =
    "summary_match:true;level_match:true;score_match:true;compact_match:true";

#[test]
fn mesh_route_explain_json_contains_preemptive_degraded_fields() {
    let parsed = run_route_explain_json(
        base_route_explain_args("allow=mesh;mesh_max_peers=1"),
        0,
        "degraded_fields",
    );
    let json = parsed.to_string();

    assert!(json.contains("\"kind\":\"mesh_route_explain\""));
    assert!(json.contains(&format!(
        "\"route_explain_contract_family\":\"{}\"",
        expected_contract_family()
    )));
    assert_route_explain_envelope(&parsed, expected_status_ok(), expected_kind_ok());
    assert_success_health_projection_consistency(&parsed);
    assert_success_operator_health_block(
        &parsed,
        SUCCESS_SELECTED_NODE,
        SUCCESS_PRESSURE,
        SUCCESS_OPERATOR_ACTION,
        SUCCESS_OPERATOR_REASON,
    );
    assert_route_explain_contract_blocks_presence(&parsed);
    assert_operator_summary_invariants(&parsed);
    assert!(json.contains(&format!(
        "\"explain_contract_version\":\"{}\"",
        expected_contract_version()
    )));
    assert!(json.contains("\"table_runtime_consistency_gate\":\""));
    assert!(json.contains("\"table_runtime_consistency_all_true\":\""));
    assert!(json.contains("\"table_runtime_consistency_summary\":\"gate="));
    assert!(json.contains("\"plan_setup_discovery_table_compact\":\"join_mode:"));
    assert!(json.contains("\"plan_setup_discovery_table_compact_consistency\":\""));
    assert!(json.contains("\"plan_setup_discovery_table_compact_consistency_match\":\""));
    assert!(json.contains("\"plan_setup_discovery_table_compact_consistency_match_source\":\""));
    assert!(json.contains("\"status_plan_setup_compact_consistency_match_source\":\""));
    assert!(json.contains("\"status_setup_compact_consistency_match_source\":\""));
    assert!(json.contains("\"status_shadow_setup_match_source_from_compact\":\""));
    assert!(json.contains("\"status_shadow_plan_setup_match_source_from_compact\":\""));
    assert!(json.contains("\"dps_payload_plan_setup_compact_consistency\":\""));
    assert!(json.contains("\"setup_compact_consistency_match\":\""));
    assert!(json.contains("\"setup_compact_consistency_match_source\":\""));
    assert!(json.contains("\"dps_payload_plan_setup_compact_consistency_match\":\""));
    assert!(json.contains("\"dps_payload_plan_setup_compact_consistency_match_source\":\""));
    assert!(json.contains("\"dps_payload_setup_compact_consistency_match\":\""));
    assert!(json.contains("\"dps_payload_setup_compact_consistency_match_source\":\""));
    assert!(json.contains("\"dps_shadow_setup_match_source_from_compact\":\""));
    assert!(json.contains("\"dps_shadow_plan_setup_match_source_from_compact\":\""));
    assert!(json.contains("\"consistency_source_matrix\":\""));
    assert!(json.contains("\"dps_payload_preemptive_shadow_compact\":\""));
    assert!(json.contains("\"preemptive_shadow_degraded_path\":\""));
    assert!(json.contains("\"preemptive_shadow_degraded_reason\":\""));
    assert!(json.contains("\"preemptive_shadow_degraded_summary\":\"path="));
    assert!(json.contains("\"preemptive_shadow_switch_candidate_confidence\":\""));
    assert!(json.contains("\"preemptive_shadow_switch_confidence_gate_min\":\""));
    assert!(json.contains("\"preemptive_shadow_switch_confidence_gate_passed\":\""));
    assert!(json.contains("\"preemptive_shadow_switch_candidate_sample_age_ticks\":\""));
    assert!(json.contains("\"preemptive_shadow_switch_confidence_summary\":\""));
    assert!(json.contains("\"preemptive_shadow_switch_block_reason_chain\":\""));
    assert!(json.contains("\"preemptive_shadow_candidate_readiness_summary\":\""));
    assert!(json.contains("\"auto_recovery_attempts\":\""));
    assert!(json.contains("\"auto_recovery_final_result\":\""));
    assert!(json.contains("\"connect_retry_budget_exhausted\":\""));
    assert!(json.contains("\"connect_recovery_needed\":\""));
    assert!(json.contains("\"connect_recovery_strategy\":\""));
    assert!(json.contains("\"connect_recovery_projection_consistency\":\""));
    assert!(json.contains("\"connect_recovery_projection_key\":\""));
    assert!(json.contains("\"route_explain_recovery_schema_version\":\""));
    assert!(json.contains("\"route_explain_recovery_fields_checksum\":\""));
    assert!(json.contains("\"selection_pressure_summary\":\"considered:"));
    assert!(json.contains("\"selection_pressure_level\":\""));
    assert!(json.contains("\"selection_pressure_score\":\""));
    assert!(json.contains("\"selection_pressure_dominant\":\""));
    assert!(json.contains("\"selection_pressure_action_hint\":\""));
    assert!(json.contains("\"selection_pressure_compact\":\"level:"));
    assert!(json.contains("\"selection_pressure_reason\":\"level="));
    assert!(json.contains("\"selected_peer_connect_retry_plan\":\""));
    assert!(json.contains("\"selected_peer_connect_backoff_profile\":\""));
    assert!(json.contains("\"dps_payload_selection_pressure_summary\":\"considered:"));
    assert!(json.contains("\"dps_payload_selection_pressure_level\":\""));
    assert!(json.contains("\"dps_payload_selection_pressure_score\":\""));
    assert!(json.contains("\"dps_payload_selection_pressure_dominant\":\""));
    assert!(json.contains("\"dps_payload_selection_pressure_action_hint\":\""));
    assert!(json.contains("\"dps_payload_selection_pressure_compact\":\"level:"));
    assert!(json.contains("\"dps_payload_selection_pressure_reason\":\"level="));
    assert!(json.contains(&format!(
        "\"selection_pressure_projection_consistency\":\"{}\"",
        PROJECTION_CONSISTENCY_ALL_TRUE
    )));
    assert!(json.contains(&format!(
        "\"selection_pressure_projection_gate\":\"{}\"",
        expected_status_ok()
    )));
    assert!(json.contains(&format!(
        "\"route_explain_health_gate\":\"{}\"",
        expected_status_ok()
    )));
    assert!(json.contains(&format!(
        "\"route_explain_health_summary\":\"{}\"",
        expected_health_summary_ok()
    )));
    assert!(json.contains(&format!(
        "\"route_explain_operator_summary\":\"health:{};selected:{};pressure:",
        expected_status_ok(),
        SUCCESS_SELECTED_NODE
    )));
    assert!(json.contains(&format!(
        "\"route_explain_operator_signature\":\"{}\"",
        expected_operator_summary(
            expected_status_ok(),
            SUCCESS_SELECTED_NODE,
            SUCCESS_PRESSURE,
            SUCCESS_OPERATOR_ACTION,
            SUCCESS_OPERATOR_REASON
        )
    )));
    assert!(json.contains(&format!(
        "\"route_explain_operator_route_key\":\"{}\"",
        expected_operator_route_key_ok(SUCCESS_OPERATOR_ACTION)
    )));
    assert!(json.contains(&format!(
        "\"route_explain_operator_health\":\"{}\"",
        expected_status_ok()
    )));
    assert!(json.contains(&format!(
        "\"route_explain_operator_selected\":\"{}\"",
        SUCCESS_SELECTED_NODE
    )));
    assert!(json.contains(&format!(
        "\"route_explain_operator_pressure\":\"{}\"",
        SUCCESS_PRESSURE
    )));
    assert!(json.contains(&format!(
        "\"route_explain_operator_action\":\"{}\"",
        SUCCESS_OPERATOR_ACTION
    )));
    assert!(json.contains(&format!(
        "\"route_explain_operator_reason\":\"{}\"",
        SUCCESS_OPERATOR_REASON
    )));
    assert!(json.contains(&format!(
        "\"route_explain_contract_integrity\":\"{}\"",
        expected_integrity_all_true()
    )));
    assert!(json.contains("\"auto_recovery_attempts\":\"0\""));
    assert!(json.contains("\"auto_recovery_final_result\":\"not_triggered\""));
    assert!(json.contains("\"connect_retry_budget_exhausted\":\"false\""));
    assert!(json.contains("\"connect_recovery_needed\":\"false\""));
    assert!(json.contains("\"connect_recovery_strategy\":\"none\""));
    assert!(json.contains("\"connect_recovery_projection_consistency\":\"true\""));
    assert!(json.contains("\"connect_recovery_projection_key\":\"needed:false;strategy:none;action:use_selected_path\""));
    assert!(json.contains("\"selected_peer_connect_retry_plan\":\"n1@198.51.100.1:443:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=443|8443\""));
    assert!(json.contains("\"selected_peer_connect_backoff_profile\":\"initial=0ms;retry1=250ms;retry2=1000ms;jitter_step=50ms;fanout=1\""));
    assert!(json.contains(&format!(
        "\"route_explain_recovery_schema_version\":\"{}\"",
        expected_recovery_schema_version()
    )));
    assert!(json.contains(&format!(
        "\"route_explain_recovery_fields_checksum\":\"{}\"",
        expected_recovery_fields_checksum()
    )));
    assert!(json.contains(&format!(
        "action:{};reason:{}\"",
        SUCCESS_OPERATOR_ACTION, SUCCESS_OPERATOR_REASON
    )));
    assert!(json.contains(";gate="));
    assert!(json.contains(";all_true="));
    assert!(json.contains(&format!(
        "\"network_state\":\"{}\"",
        expected_network_state()
    )));
}
