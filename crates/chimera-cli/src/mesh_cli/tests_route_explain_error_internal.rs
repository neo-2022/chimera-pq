use super::route_explain_error::{
    build_route_explain_error_json, error_action_hint, error_retriable, error_stage_category,
    resolution_hint, retry_backoff_hint,
};
use super::route_explain_meta::{
    ROUTE_EXPLAIN_CONTRACT_FAMILY, ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH, ROUTE_EXPLAIN_KIND_ERROR,
    ROUTE_EXPLAIN_NETWORK_STATE_NOT_MODIFIED, ROUTE_EXPLAIN_STATUS_ERROR,
};
use super::tests_contract_constants::{
    ACTION_FIX_PEER_SPEC, ACTION_FIX_POLICY_PAYLOAD, ACTION_FIX_TABLE_POLICY, ACTION_INSPECT_ERROR,
    ACTION_INSPECT_RUNTIME_BOOTSTRAP, BACKOFF_AFTER_FIX, BACKOFF_IMMEDIATE, BACKOFF_SHORT,
    CATEGORY_HEALTH, CATEGORY_INPUT, CATEGORY_PLANNING, CATEGORY_POLICY, CATEGORY_RUNTIME,
    CATEGORY_UNKNOWN, ERROR_ACTION_ADJUST_POLICY, ERROR_PRESSURE_UNKNOWN, ERROR_SELECTED_NONE,
    ERROR_STAGE_PLAN_PATH, RESOLUTION_CHECK_PEER_SPEC, RESOLUTION_INSPECT_ERROR_DETAILS,
    RESOLUTION_INSPECT_RUNTIME_NAMESPACE, RESOLUTION_RELAX_POLICY, RESOLUTION_TABLE_POLICY_BOUNDS,
    RETRIABLE_FALSE, RETRIABLE_TRUE, STAGE_PEER_SPEC, STAGE_PEER_TABLE_POLICY, STAGE_POLICY_PARSE,
    STAGE_RESELECTION_PLAN, STAGE_RUNTIME_BOOTSTRAP,
};
use super::tests_json_error_utils::assert_error_contract_consistency;
use crate::mesh_cli::options::MeshRouteExplainOptions;

