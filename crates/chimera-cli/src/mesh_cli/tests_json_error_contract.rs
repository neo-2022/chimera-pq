use super::tests_contract_constants::{
    ACTION_FIX_PEER_SPEC, ACTION_FIX_POLICY_PAYLOAD, ACTION_FIX_TABLE_POLICY,
    ACTION_INSPECT_DISCOVERY_INPUT, BACKOFF_AFTER_FIX, CATEGORY_INPUT, CATEGORY_POLICY,
    ERROR_ACTION_ADJUST_POLICY, ERROR_PRESSURE_UNKNOWN, ERROR_SELECTED_NONE, ERROR_STAGE_PLAN_PATH,
    RESOLUTION_CHECK_PEER_SPEC, RESOLUTION_INSPECT_DISCOVERY_RECORDS,
    RESOLUTION_POLICY_PAYLOAD_SYNTAX, RESOLUTION_TABLE_POLICY_BOUNDS, RETRIABLE_FALSE,
    STAGE_PEER_SPEC, STAGE_PEER_TABLE_POLICY, STAGE_POLICY_PARSE, STAGE_SIMULATION_INPUT,
};
use super::tests_json_error_utils::{
    assert_error_contract_consistency, assert_error_health_projection_consistency,
    assert_error_operator_health_block, assert_error_plan_path_contract,
};
use super::tests_json_runner_utils::run_route_explain_json;
use super::tests_json_utils::{
    assert_health_summary_shape, assert_operator_summary_invariants,
    assert_route_explain_contract_blocks_presence, assert_route_explain_envelope,
    base_route_explain_args, expected_integrity_all_true, expected_kind_error,
    expected_network_state, expected_operator_route_key_error, expected_operator_summary,
    expected_status_error,
};

struct ExpectedErrorContract<'a> {
    stage: &'a str,
    action: &'a str,
    stage_category: &'a str,
    retriable: &'a str,
    retry_backoff_hint: &'a str,
    resolution_hint: &'a str,
}

fn assert_error_stage_action(label: &str, args: Vec<String>, expected: ExpectedErrorContract<'_>) {
    let parsed = run_route_explain_json(args, 2, label);

    assert_route_explain_envelope(&parsed, expected_status_error(), expected_kind_error());
    assert_eq!(parsed["error_stage"], expected.stage);
    assert_eq!(parsed["route_explain_error_action"], expected.action);
    assert_eq!(
        parsed["route_explain_error_stage_category"],
        expected.stage_category
    );
    assert_eq!(parsed["route_explain_error_retriable"], expected.retriable);
    assert_eq!(
        parsed["route_explain_error_retry_backoff_hint"],
        expected.retry_backoff_hint
    );
    assert_eq!(
        parsed["route_explain_error_resolution_hint"],
        expected.resolution_hint
    );
    assert_eq!(
        parsed["route_explain_error_signature"],
        format!(
            "stage:{};category:{};retriable:{};action:{}",
            expected.stage, expected.stage_category, expected.retriable, expected.action
        )
    );
    assert_eq!(
        parsed["route_explain_error_route_key"],
        format!("{}:{}", expected.stage_category, expected.action)
    );
    assert_error_operator_health_block(&parsed, expected.action, expected.stage);
    assert_eq!(
        parsed["route_explain_contract_integrity"],
        expected_integrity_all_true()
    );
    assert_route_explain_contract_blocks_presence(&parsed);
    assert_health_summary_shape(
        parsed["route_explain_health_summary"]
            .as_str()
            .unwrap_or_default(),
    );
    assert_error_health_projection_consistency(&parsed);
    assert_eq!(parsed["network_state"], expected_network_state());
    let operator = parsed["route_explain_operator_summary"]
        .as_str()
        .unwrap_or_default();
    let signature = parsed["route_explain_operator_signature"]
        .as_str()
        .unwrap_or_default();
    assert_eq!(operator, signature);
    assert!(operator.contains(&format!(
        "health:{};selected:{};pressure:{};",
        expected_status_error(),
        ERROR_SELECTED_NONE,
        ERROR_PRESSURE_UNKNOWN
    )));
    assert!(operator.contains(&format!("action:{};", expected.action)));
    assert!(operator.contains(&format!("reason:{}", expected.stage)));
    assert_operator_summary_invariants(&parsed);
    assert_error_contract_consistency(&parsed);
}

