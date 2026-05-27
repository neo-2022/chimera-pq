use super::tests_contract_constants::{
    ERROR_ACTION_ADJUST_POLICY, ERROR_PRESSURE_UNKNOWN, ERROR_SELECTED_NONE, ERROR_STAGE_PLAN_PATH,
};
use super::tests_json_error_utils::{
    assert_error_contract_consistency, assert_error_health_projection_consistency,
    assert_error_operator_health_block, assert_error_plan_path_contract,
};
use super::tests_json_runner_utils::run_route_explain_json;
use super::tests_json_utils::{
    assert_health_summary_shape, assert_operator_summary_invariants,
    assert_route_explain_contract_blocks_presence, base_route_explain_args,
    expected_contract_family, expected_contract_version, expected_integrity_all_true,
    expected_kind_error, expected_network_state, expected_operator_route_key_error,
    expected_operator_summary, expected_status_error, summary_field,
};

#[test]
fn mesh_route_explain_json_error_snapshot_core() {
    let parsed = run_route_explain_json(
        base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95"),
        2,
        "error_snapshot_core",
    );
    assert_route_explain_contract_blocks_presence(&parsed);

    let snapshot = format!(
        "status={};kind={};family={};contract={};stage={};action={};operator={};integrity={};state={}",
        parsed["status"].as_str().unwrap_or(""),
        parsed["kind"].as_str().unwrap_or(""),
        parsed["route_explain_contract_family"]
            .as_str()
            .unwrap_or(""),
        parsed["explain_contract_version"].as_str().unwrap_or(""),
        parsed["error_stage"].as_str().unwrap_or(""),
        parsed["route_explain_error_action"].as_str().unwrap_or(""),
        parsed["route_explain_operator_summary"]
            .as_str()
            .unwrap_or(""),
        parsed["route_explain_contract_integrity"]
            .as_str()
            .unwrap_or(""),
        parsed["network_state"].as_str().unwrap_or(""),
    );

    let expected_operator = expected_operator_summary(
        expected_status_error(),
        ERROR_SELECTED_NONE,
        ERROR_PRESSURE_UNKNOWN,
        ERROR_ACTION_ADJUST_POLICY,
        ERROR_STAGE_PLAN_PATH,
    );
    let expected_snapshot = format!(
        "status={};kind={};family={};contract={};stage={};action={};operator={};integrity={};state={}",
        expected_status_error(),
        expected_kind_error(),
        expected_contract_family(),
        expected_contract_version(),
        ERROR_STAGE_PLAN_PATH,
        ERROR_ACTION_ADJUST_POLICY,
        expected_operator,
        expected_integrity_all_true(),
        expected_network_state(),
    );
    assert_eq!(snapshot, expected_snapshot);
}

#[test]
fn mesh_route_explain_json_operator_summary_contract_error() {
    let parsed = run_route_explain_json(
        base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95"),
        2,
        "operator_summary_error",
    );
    let summary = parsed["route_explain_operator_summary"]
        .as_str()
        .unwrap_or("");
    let signature = parsed["route_explain_operator_signature"]
        .as_str()
        .unwrap_or("");
    let route_key = parsed["route_explain_operator_route_key"]
        .as_str()
        .unwrap_or("");
    assert_route_explain_contract_blocks_presence(&parsed);

    assert_eq!(
        summary_field(summary, "health"),
        Some(expected_status_error())
    );
    assert_eq!(
        summary_field(summary, "selected"),
        Some(ERROR_SELECTED_NONE)
    );
    assert_eq!(
        summary_field(summary, "pressure"),
        Some(ERROR_PRESSURE_UNKNOWN)
    );
    assert_eq!(
        summary_field(summary, "action"),
        Some(ERROR_ACTION_ADJUST_POLICY)
    );
    assert_eq!(
        summary_field(summary, "reason"),
        Some(ERROR_STAGE_PLAN_PATH)
    );
    assert_eq!(summary, signature);
    assert_eq!(
        route_key,
        expected_operator_route_key_error(ERROR_ACTION_ADJUST_POLICY)
    );
    assert_error_operator_health_block(&parsed, ERROR_ACTION_ADJUST_POLICY, ERROR_STAGE_PLAN_PATH);
    assert_health_summary_shape(
        parsed["route_explain_health_summary"]
            .as_str()
            .unwrap_or_default(),
    );
    assert_error_health_projection_consistency(&parsed);
    assert_error_contract_consistency(&parsed);
    assert_error_plan_path_contract(&parsed);
    assert_operator_summary_invariants(&parsed);
}
