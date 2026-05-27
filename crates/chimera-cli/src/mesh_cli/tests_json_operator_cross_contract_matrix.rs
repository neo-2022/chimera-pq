use super::route_explain_error::build_route_explain_error_json;
use super::tests_contract_constants::{
    ACTION_INSPECT_DISCOVERY_INPUT, ACTION_INSPECT_ERROR, ACTION_INSPECT_FAILOVER_INPUT,
    ACTION_INSPECT_HEALTH_INPUTS, ACTION_INSPECT_RUNTIME_BOOTSTRAP, BACKOFF_AFTER_FIX,
    BACKOFF_IMMEDIATE, BACKOFF_SHORT, CATEGORY_HEALTH, CATEGORY_PLANNING, CATEGORY_RUNTIME,
    CATEGORY_UNKNOWN, STAGE_DISCOVERY_MERGE, STAGE_FAILOVER_PLAN, STAGE_HEALTH_STATE_UPDATE,
    STAGE_RESELECTION_PLAN, STAGE_RUNTIME_BOOTSTRAP,
};
use super::tests_json_error_utils::{
    assert_error_contract_consistency, assert_error_health_projection_consistency,
};
use super::tests_json_runner_utils::run_route_explain_json;
use super::tests_json_utils::{
    assert_operator_summary_invariants, assert_route_explain_contract_blocks_presence,
    assert_route_explain_envelope, base_route_explain_args, expected_kind_error,
    expected_status_error, summary_field,
};
use crate::mesh_cli::options::MeshRouteExplainOptions;

#[test]
fn mesh_route_explain_operator_cross_contract_consistent_for_internal_error_stage_matrix() {
    struct ErrorCase<'a> {
        stage: &'a str,
        action: &'a str,
        category: &'a str,
        retriable: &'a str,
        backoff: &'a str,
    }

    let success = run_route_explain_json(
        base_route_explain_args("allow=mesh;mesh_max_peers=1"),
        0,
        "operator_cross_internal_error_matrix_success",
    );

    let options = MeshRouteExplainOptions {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
        policy_payload: "allow=mesh".to_string(),
        failed_node_id: None,
        cooldown_node_id: None,
        table_max_entries: None,
        table_max_entries_per_region: None,
        table_stale_after_ticks: None,
        connect_timeout_ms: None,
        peers: vec!["n1@198.51.100.1:443@eu@20@90".to_string()],
        json_output: true,
        out_path: None,
    };

    let error_cases = [
        ErrorCase {
            stage: STAGE_RUNTIME_BOOTSTRAP,
            action: ACTION_INSPECT_RUNTIME_BOOTSTRAP,
            category: CATEGORY_RUNTIME,
            retriable: "true",
            backoff: BACKOFF_IMMEDIATE,
        },
        ErrorCase {
            stage: STAGE_DISCOVERY_MERGE,
            action: ACTION_INSPECT_DISCOVERY_INPUT,
            category: CATEGORY_PLANNING,
            retriable: "true",
            backoff: BACKOFF_SHORT,
        },
        ErrorCase {
            stage: STAGE_FAILOVER_PLAN,
            action: ACTION_INSPECT_FAILOVER_INPUT,
            category: CATEGORY_PLANNING,
            retriable: "true",
            backoff: BACKOFF_SHORT,
        },
        ErrorCase {
            stage: STAGE_HEALTH_STATE_UPDATE,
            action: ACTION_INSPECT_HEALTH_INPUTS,
            category: CATEGORY_HEALTH,
            retriable: "true",
            backoff: BACKOFF_SHORT,
        },
        ErrorCase {
            stage: STAGE_RESELECTION_PLAN,
            action: ACTION_INSPECT_HEALTH_INPUTS,
            category: CATEGORY_HEALTH,
            retriable: "true",
            backoff: BACKOFF_SHORT,
        },
        ErrorCase {
            stage: "unknown_stage",
            action: ACTION_INSPECT_ERROR,
            category: CATEGORY_UNKNOWN,
            retriable: "false",
            backoff: BACKOFF_AFTER_FIX,
        },
    ];

    let success_summary = success["route_explain_operator_summary"]
        .as_str()
        .unwrap_or_default();
    let success_health = success["route_explain_operator_health"]
        .as_str()
        .unwrap_or_default();
    let success_selected = success["route_explain_operator_selected"]
        .as_str()
        .unwrap_or_default();
    let success_pressure = success["route_explain_operator_pressure"]
        .as_str()
        .unwrap_or_default();
    let success_action = success["route_explain_operator_action"]
        .as_str()
        .unwrap_or_default();
    let success_reason = success["route_explain_operator_reason"]
        .as_str()
        .unwrap_or_default();

    assert_eq!(
        summary_field(success_summary, "health"),
        Some(success_health)
    );
    assert_eq!(
        summary_field(success_summary, "selected"),
        Some(success_selected)
    );
    assert_eq!(
        summary_field(success_summary, "pressure"),
        Some(success_pressure)
    );
    assert_eq!(
        summary_field(success_summary, "action"),
        Some(success_action)
    );
    assert_eq!(
        summary_field(success_summary, "reason"),
        Some(success_reason)
    );

    for case in error_cases {
        let json = build_route_explain_error_json(&options, case.stage, "stage failed");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).unwrap_or_else(|e| unreachable!("valid error json: {e}"));

        assert_route_explain_envelope(&parsed, expected_status_error(), expected_kind_error());
        assert_route_explain_contract_blocks_presence(&parsed);
        assert_operator_summary_invariants(&parsed);
        assert_error_contract_consistency(&parsed);
        assert_error_health_projection_consistency(&parsed);

        let summary = parsed["route_explain_operator_summary"]
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
        let action = parsed["route_explain_operator_action"]
            .as_str()
            .unwrap_or_default();
        let reason = parsed["route_explain_operator_reason"]
            .as_str()
            .unwrap_or_default();

        assert_eq!(summary_field(summary, "health"), Some(health));
        assert_eq!(summary_field(summary, "selected"), Some(selected));
        assert_eq!(summary_field(summary, "pressure"), Some(pressure));
        assert_eq!(summary_field(summary, "action"), Some(action));
        assert_eq!(summary_field(summary, "reason"), Some(reason));
        assert_eq!(
            parsed["route_explain_operator_route_key"],
            format!("{health}:{action}")
        );

        assert_eq!(action, case.action);
        assert_eq!(reason, case.stage);
        assert_eq!(parsed["route_explain_error_stage_category"], case.category);
        assert_eq!(parsed["route_explain_error_retriable"], case.retriable);
        assert_eq!(
            parsed["route_explain_error_retry_backoff_hint"],
            case.backoff
        );
        assert_eq!(
            parsed["route_explain_error_route_key"],
            format!("{}:{}", case.category, case.action)
        );

        assert!(summary.contains("health:"));
        assert!(summary.contains("selected:"));
        assert!(summary.contains("pressure:"));
        assert!(summary.contains("action:"));
        assert!(summary.contains("reason:"));
        assert!(success_summary.contains("health:"));
        assert!(success_summary.contains("selected:"));
        assert!(success_summary.contains("pressure:"));
        assert!(success_summary.contains("action:"));
        assert!(success_summary.contains("reason:"));
    }
}