#[test]
fn mesh_route_explain_json_error_is_structured_and_safe() {
    let parsed = run_route_explain_json(
        base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95"),
        2,
        "json_error",
    );

    assert_route_explain_envelope(&parsed, expected_status_error(), expected_kind_error());
    assert_error_plan_path_contract(&parsed);
    assert_error_operator_health_block(&parsed, ERROR_ACTION_ADJUST_POLICY, ERROR_STAGE_PLAN_PATH);
    assert_eq!(
        parsed["route_explain_contract_integrity"],
        expected_integrity_all_true()
    );
    assert_route_explain_contract_blocks_presence(&parsed);
    assert_health_summary_shape(
        parsed["route_explain_health_summary"]
            .as_str()
            .unwrap_or_default(),
    );
    assert_error_health_projection_consistency(&parsed);
    assert_error_contract_consistency(&parsed);
    assert_eq!(parsed["network_state"], expected_network_state());
    let expected_operator = expected_operator_summary(
        expected_status_error(),
        ERROR_SELECTED_NONE,
        ERROR_PRESSURE_UNKNOWN,
        ERROR_ACTION_ADJUST_POLICY,
        ERROR_STAGE_PLAN_PATH,
    );
    assert_eq!(parsed["route_explain_operator_summary"], expected_operator);
    assert_eq!(
        parsed["route_explain_operator_signature"],
        expected_operator
    );
    assert_eq!(
        parsed["route_explain_operator_route_key"],
        expected_operator_route_key_error(ERROR_ACTION_ADJUST_POLICY)
    );
}

