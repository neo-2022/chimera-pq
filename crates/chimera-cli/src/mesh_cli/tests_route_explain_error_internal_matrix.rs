use super::route_explain_error::{
    build_route_explain_error_json, route_explain_error_json_field_count,
};
use super::route_explain_meta::{
    ROUTE_EXPLAIN_CONTRACT_FAMILY, ROUTE_EXPLAIN_CONTRACT_VERSION, ROUTE_EXPLAIN_ERROR_HEALTH_GATE,
    ROUTE_EXPLAIN_ERROR_HEALTH_SUMMARY, ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH,
    ROUTE_EXPLAIN_KIND_ERROR, ROUTE_EXPLAIN_NETWORK_STATE_NOT_MODIFIED, ROUTE_EXPLAIN_STATUS_ERROR,
};
use super::tests_contract_constants::{
    ACTION_INSPECT_DISCOVERY_INPUT, ACTION_INSPECT_ERROR, ACTION_INSPECT_FAILOVER_INPUT,
    ACTION_INSPECT_HEALTH_INPUTS, ACTION_INSPECT_RUNTIME_BOOTSTRAP, BACKOFF_AFTER_FIX,
    BACKOFF_IMMEDIATE, BACKOFF_SHORT, CATEGORY_HEALTH, CATEGORY_PLANNING, CATEGORY_RUNTIME,
    CATEGORY_UNKNOWN, ERROR_PRESSURE_UNKNOWN, ERROR_SELECTED_NONE,
    RESOLUTION_INSPECT_DISCOVERY_RECORDS, RESOLUTION_INSPECT_ERROR_DETAILS,
    RESOLUTION_INSPECT_FAILOVER_EVENT, RESOLUTION_INSPECT_HEALTH_STATE,
    RESOLUTION_INSPECT_RUNTIME_NAMESPACE, RETRIABLE_FALSE, RETRIABLE_TRUE, STAGE_DISCOVERY_MERGE,
    STAGE_FAILOVER_PLAN, STAGE_HEALTH_STATE_UPDATE, STAGE_RESELECTION_PLAN,
    STAGE_RUNTIME_BOOTSTRAP,
};
use super::tests_json_error_utils::assert_error_contract_consistency;
use super::tests_json_utils::expected_integrity_all_true;
use crate::mesh_cli::options::MeshRouteExplainOptions;
const SELECTED_NONE: &str = ERROR_SELECTED_NONE;
const PRESSURE_UNKNOWN: &str = ERROR_PRESSURE_UNKNOWN;

