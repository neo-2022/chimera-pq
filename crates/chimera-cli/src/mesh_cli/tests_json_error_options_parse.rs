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
fn mesh_route_explain_json_options_parse_contract_matrix() {
    let parsed = run_route_explain_json(
        {
            let mut args =
                base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
            args.extend(["--namespace".to_string(), "cef-alt".to_string()]);
            args
        },
        2,
        "error_matrix_options_parse_duplicate_namespace",
    );
    assert_options_parse_error_contract(&parsed, "duplicate singleton flag '--namespace'");

    for (label, args, expected_error) in [
        (
            "error_matrix_options_parse_unknown_flag",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend(["--mystery-flag".to_string(), "x".to_string()]);
                args
            },
            "unknown flag '--mystery-flag'",
        ),
        (
            "error_matrix_options_parse_positional",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.push("positional".to_string());
                args
            },
            "unexpected positional argument 'positional'",
        ),
        (
            "error_matrix_options_parse_missing_policy_payload",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
            ],
            "missing --policy-payload",
        ),
        (
            "error_matrix_options_parse_missing_namespace_precedes_missing_peer",
            vec![
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=95".to_string(),
            ],
            "missing --namespace",
        ),
        (
            "error_matrix_options_parse_missing_node_precedes_missing_peer",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=95".to_string(),
            ],
            "missing --node",
        ),
        (
            "error_matrix_options_parse_missing_policy_precedes_missing_peer",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
            ],
            "missing --policy-payload",
        ),
        (
            "error_matrix_options_parse_blank_policy_payload",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "   ".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
            ],
            "blank value for flag '--policy-payload'",
        ),
        (
            "error_matrix_options_parse_missing_peer",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=95".to_string(),
            ],
            "at least one --peer is required",
        ),
        (
            "error_matrix_options_parse_invalid_table_max_entries",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend(["--table-max-entries".to_string(), "abc".to_string()]);
                args
            },
            "invalid --table-max-entries value 'abc'",
        ),
        (
            "error_matrix_options_parse_invalid_table_max_entries_trimmed",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend(["--table-max-entries".to_string(), "  abc  ".to_string()]);
                args
            },
            "invalid --table-max-entries value 'abc'",
        ),
        (
            "error_matrix_options_parse_invalid_table_max_per_region",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend(["--table-max-per-region".to_string(), "xyz".to_string()]);
                args
            },
            "invalid --table-max-per-region value 'xyz'",
        ),
        (
            "error_matrix_options_parse_invalid_table_max_per_region_trimmed",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend(["--table-max-per-region".to_string(), "  xyz  ".to_string()]);
                args
            },
            "invalid --table-max-per-region value 'xyz'",
        ),
        (
            "error_matrix_options_parse_invalid_table_stale_after",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend(["--table-stale-after".to_string(), "oops".to_string()]);
                args
            },
            "invalid --table-stale-after value 'oops'",
        ),
        (
            "error_matrix_options_parse_invalid_table_stale_after_trimmed",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend(["--table-stale-after".to_string(), "  oops  ".to_string()]);
                args
            },
            "invalid --table-stale-after value 'oops'",
        ),
        (
            "error_matrix_options_parse_blank_table_max_entries",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend(["--table-max-entries".to_string(), "   ".to_string()]);
                args
            },
            "blank value for flag '--table-max-entries'",
        ),
        (
            "error_matrix_options_parse_blank_table_max_per_region",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend(["--table-max-per-region".to_string(), "   ".to_string()]);
                args
            },
            "blank value for flag '--table-max-per-region'",
        ),
        (
            "error_matrix_options_parse_blank_table_stale_after",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend(["--table-stale-after".to_string(), "   ".to_string()]);
                args
            },
            "blank value for flag '--table-stale-after'",
        ),
        (
            "error_matrix_options_parse_duplicate_table_max_entries",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend([
                    "--table-max-entries".to_string(),
                    "10".to_string(),
                    "--table-max-entries".to_string(),
                    "20".to_string(),
                ]);
                args
            },
            "duplicate singleton flag '--table-max-entries'",
        ),
        (
            "error_matrix_options_parse_duplicate_table_max_per_region",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend([
                    "--table-max-per-region".to_string(),
                    "7".to_string(),
                    "--table-max-per-region".to_string(),
                    "8".to_string(),
                ]);
                args
            },
            "duplicate singleton flag '--table-max-per-region'",
        ),
        (
            "error_matrix_options_parse_duplicate_table_stale_after",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend([
                    "--table-stale-after".to_string(),
                    "3".to_string(),
                    "--table-stale-after".to_string(),
                    "4".to_string(),
                ]);
                args
            },
            "duplicate singleton flag '--table-stale-after'",
        ),
        (
            "error_matrix_options_parse_duplicate_invite_token",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend([
                    "--invite-token".to_string(),
                    "tok-a".to_string(),
                    "--invite-token".to_string(),
                    "tok-b".to_string(),
                ]);
                args
            },
            "duplicate singleton flag '--invite-token'",
        ),
        (
            "error_matrix_options_parse_duplicate_cooldown_node",
            {
                let mut args =
                    base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
                args.extend([
                    "--cooldown-node".to_string(),
                    "n2".to_string(),
                    "--cooldown-node".to_string(),
                    "n3".to_string(),
                ]);
                args
            },
            "duplicate singleton flag '--cooldown-node'",
        ),
    ] {
        let parsed = run_route_explain_json(args, 2, label);
        assert_options_parse_error_contract(&parsed, expected_error);
    }

    let parsed = run_route_explain_json(
        vec![
            "--node".to_string(),
            "node-client".to_string(),
            "--policy-payload".to_string(),
            "allow=mesh;mesh_max_peers=1;mesh_min_reliability=95".to_string(),
        ],
        2,
        "error_matrix_options_parse_namespace_unknown_fallback",
    );
    assert_options_parse_error_contract(&parsed, "missing --namespace");
    assert_eq!(parsed["namespace"].as_str().unwrap_or_default(), "unknown");
    assert_eq!(parsed["node"].as_str().unwrap_or_default(), "node-client");

    let parsed = run_route_explain_json(
        vec![
            "--namespace".to_string(),
            "cef-public".to_string(),
            "--policy-payload".to_string(),
            "allow=mesh;mesh_max_peers=1;mesh_min_reliability=95".to_string(),
        ],
        2,
        "error_matrix_options_parse_node_unknown_fallback",
    );
    assert_options_parse_error_contract(&parsed, "missing --node");
    assert_eq!(
        parsed["namespace"].as_str().unwrap_or_default(),
        "cef-public"
    );
    assert_eq!(parsed["node"].as_str().unwrap_or_default(), "unknown");

    let parsed = run_route_explain_json(
        vec![
            "--policy-payload".to_string(),
            "allow=mesh;mesh_max_peers=1;mesh_min_reliability=95".to_string(),
        ],
        2,
        "error_matrix_options_parse_both_unknown_fallback",
    );
    assert_options_parse_error_contract(&parsed, "missing --namespace");
    assert_eq!(parsed["namespace"].as_str().unwrap_or_default(), "unknown");
    assert_eq!(parsed["node"].as_str().unwrap_or_default(), "unknown");

    let parsed = run_route_explain_json(
        vec![
            "--namespace".to_string(),
            "   ".to_string(),
            "--node".to_string(),
            "node-client".to_string(),
            "--policy-payload".to_string(),
            "allow=mesh;mesh_max_peers=1;mesh_min_reliability=95".to_string(),
        ],
        2,
        "error_matrix_options_parse_blank_namespace_unknown_fallback",
    );
    assert_options_parse_error_contract(&parsed, "blank value for flag '--namespace'");
    assert_eq!(parsed["namespace"].as_str().unwrap_or_default(), "unknown");
    assert_eq!(parsed["node"].as_str().unwrap_or_default(), "node-client");

    let parsed = run_route_explain_json(
        vec![
            "--namespace".to_string(),
            "cef-public".to_string(),
            "--node".to_string(),
            "   ".to_string(),
            "--policy-payload".to_string(),
            "allow=mesh;mesh_max_peers=1;mesh_min_reliability=95".to_string(),
        ],
        2,
        "error_matrix_options_parse_blank_node_unknown_fallback",
    );
    assert_options_parse_error_contract(&parsed, "blank value for flag '--node'");
    assert_eq!(
        parsed["namespace"].as_str().unwrap_or_default(),
        "cef-public"
    );
    assert_eq!(parsed["node"].as_str().unwrap_or_default(), "unknown");

    let parsed = run_route_explain_json(
        vec![
            "--namespace".to_string(),
            "   ".to_string(),
            "--node".to_string(),
            "   ".to_string(),
            "--policy-payload".to_string(),
            "allow=mesh;mesh_max_peers=1;mesh_min_reliability=95".to_string(),
        ],
        2,
        "error_matrix_options_parse_blank_namespace_and_node_unknown_fallback",
    );
    assert_options_parse_error_contract(&parsed, "blank value for flag '--namespace'");
    assert_eq!(parsed["namespace"].as_str().unwrap_or_default(), "unknown");
    assert_eq!(parsed["node"].as_str().unwrap_or_default(), "unknown");
}