#[test]
fn mesh_route_explain_json_error_stage_action_matrix() {
    assert_error_stage_action(
        "error_matrix_peer_spec",
        {
            let mut args =
                base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
            args[7] = "bad-peer-spec".to_string();
            args
        },
        ExpectedErrorContract {
            stage: STAGE_PEER_SPEC,
            action: ACTION_FIX_PEER_SPEC,
            stage_category: CATEGORY_INPUT,
            retriable: RETRIABLE_FALSE,
            retry_backoff_hint: BACKOFF_AFTER_FIX,
            resolution_hint: RESOLUTION_CHECK_PEER_SPEC,
        },
    );

    for (label, peer_value, expected_error) in [
        (
            "error_matrix_peer_spec_blank_node_id",
            " @198.51.100.1:443@eu@20@90",
            "blank peer node_id",
        ),
        (
            "error_matrix_peer_spec_blank_endpoint",
            "n1@   @eu@20@90",
            "blank peer endpoint",
        ),
        (
            "error_matrix_peer_spec_blank_region",
            "n1@198.51.100.1:443@   @20@90",
            "blank peer region",
        ),
        (
            "error_matrix_peer_spec_invalid_load_score",
            "n1@198.51.100.1:443@eu@x@90",
            "invalid load score",
        ),
        (
            "error_matrix_peer_spec_invalid_reliability_score",
            "n1@198.51.100.1:443@eu@20@y",
            "invalid reliability score",
        ),
    ] {
        let mut args =
            base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
        args[7] = peer_value.to_string();
        let parsed = run_route_explain_json(args, 2, label);
        assert_route_explain_envelope(&parsed, expected_status_error(), expected_kind_error());
        assert_eq!(parsed["error_stage"], STAGE_PEER_SPEC);
        assert_eq!(parsed["route_explain_error_action"], ACTION_FIX_PEER_SPEC);
        assert_eq!(parsed["error"].as_str().unwrap_or_default(), expected_error);
    }

    assert_error_stage_action(
        "error_matrix_simulation_input",
        {
            let mut args =
                base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
            args.extend(["--failed-node".to_string(), "n-missing".to_string()]);
            args
        },
        ExpectedErrorContract {
            stage: STAGE_SIMULATION_INPUT,
            action: ACTION_INSPECT_DISCOVERY_INPUT,
            stage_category: CATEGORY_INPUT,
            retriable: RETRIABLE_FALSE,
            retry_backoff_hint: BACKOFF_AFTER_FIX,
            resolution_hint: RESOLUTION_INSPECT_DISCOVERY_RECORDS,
        },
    );

    assert_error_stage_action(
        "error_matrix_simulation_input_duplicate_peer",
        {
            let mut args =
                base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
            args.extend([
                "--peer".to_string(),
                "n1@198.51.100.3:443@eu-west@20@80".to_string(),
            ]);
            args
        },
        ExpectedErrorContract {
            stage: STAGE_SIMULATION_INPUT,
            action: ACTION_INSPECT_DISCOVERY_INPUT,
            stage_category: CATEGORY_INPUT,
            retriable: RETRIABLE_FALSE,
            retry_backoff_hint: BACKOFF_AFTER_FIX,
            resolution_hint: RESOLUTION_INSPECT_DISCOVERY_RECORDS,
        },
    );

    assert_error_stage_action(
        "error_matrix_simulation_input_conflicting_failed_and_cooldown",
        {
            let mut args =
                base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
            args.extend([
                "--failed-node".to_string(),
                "n1".to_string(),
                "--cooldown-node".to_string(),
                "n1".to_string(),
            ]);
            args
        },
        ExpectedErrorContract {
            stage: STAGE_SIMULATION_INPUT,
            action: ACTION_INSPECT_DISCOVERY_INPUT,
            stage_category: CATEGORY_INPUT,
            retriable: RETRIABLE_FALSE,
            retry_backoff_hint: BACKOFF_AFTER_FIX,
            resolution_hint: RESOLUTION_INSPECT_DISCOVERY_RECORDS,
        },
    );

    assert_error_stage_action(
        "error_matrix_policy_parse",
        base_route_explain_args("mesh_max_peers=0"),
        ExpectedErrorContract {
            stage: STAGE_POLICY_PARSE,
            action: ACTION_FIX_POLICY_PAYLOAD,
            stage_category: CATEGORY_POLICY,
            retriable: RETRIABLE_FALSE,
            retry_backoff_hint: BACKOFF_AFTER_FIX,
            resolution_hint: RESOLUTION_POLICY_PAYLOAD_SYNTAX,
        },
    );

    assert_error_stage_action(
        "error_matrix_policy_parse_connect_fallback_ports_invalid",
        base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_connect_fallback_ports=abc"),
        ExpectedErrorContract {
            stage: STAGE_POLICY_PARSE,
            action: ACTION_FIX_POLICY_PAYLOAD,
            stage_category: CATEGORY_POLICY,
            retriable: RETRIABLE_FALSE,
            retry_backoff_hint: BACKOFF_AFTER_FIX,
            resolution_hint: RESOLUTION_POLICY_PAYLOAD_SYNTAX,
        },
    );

    assert_error_stage_action(
        "error_matrix_table_policy",
        {
            let mut args =
                base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
            args.extend(["--table-max-entries".to_string(), "0".to_string()]);
            args
        },
        ExpectedErrorContract {
            stage: STAGE_PEER_TABLE_POLICY,
            action: ACTION_FIX_TABLE_POLICY,
            stage_category: CATEGORY_POLICY,
            retriable: RETRIABLE_FALSE,
            retry_backoff_hint: BACKOFF_AFTER_FIX,
            resolution_hint: RESOLUTION_TABLE_POLICY_BOUNDS,
        },
    );
}
