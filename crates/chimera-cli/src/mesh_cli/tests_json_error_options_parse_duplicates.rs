use super::tests_contract_constants::{
    ACTION_INSPECT_ERROR, BACKOFF_AFTER_FIX, CATEGORY_INPUT, RESOLUTION_INSPECT_ERROR_DETAILS,
    RETRIABLE_FALSE, STAGE_OPTIONS_PARSE,
};
use super::tests_json_error_utils::{
    assert_error_contract_consistency, assert_error_health_projection_consistency,
};
use super::tests_json_runner_utils::run_route_explain_json;
use super::tests_json_utils::{
    assert_route_explain_envelope, base_route_explain_args, expected_kind_error,
    expected_status_error,
};

fn assert_options_parse_error_contract(parsed: &serde_json::Value, expected_error: &str) {
    assert_route_explain_envelope(parsed, expected_status_error(), expected_kind_error());
    assert_eq!(parsed["error_stage"], STAGE_OPTIONS_PARSE);
    assert_eq!(parsed["route_explain_error_action"], ACTION_INSPECT_ERROR);
    assert_eq!(parsed["route_explain_error_stage_category"], CATEGORY_INPUT);
    assert_eq!(parsed["route_explain_error_retriable"], RETRIABLE_FALSE);
    assert_eq!(
        parsed["route_explain_error_retry_backoff_hint"],
        BACKOFF_AFTER_FIX
    );
    assert_eq!(
        parsed["route_explain_error_resolution_hint"],
        RESOLUTION_INSPECT_ERROR_DETAILS
    );
    assert_eq!(parsed["error"].as_str().unwrap_or_default(), expected_error);
    assert_error_health_projection_consistency(parsed);
    assert_error_contract_consistency(parsed);
}

#[test]
fn mesh_route_explain_json_options_parse_duplicate_singleton_matrix() {
    for (label, args, expected_error) in [
        (
            "error_matrix_options_parse_duplicate_node",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend(["--node".to_string(), "node-alt".to_string()]);
                args
            },
            "duplicate singleton flag '--node'",
        ),
        (
            "error_matrix_options_parse_duplicate_policy_payload",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend([
                    "--policy-payload".to_string(),
                    "allow=mesh;mesh_max_peers=2;mesh_min_reliability=80".to_string(),
                ]);
                args
            },
            "duplicate singleton flag '--policy-payload'",
        ),
        (
            "error_matrix_options_parse_duplicate_failed_node",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend([
                    "--failed-node".to_string(),
                    "n2".to_string(),
                    "--failed-node".to_string(),
                    "n3".to_string(),
                ]);
                args
            },
            "duplicate singleton flag '--failed-node'",
        ),
    ] {
        let parsed = run_route_explain_json(args, 2, label);
        assert_options_parse_error_contract(&parsed, expected_error);
    }
}
