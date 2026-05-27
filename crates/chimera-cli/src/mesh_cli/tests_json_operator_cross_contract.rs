use super::tests_contract_constants::{
    ACTION_INSPECT_ERROR, ERROR_ACTION_ADJUST_POLICY, STAGE_OPTIONS_PARSE, SUCCESS_OPERATOR_ACTION,
};
use super::tests_json_error_utils::{
    assert_error_contract_consistency, assert_error_health_projection_consistency,
};
use super::tests_json_runner_utils::run_route_explain_json;
use super::tests_json_utils::{
    assert_health_pressure_projection_consistency, assert_operator_summary_invariants,
    assert_route_explain_contract_blocks_presence, assert_route_explain_envelope,
    base_route_explain_args, expected_integrity_all_true, expected_kind_error, expected_kind_ok,
    expected_recovery_fields_checksum, expected_recovery_schema_version, expected_status_error,
    expected_status_ok, summary_field,
};

#[test]
fn mesh_route_explain_json_operator_contract_cross_envelope_consistency() {
    let success = run_route_explain_json(
        base_route_explain_args("allow=mesh;mesh_max_peers=1"),
        0,
        "operator_cross_success",
    );
    let error = run_route_explain_json(
        base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95"),
        2,
        "operator_cross_error",
    );
    let options_parse_error = run_route_explain_json(
        vec![
            "--namespace".to_string(),
            "cef-public".to_string(),
            "--node".to_string(),
            "node-client".to_string(),
            "--peer".to_string(),
            "n1@198.51.100.1:443@eu@20@90".to_string(),
        ],
        2,
        "operator_cross_options_parse_error",
    );

    for parsed in [&success, &error, &options_parse_error] {
        let expected_kind = if parsed["status"] == expected_status_ok() {
            expected_kind_ok()
        } else {
            expected_kind_error()
        };
        assert_route_explain_envelope(
            parsed,
            parsed["status"].as_str().unwrap_or(""),
            expected_kind,
        );
        assert_health_pressure_projection_consistency(parsed);
        assert_route_explain_contract_blocks_presence(parsed);
        assert_operator_summary_invariants(parsed);
    }
    assert_error_health_projection_consistency(&error);
    assert_error_health_projection_consistency(&options_parse_error);
    assert_error_contract_consistency(&error);
    assert_error_contract_consistency(&options_parse_error);

    assert_eq!(
        success["route_explain_operator_health"]
            .as_str()
            .unwrap_or_default(),
        expected_status_ok()
    );
    assert_eq!(
        error["route_explain_operator_health"]
            .as_str()
            .unwrap_or_default(),
        expected_status_error()
    );
    assert_eq!(
        success["route_explain_operator_action"]
            .as_str()
            .unwrap_or_default(),
        SUCCESS_OPERATOR_ACTION
    );
    assert_eq!(
        error["route_explain_operator_action"]
            .as_str()
            .unwrap_or_default(),
        ERROR_ACTION_ADJUST_POLICY
    );
    assert_eq!(
        options_parse_error["route_explain_operator_action"]
            .as_str()
            .unwrap_or_default(),
        ACTION_INSPECT_ERROR
    );
    assert_eq!(
        options_parse_error["error_stage"]
            .as_str()
            .unwrap_or_default(),
        STAGE_OPTIONS_PARSE
    );
    assert_eq!(
        success["route_explain_contract_integrity"]
            .as_str()
            .unwrap_or_default(),
        expected_integrity_all_true()
    );
    assert_eq!(
        error["route_explain_contract_integrity"]
            .as_str()
            .unwrap_or_default(),
        expected_integrity_all_true()
    );

    // Success envelope must carry first-class recovery projection fields.
    let success_attempts = success["auto_recovery_attempts"]
        .as_str()
        .unwrap_or_default();
    assert!(!success_attempts.is_empty());
    assert!(success_attempts.parse::<usize>().is_ok());
    let success_final = success["auto_recovery_final_result"]
        .as_str()
        .unwrap_or_default();
    assert!(!success_final.is_empty());
    let success_exhausted = success["connect_retry_budget_exhausted"]
        .as_str()
        .unwrap_or_default();
    assert!(matches!(success_exhausted, "true" | "false"));
    let success_recovery_needed = success["connect_recovery_needed"]
        .as_str()
        .unwrap_or_default();
    assert!(matches!(success_recovery_needed, "true" | "false"));
    let success_recovery_strategy = success["connect_recovery_strategy"]
        .as_str()
        .unwrap_or_default();
    assert!(matches!(
        success_recovery_strategy,
        "none" | "retry_connect_endpoints"
    ));
    if success_recovery_needed == "true" {
        assert_eq!(success_recovery_strategy, "retry_connect_endpoints");
    } else {
        assert_eq!(success_recovery_strategy, "none");
    }
    let success_projection_consistency = success["connect_recovery_projection_consistency"]
        .as_str()
        .unwrap_or_default();
    assert_eq!(success_projection_consistency, "true");
    let success_projection_key = success["connect_recovery_projection_key"]
        .as_str()
        .unwrap_or_default();
    if success_recovery_needed == "true" {
        assert_eq!(
            success_projection_key,
            "needed:true;strategy:retry_connect_endpoints;action:retry_connect_endpoints"
        );
    } else {
        assert_eq!(
            success_projection_key,
            "needed:false;strategy:none;action:use_selected_path"
        );
    }
    let recovery_schema = success["route_explain_recovery_schema_version"]
        .as_str()
        .unwrap_or_default();
    assert_eq!(recovery_schema, expected_recovery_schema_version());
    let recovery_checksum = success["route_explain_recovery_fields_checksum"]
        .as_str()
        .unwrap_or_default();
    assert_eq!(recovery_checksum, expected_recovery_fields_checksum());

    // Error envelopes keep recovery schema/checksum and a deterministic
    // no-recovery projection for contract parity.
    for parsed in [&error, &options_parse_error] {
        let action = parsed["route_explain_error_action"]
            .as_str()
            .unwrap_or_default();
        assert_eq!(
            parsed["auto_recovery_attempts"]
                .as_str()
                .unwrap_or_default(),
            "0"
        );
        assert_eq!(
            parsed["auto_recovery_final_result"]
                .as_str()
                .unwrap_or_default(),
            "not_applicable_error"
        );
        assert_eq!(
            parsed["connect_retry_budget_exhausted"]
                .as_str()
                .unwrap_or_default(),
            "unknown"
        );
        assert_eq!(
            parsed["connect_recovery_needed"]
                .as_str()
                .unwrap_or_default(),
            "false"
        );
        assert_eq!(
            parsed["connect_recovery_strategy"]
                .as_str()
                .unwrap_or_default(),
            "none"
        );
        assert_eq!(
            parsed["connect_recovery_projection_consistency"]
                .as_str()
                .unwrap_or_default(),
            "true"
        );
        assert_eq!(
            parsed["connect_recovery_projection_key"]
                .as_str()
                .unwrap_or_default(),
            format!("needed:false;strategy:none;action:{action}")
        );
        assert_eq!(
            parsed["route_explain_recovery_schema_version"]
                .as_str()
                .unwrap_or_default(),
            expected_recovery_schema_version()
        );
        assert_eq!(
            parsed["route_explain_recovery_fields_checksum"]
                .as_str()
                .unwrap_or_default(),
            expected_recovery_fields_checksum()
        );
    }
}

