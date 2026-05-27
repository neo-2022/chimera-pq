#![forbid(unsafe_code)]

use super::tests_json_runner_utils::run_mesh_subcommand_json;

pub(super) fn assert_error_parity(connect: &serde_json::Value, launch: &serde_json::Value) {
    for key in [
        "status",
        "kind",
        "network_state",
        "error_stage",
        "error",
        "route_explain_error_action",
        "route_explain_error_stage_category",
        "route_explain_error_retriable",
        "route_explain_error_retry_backoff_hint",
        "route_explain_error_resolution_hint",
        "route_explain_error_signature",
        "route_explain_error_route_key",
        "route_explain_operator_summary",
        "route_explain_operator_signature",
        "route_explain_operator_route_key",
        "route_explain_health_gate",
        "route_explain_health_summary",
        "route_explain_contract_integrity",
        "route_explain_recovery_schema_version",
        "route_explain_recovery_fields_checksum",
        "connect_recovery_projection_key",
    ] {
        assert_eq!(launch[key], connect[key], "mismatch at key '{key}'");
    }

    // Error envelopes for connect-probe and launch-preflight should be fully
    // identical when input arguments are identical.
    assert_eq!(launch, connect, "full json parity mismatch");
}

fn run_error_parity_case_with_stage(case_id: &str, expected_stage: &str, args: Vec<String>) {
    let connect_id = format!("parity_matrix_{case_id}_connect");
    let launch_id = format!("parity_matrix_{case_id}_launch");
    let connect = run_mesh_subcommand_json("connect-probe", args.clone(), 2, &connect_id);
    let launch = run_mesh_subcommand_json("launch-preflight", args, 2, &launch_id);
    assert_error_parity(&connect, &launch);
    assert_eq!(
        connect["error_stage"], expected_stage,
        "connect wrong stage for {case_id}"
    );
    assert_eq!(
        launch["error_stage"], expected_stage,
        "launch wrong stage for {case_id}"
    );
}

fn base_core_args() -> Vec<String> {
    vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
    ]
}

mod stage_and_peer_matrix;

mod policy_and_path_matrix;

mod single_cases;
