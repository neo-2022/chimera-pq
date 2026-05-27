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
fn mesh_route_explain_json_options_parse_rejects_short_flag_shape() {
    let parsed = run_route_explain_json(
        {
            let mut args =
                base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
            args.extend(["-x".to_string(), "1".to_string()]);
            args
        },
        2,
        "error_matrix_options_parse_short_flag_shape",
    );
    assert_options_parse_error_contract(&parsed, "unexpected positional argument '-x'");
}

#[test]
fn mesh_route_explain_json_options_parse_rejects_double_dash_flag_shape() {
    let parsed = run_route_explain_json(
        {
            let mut args =
                base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
            args.extend(["--".to_string(), "1".to_string()]);
            args
        },
        2,
        "error_matrix_options_parse_double_dash_flag_shape",
    );
    assert_options_parse_error_contract(&parsed, "unknown flag '--'");
}

#[test]
fn mesh_route_explain_json_options_parse_rejects_json_with_inline_assignment() {
    let parsed = run_route_explain_json(
        {
            let mut args =
                base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
            args.push("--json=1".to_string());
            args
        },
        2,
        "error_matrix_options_parse_json_inline_assignment",
    );
    assert_options_parse_error_contract(&parsed, "unknown flag '--json=1'");
}

#[test]
fn mesh_route_explain_json_options_parse_namespace_fallback_is_unknown_when_next_token_is_flag() {
    let parsed = run_route_explain_json(
        vec![
            "--namespace".to_string(),
            "--node".to_string(),
            "node-client".to_string(),
            "--policy-payload".to_string(),
            "allow=mesh;mesh_max_peers=1;mesh_min_reliability=95".to_string(),
            "--peer".to_string(),
            "n1@198.51.100.1:443@eu@20@90".to_string(),
        ],
        2,
        "error_matrix_options_parse_namespace_next_token_is_flag",
    );
    assert_options_parse_error_contract(&parsed, "unexpected positional argument 'node-client'");
    assert_eq!(parsed["namespace"].as_str().unwrap_or_default(), "unknown");
    assert_eq!(parsed["node"].as_str().unwrap_or_default(), "node-client");
}

#[test]
fn mesh_route_explain_json_options_parse_node_fallback_is_unknown_when_next_token_is_flag() {
    let parsed = run_route_explain_json(
        vec![
            "--namespace".to_string(),
            "cef-public".to_string(),
            "--node".to_string(),
            "--policy-payload".to_string(),
            "allow=mesh;mesh_max_peers=1;mesh_min_reliability=95".to_string(),
            "--peer".to_string(),
            "n1@198.51.100.1:443@eu@20@90".to_string(),
        ],
        2,
        "error_matrix_options_parse_node_next_token_is_flag",
    );
    assert_options_parse_error_contract(
        &parsed,
        "unexpected positional argument 'allow=mesh;mesh_max_peers=1;mesh_min_reliability=95'",
    );
    assert_eq!(
        parsed["namespace"].as_str().unwrap_or_default(),
        "cef-public"
    );
    assert_eq!(parsed["node"].as_str().unwrap_or_default(), "unknown");
}

#[test]
fn mesh_route_explain_json_options_parse_namespace_fallback_is_unknown_when_value_is_missing() {
    let parsed = run_route_explain_json(
        vec!["--namespace".to_string()],
        2,
        "error_matrix_options_parse_namespace_missing_value_unknown_fallback",
    );
    assert_options_parse_error_contract(&parsed, "missing --node");
    assert_eq!(parsed["namespace"].as_str().unwrap_or_default(), "unknown");
    assert_eq!(parsed["node"].as_str().unwrap_or_default(), "unknown");
}

#[test]
fn mesh_route_explain_json_options_parse_node_fallback_is_unknown_when_value_is_missing() {
    let parsed = run_route_explain_json(
        vec![
            "--namespace".to_string(),
            "cef-public".to_string(),
            "--node".to_string(),
        ],
        2,
        "error_matrix_options_parse_node_missing_value_unknown_fallback",
    );
    assert_options_parse_error_contract(&parsed, "missing --policy-payload");
    assert_eq!(
        parsed["namespace"].as_str().unwrap_or_default(),
        "cef-public"
    );
    assert_eq!(parsed["node"].as_str().unwrap_or_default(), "unknown");
}

#[test]
fn mesh_route_explain_json_options_parse_rejects_positional_after_json_flag() {
    let parsed = run_route_explain_json(
        {
            let mut args =
                base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
            args.extend(["--json".to_string(), "extra".to_string()]);
            args
        },
        2,
        "error_matrix_options_parse_positional_after_json_flag",
    );
    assert_options_parse_error_contract(&parsed, "unexpected positional argument 'extra'");
}