#[test]
fn mesh_route_explain_json_operator_summary_is_consistent_across_success_and_error_stages() {
    let success = run_route_explain_json(
        base_route_explain_args("allow=mesh;mesh_max_peers=1"),
        0,
        "operator_summary_consistency_success",
    );
    let plan_error = run_route_explain_json(
        base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95"),
        2,
        "operator_summary_consistency_plan_error",
    );
    let policy_error = run_route_explain_json(
        base_route_explain_args("mesh_max_peers=0"),
        2,
        "operator_summary_consistency_policy_error",
    );
    let simulation_error = run_route_explain_json(
        {
            let mut args = base_route_explain_args("allow=mesh;mesh_max_peers=1");
            args.extend(["--failed-node".to_string(), "n-missing".to_string()]);
            args
        },
        2,
        "operator_summary_consistency_simulation_error",
    );

    for parsed in [&success, &plan_error, &policy_error, &simulation_error] {
        let status = parsed["status"].as_str().unwrap_or_default();
        let summary = parsed["route_explain_operator_summary"]
            .as_str()
            .unwrap_or_default();
        let signature = parsed["route_explain_operator_signature"]
            .as_str()
            .unwrap_or_default();
        let action = parsed["route_explain_operator_action"]
            .as_str()
            .unwrap_or_default();
        let reason = parsed["route_explain_operator_reason"]
            .as_str()
            .unwrap_or_default();
        let route_key = parsed["route_explain_operator_route_key"]
            .as_str()
            .unwrap_or_default();
        let health = parsed["route_explain_operator_health"]
            .as_str()
            .unwrap_or_default();
        let selected = parsed["route_explain_operator_selected"]
            .as_str()
            .unwrap_or_default();
        let pressure = parsed["route_explain_operator_pressure"]
            .as_str()
            .unwrap_or_default();

        assert_eq!(summary, signature);
        assert_eq!(summary_field(summary, "health"), Some(health));
        assert_eq!(summary_field(summary, "selected"), Some(selected));
        assert_eq!(summary_field(summary, "pressure"), Some(pressure));
        assert_eq!(summary_field(summary, "action"), Some(action));
        assert_eq!(summary_field(summary, "reason"), Some(reason));
        assert_eq!(route_key, format!("{health}:{action}"));

        if status == expected_status_ok() {
            assert_eq!(health, expected_status_ok());
        } else {
            assert_eq!(status, expected_status_error());
            assert_eq!(health, expected_status_error());
            let error_action = parsed["route_explain_error_action"]
                .as_str()
                .unwrap_or_default();
            let error_stage = parsed["error_stage"].as_str().unwrap_or_default();
            assert_eq!(action, error_action);
            assert_eq!(reason, error_stage);
        }
    }
}