#[test]
fn mesh_route_explain_json_operator_summary_consistent_for_cli_error_matrix() {
    struct CliCase<'a> {
        label: &'a str,
        args: Vec<String>,
        action: &'a str,
        stage: &'a str,
    }

    let mut peer_spec_args =
        base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
    peer_spec_args[7] = "bad-peer-spec".to_string();

    let mut simulation_args =
        base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95");
    simulation_args.extend(["--failed-node".to_string(), "n-missing".to_string()]);

    let cli_cases = [
        CliCase {
            label: "operator_summary_cli_policy_parse_error",
            args: base_route_explain_args("mesh_max_peers=0"),
            action: "fix_policy_payload",
            stage: "policy_parse",
        },
        CliCase {
            label: "operator_summary_cli_plan_path_error",
            args: base_route_explain_args("allow=mesh;mesh_max_peers=1;mesh_min_reliability=95"),
            action: "adjust_policy_or_peers",
            stage: "plan_path",
        },
        CliCase {
            label: "operator_summary_cli_peer_spec_error",
            args: peer_spec_args,
            action: "fix_peer_spec",
            stage: "peer_spec",
        },
        CliCase {
            label: "operator_summary_cli_simulation_error",
            args: simulation_args,
            action: "inspect_discovery_input",
            stage: "simulation_input",
        },
    ];

    for case in cli_cases {
        let parsed = run_route_explain_json(case.args, 2, case.label);

        assert_route_explain_envelope(&parsed, expected_status_error(), expected_kind_error());
        assert_route_explain_contract_blocks_presence(&parsed);
        assert_operator_summary_invariants(&parsed);
        assert_error_contract_consistency(&parsed);
        assert_error_health_projection_consistency(&parsed);

        let summary = parsed["route_explain_operator_summary"]
            .as_str()
            .unwrap_or_default();
        let signature = parsed["route_explain_operator_signature"]
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
        let action = parsed["route_explain_operator_action"]
            .as_str()
            .unwrap_or_default();
        let reason = parsed["route_explain_operator_reason"]
            .as_str()
            .unwrap_or_default();
        let route_key = parsed["route_explain_operator_route_key"]
            .as_str()
            .unwrap_or_default();
        let error_action = parsed["route_explain_error_action"]
            .as_str()
            .unwrap_or_default();
        let error_stage = parsed["error_stage"].as_str().unwrap_or_default();

        assert_eq!(summary, signature);
        assert_eq!(summary_field(summary, "health"), Some(health));
        assert_eq!(summary_field(summary, "selected"), Some(selected));
        assert_eq!(summary_field(summary, "pressure"), Some(pressure));
        assert_eq!(summary_field(summary, "action"), Some(action));
        assert_eq!(summary_field(summary, "reason"), Some(reason));
        assert_eq!(route_key, format!("{health}:{action}"));

        assert_eq!(action, case.action);
        assert_eq!(reason, case.stage);
        assert_eq!(error_action, case.action);
        assert_eq!(error_stage, case.stage);
    }
}