#[test]
fn route_explain_error_json_escapes_user_fields() {
    let options = MeshRouteExplainOptions {
        namespace: "cef\"public".to_string(),
        node_name: "node\"client".to_string(),
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

    let json = build_route_explain_error_json(&options, ERROR_STAGE_PLAN_PATH, "zero \"eligible\"");
    let parsed: serde_json::Value = serde_json::from_str(&json)
        .unwrap_or_else(|e| unreachable!("error json should be valid: {e}"));

    assert_eq!(parsed["status"], ROUTE_EXPLAIN_STATUS_ERROR);
    assert_eq!(parsed["kind"], ROUTE_EXPLAIN_KIND_ERROR);
    assert_eq!(
        parsed["route_explain_contract_family"],
        ROUTE_EXPLAIN_CONTRACT_FAMILY
    );
    assert_eq!(parsed["namespace"], "cef\"public");
    assert_eq!(parsed["node"], "node\"client");
    assert_eq!(parsed["error_stage"], ERROR_STAGE_PLAN_PATH);
    assert_eq!(parsed["error"], "zero \"eligible\"");
    assert_eq!(
        parsed["route_explain_error_action"],
        ERROR_ACTION_ADJUST_POLICY
    );
    assert_eq!(
        parsed["route_explain_operator_action"],
        ERROR_ACTION_ADJUST_POLICY
    );
    assert_eq!(
        parsed["route_explain_operator_health"],
        ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH
    );
    assert_eq!(
        parsed["route_explain_operator_selected"],
        ERROR_SELECTED_NONE
    );
    assert_eq!(
        parsed["route_explain_operator_pressure"],
        ERROR_PRESSURE_UNKNOWN
    );
    assert_eq!(
        parsed["route_explain_operator_reason"],
        ERROR_STAGE_PLAN_PATH
    );
    assert_eq!(
        parsed["route_explain_error_stage_category"],
        CATEGORY_PLANNING
    );
    assert_eq!(parsed["route_explain_error_retriable"], RETRIABLE_TRUE);
    assert_eq!(
        parsed["route_explain_error_retry_backoff_hint"],
        BACKOFF_SHORT
    );
    assert_eq!(
        parsed["route_explain_error_resolution_hint"],
        RESOLUTION_RELAX_POLICY
    );
    assert_eq!(
        parsed["route_explain_error_signature"],
        format!(
            "stage:{};category:{};retriable:{};action:{}",
            ERROR_STAGE_PLAN_PATH, CATEGORY_PLANNING, RETRIABLE_TRUE, ERROR_ACTION_ADJUST_POLICY
        )
    );
    assert_eq!(
        parsed["route_explain_error_route_key"],
        format!("{}:{}", CATEGORY_PLANNING, ERROR_ACTION_ADJUST_POLICY)
    );
    assert_error_contract_consistency(&parsed);
    assert_eq!(
        parsed["route_explain_operator_route_key"],
        format!("error:{}", ERROR_ACTION_ADJUST_POLICY)
    );
    assert_eq!(
        parsed["network_state"],
        ROUTE_EXPLAIN_NETWORK_STATE_NOT_MODIFIED
    );
}

#[test]
fn route_explain_error_actions_are_stage_specific() {
    assert_eq!(error_action_hint(STAGE_PEER_SPEC), ACTION_FIX_PEER_SPEC);
    assert_eq!(
        error_action_hint(STAGE_PEER_TABLE_POLICY),
        ACTION_FIX_TABLE_POLICY
    );
    assert_eq!(
        error_action_hint(STAGE_POLICY_PARSE),
        ACTION_FIX_POLICY_PAYLOAD
    );
    assert_eq!(
        error_action_hint(ERROR_STAGE_PLAN_PATH),
        ERROR_ACTION_ADJUST_POLICY
    );
    assert_eq!(error_action_hint(CATEGORY_UNKNOWN), ACTION_INSPECT_ERROR);
}

#[test]
fn route_explain_error_categories_are_stage_specific() {
    assert_eq!(error_stage_category(STAGE_PEER_SPEC), CATEGORY_INPUT);
    assert_eq!(error_stage_category(STAGE_POLICY_PARSE), CATEGORY_POLICY);
    assert_eq!(
        error_stage_category(STAGE_RUNTIME_BOOTSTRAP),
        CATEGORY_RUNTIME
    );
    assert_eq!(
        error_stage_category(ERROR_STAGE_PLAN_PATH),
        CATEGORY_PLANNING
    );
    assert_eq!(
        error_stage_category(STAGE_RESELECTION_PLAN),
        CATEGORY_HEALTH
    );
    assert_eq!(error_stage_category(CATEGORY_UNKNOWN), CATEGORY_UNKNOWN);
}

#[test]
fn route_explain_error_retriable_is_category_specific() {
    assert!(!error_retriable(CATEGORY_INPUT));
    assert!(!error_retriable(CATEGORY_POLICY));
    assert!(error_retriable(CATEGORY_RUNTIME));
    assert!(error_retriable(CATEGORY_PLANNING));
    assert!(error_retriable(CATEGORY_HEALTH));
    assert!(!error_retriable(CATEGORY_UNKNOWN));
}

#[test]
fn route_explain_error_retry_backoff_hint_is_consistent() {
    assert_eq!(
        retry_backoff_hint(CATEGORY_RUNTIME, RETRIABLE_TRUE),
        BACKOFF_IMMEDIATE
    );
    assert_eq!(
        retry_backoff_hint(CATEGORY_PLANNING, RETRIABLE_TRUE),
        BACKOFF_SHORT
    );
    assert_eq!(
        retry_backoff_hint(CATEGORY_HEALTH, RETRIABLE_TRUE),
        BACKOFF_SHORT
    );
    assert_eq!(
        retry_backoff_hint(CATEGORY_POLICY, RETRIABLE_FALSE),
        BACKOFF_AFTER_FIX
    );
    assert_eq!(
        retry_backoff_hint(CATEGORY_INPUT, RETRIABLE_FALSE),
        BACKOFF_AFTER_FIX
    );
}

#[test]
fn route_explain_error_resolution_hint_is_action_specific() {
    assert_eq!(
        resolution_hint(ACTION_FIX_PEER_SPEC),
        RESOLUTION_CHECK_PEER_SPEC
    );
    assert_eq!(
        resolution_hint(ACTION_FIX_TABLE_POLICY),
        RESOLUTION_TABLE_POLICY_BOUNDS
    );
    assert_eq!(
        resolution_hint(ERROR_ACTION_ADJUST_POLICY),
        RESOLUTION_RELAX_POLICY
    );
    assert_eq!(
        resolution_hint(ACTION_INSPECT_RUNTIME_BOOTSTRAP),
        RESOLUTION_INSPECT_RUNTIME_NAMESPACE
    );
    assert_eq!(
        resolution_hint(ACTION_INSPECT_ERROR),
        RESOLUTION_INSPECT_ERROR_DETAILS
    );
}