#[test]
fn route_explain_error_json_stage_matrix_contract_is_consistent() {
    struct Case<'a> {
        stage: &'a str,
        expected_action: &'a str,
        expected_category: &'a str,
        expected_retriable: &'a str,
        expected_backoff: &'a str,
        expected_resolution: &'a str,
    }

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
    let cases = [
        Case {
            stage: STAGE_RUNTIME_BOOTSTRAP,
            expected_action: ACTION_INSPECT_RUNTIME_BOOTSTRAP,
            expected_category: CATEGORY_RUNTIME,
            expected_retriable: RETRIABLE_TRUE,
            expected_backoff: BACKOFF_IMMEDIATE,
            expected_resolution: RESOLUTION_INSPECT_RUNTIME_NAMESPACE,
        },
        Case {
            stage: STAGE_DISCOVERY_MERGE,
            expected_action: ACTION_INSPECT_DISCOVERY_INPUT,
            expected_category: CATEGORY_PLANNING,
            expected_retriable: RETRIABLE_TRUE,
            expected_backoff: BACKOFF_SHORT,
            expected_resolution: RESOLUTION_INSPECT_DISCOVERY_RECORDS,
        },
        Case {
            stage: STAGE_FAILOVER_PLAN,
            expected_action: ACTION_INSPECT_FAILOVER_INPUT,
            expected_category: CATEGORY_PLANNING,
            expected_retriable: RETRIABLE_TRUE,
            expected_backoff: BACKOFF_SHORT,
            expected_resolution: RESOLUTION_INSPECT_FAILOVER_EVENT,
        },
        Case {
            stage: STAGE_HEALTH_STATE_UPDATE,
            expected_action: ACTION_INSPECT_HEALTH_INPUTS,
            expected_category: CATEGORY_HEALTH,
            expected_retriable: RETRIABLE_TRUE,
            expected_backoff: BACKOFF_SHORT,
            expected_resolution: RESOLUTION_INSPECT_HEALTH_STATE,
        },
        Case {
            stage: STAGE_RESELECTION_PLAN,
            expected_action: ACTION_INSPECT_HEALTH_INPUTS,
            expected_category: CATEGORY_HEALTH,
            expected_retriable: RETRIABLE_TRUE,
            expected_backoff: BACKOFF_SHORT,
            expected_resolution: RESOLUTION_INSPECT_HEALTH_STATE,
        },
        Case {
            stage: "unknown_stage",
            expected_action: ACTION_INSPECT_ERROR,
            expected_category: CATEGORY_UNKNOWN,
            expected_retriable: RETRIABLE_FALSE,
            expected_backoff: BACKOFF_AFTER_FIX,
            expected_resolution: RESOLUTION_INSPECT_ERROR_DETAILS,
        },
    ];

    for case in cases {
        let json = build_route_explain_error_json(&options, case.stage, "stage failed");
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .unwrap_or_else(|e| unreachable!("error json should be valid: {e}"));

        assert_eq!(parsed["status"], ROUTE_EXPLAIN_STATUS_ERROR);
        assert_eq!(parsed["kind"], ROUTE_EXPLAIN_KIND_ERROR);
        assert_eq!(
            parsed["route_explain_contract_family"],
            ROUTE_EXPLAIN_CONTRACT_FAMILY
        );
        assert_eq!(
            parsed["explain_contract_version"],
            ROUTE_EXPLAIN_CONTRACT_VERSION
        );
        assert_eq!(parsed["error_stage"], case.stage);
        assert_eq!(parsed["route_explain_error_action"], case.expected_action);
        assert_eq!(
            parsed["route_explain_error_stage_category"],
            case.expected_category
        );
        assert_eq!(
            parsed["route_explain_error_retriable"],
            case.expected_retriable
        );
        assert_eq!(
            parsed["route_explain_error_retry_backoff_hint"],
            case.expected_backoff
        );
        assert_eq!(
            parsed["route_explain_error_resolution_hint"],
            case.expected_resolution
        );
        assert_eq!(
            parsed["route_explain_error_signature"],
            format!(
                "stage:{};category:{};retriable:{};action:{}",
                case.stage, case.expected_category, case.expected_retriable, case.expected_action
            )
        );
        assert_eq!(
            parsed["route_explain_error_route_key"],
            format!("{}:{}", case.expected_category, case.expected_action)
        );
        assert_error_contract_consistency(&parsed);
        assert_eq!(
            parsed["route_explain_operator_health"],
            ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH
        );
        assert_eq!(parsed["route_explain_operator_selected"], SELECTED_NONE);
        assert_eq!(parsed["route_explain_operator_pressure"], PRESSURE_UNKNOWN);
        assert_eq!(
            parsed["route_explain_operator_action"],
            case.expected_action
        );
        assert_eq!(parsed["route_explain_operator_reason"], case.stage);
        assert_eq!(
            parsed["route_explain_operator_route_key"],
            format!("error:{}", case.expected_action)
        );
        assert_eq!(
            parsed["route_explain_operator_summary"],
            format!(
                "health:{};selected:{};pressure:{};action:{};reason:{}",
                ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH,
                SELECTED_NONE,
                PRESSURE_UNKNOWN,
                case.expected_action,
                case.stage
            )
        );
        assert_eq!(
            parsed["route_explain_health_gate"], ROUTE_EXPLAIN_ERROR_HEALTH_GATE,
            "health gate should stay stable for all error stages"
        );
        assert_eq!(
            parsed["route_explain_health_summary"],
            ROUTE_EXPLAIN_ERROR_HEALTH_SUMMARY
        );
        assert_eq!(
            parsed["route_explain_contract_integrity"],
            expected_integrity_all_true()
        );
        assert_eq!(
            parsed["network_state"],
            ROUTE_EXPLAIN_NETWORK_STATE_NOT_MODIFIED
        );
        assert_eq!(
            parsed.as_object().map(|obj| obj.len()).unwrap_or_default(),
            route_explain_error_json_field_count()
        );
    }
}
