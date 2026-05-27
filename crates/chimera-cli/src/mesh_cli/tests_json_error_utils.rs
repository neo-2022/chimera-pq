use super::route_explain_error::{error_retriable, retry_backoff_hint};
use super::route_explain_meta::{
    ROUTE_EXPLAIN_ERROR_HEALTH_GATE, ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH,
    ROUTE_EXPLAIN_ERROR_OPERATOR_PRESSURE, ROUTE_EXPLAIN_ERROR_OPERATOR_SELECTED,
    ROUTE_EXPLAIN_KIND_ERROR, ROUTE_EXPLAIN_RECOVERY_FIELDS_CHECKSUM,
    ROUTE_EXPLAIN_RECOVERY_SCHEMA_VERSION, ROUTE_EXPLAIN_STATUS_ERROR,
};
use super::tests_contract_constants::{
    BACKOFF_SHORT, CATEGORY_PLANNING, ERROR_ACTION_ADJUST_POLICY, ERROR_STAGE_PLAN_PATH,
    RESOLUTION_RELAX_POLICY, RETRIABLE_FALSE, RETRIABLE_TRUE, UNKNOWN_VALUE,
};
use super::tests_json_utils::{expected_health_summary_error, expected_status_error};

pub(crate) fn assert_error_health_projection_consistency(parsed: &serde_json::Value) {
    assert_eq!(
        parsed["status"].as_str().unwrap_or(""),
        ROUTE_EXPLAIN_STATUS_ERROR
    );
    assert_eq!(
        parsed["kind"].as_str().unwrap_or(""),
        ROUTE_EXPLAIN_KIND_ERROR
    );
    assert_eq!(
        parsed["route_explain_health_gate"].as_str().unwrap_or(""),
        ROUTE_EXPLAIN_ERROR_HEALTH_GATE
    );
    let summary = parsed["route_explain_health_summary"]
        .as_str()
        .unwrap_or("");
    assert_eq!(summary, expected_health_summary_error());
    assert_eq!(
        super::tests_json_utils::summary_field(summary, "table"),
        Some(expected_status_error())
    );
    assert_eq!(
        super::tests_json_utils::summary_field(summary, "degraded"),
        Some(UNKNOWN_VALUE)
    );
    assert_eq!(
        super::tests_json_utils::summary_field(summary, "pressure_projection"),
        Some(UNKNOWN_VALUE)
    );
}

pub(crate) fn assert_error_category_retriable_invariants(parsed: &serde_json::Value) {
    if parsed["status"] != ROUTE_EXPLAIN_STATUS_ERROR {
        return;
    }
    let category = parsed["route_explain_error_stage_category"]
        .as_str()
        .unwrap_or("");
    let retriable = parsed["route_explain_error_retriable"]
        .as_str()
        .unwrap_or("");
    let expected = if error_retriable(category) {
        RETRIABLE_TRUE
    } else {
        RETRIABLE_FALSE
    };
    assert_eq!(retriable, expected);
}

pub(crate) fn assert_error_signature_invariants(parsed: &serde_json::Value) {
    if parsed["status"] != ROUTE_EXPLAIN_STATUS_ERROR {
        return;
    }
    let stage = parsed["error_stage"].as_str().unwrap_or("");
    let category = parsed["route_explain_error_stage_category"]
        .as_str()
        .unwrap_or("");
    let retriable = parsed["route_explain_error_retriable"]
        .as_str()
        .unwrap_or("");
    let action = parsed["route_explain_error_action"].as_str().unwrap_or("");
    let signature = parsed["route_explain_error_signature"]
        .as_str()
        .unwrap_or("");
    let route_key = parsed["route_explain_error_route_key"]
        .as_str()
        .unwrap_or("");
    assert_eq!(
        signature,
        format!("stage:{stage};category:{category};retriable:{retriable};action:{action}")
    );
    assert_eq!(route_key, format!("{category}:{action}"));
}

pub(crate) fn assert_error_operator_route_alignment(parsed: &serde_json::Value) {
    if parsed["status"] != ROUTE_EXPLAIN_STATUS_ERROR {
        return;
    }
    let stage_category = parsed["route_explain_error_stage_category"]
        .as_str()
        .unwrap_or("");
    let action = parsed["route_explain_error_action"].as_str().unwrap_or("");
    let error_route_key = parsed["route_explain_error_route_key"]
        .as_str()
        .unwrap_or("");
    let operator_action = parsed["route_explain_operator_action"]
        .as_str()
        .unwrap_or("");
    let operator_route_key = parsed["route_explain_operator_route_key"]
        .as_str()
        .unwrap_or("");

    assert_eq!(error_route_key, format!("{stage_category}:{action}"));
    assert_eq!(operator_action, action);
    assert_eq!(operator_route_key, format!("error:{action}"));
}

pub(crate) fn assert_error_retry_backoff_invariants(parsed: &serde_json::Value) {
    if parsed["status"] != ROUTE_EXPLAIN_STATUS_ERROR {
        return;
    }
    let category = parsed["route_explain_error_stage_category"]
        .as_str()
        .unwrap_or("");
    let retriable = parsed["route_explain_error_retriable"]
        .as_str()
        .unwrap_or("");
    let backoff = parsed["route_explain_error_retry_backoff_hint"]
        .as_str()
        .unwrap_or("");
    let expected = retry_backoff_hint(category, retriable);
    assert_eq!(backoff, expected);
}

pub(crate) fn assert_error_recovery_projection_consistency(parsed: &serde_json::Value) {
    if parsed["status"] != ROUTE_EXPLAIN_STATUS_ERROR {
        return;
    }
    let action = parsed["route_explain_error_action"]
        .as_str()
        .unwrap_or_default();
    let needed = parsed["connect_recovery_needed"]
        .as_str()
        .unwrap_or_default();
    let strategy = parsed["connect_recovery_strategy"]
        .as_str()
        .unwrap_or_default();
    let projection_consistency = parsed["connect_recovery_projection_consistency"]
        .as_str()
        .unwrap_or_default();
    let projection_key = parsed["connect_recovery_projection_key"]
        .as_str()
        .unwrap_or_default();
    let schema = parsed["route_explain_recovery_schema_version"]
        .as_str()
        .unwrap_or_default();
    let checksum = parsed["route_explain_recovery_fields_checksum"]
        .as_str()
        .unwrap_or_default();
    let attempts = parsed["auto_recovery_attempts"]
        .as_str()
        .unwrap_or_default();
    let final_result = parsed["auto_recovery_final_result"]
        .as_str()
        .unwrap_or_default();
    let budget_exhausted = parsed["connect_retry_budget_exhausted"]
        .as_str()
        .unwrap_or_default();

    assert_eq!(attempts, "0");
    assert_eq!(final_result, "not_applicable_error");
    assert_eq!(budget_exhausted, "unknown");
    assert_eq!(needed, "false");
    assert_eq!(strategy, "none");
    assert_eq!(projection_consistency, "true");
    assert_eq!(
        projection_key,
        format!("needed:false;strategy:none;action:{action}")
    );
    assert_eq!(schema, ROUTE_EXPLAIN_RECOVERY_SCHEMA_VERSION);
    assert_eq!(checksum, ROUTE_EXPLAIN_RECOVERY_FIELDS_CHECKSUM);
}

pub(crate) fn assert_error_contract_consistency(parsed: &serde_json::Value) {
    assert_error_category_retriable_invariants(parsed);
    assert_error_signature_invariants(parsed);
    assert_error_retry_backoff_invariants(parsed);
    assert_error_operator_route_alignment(parsed);
    assert_error_recovery_projection_consistency(parsed);
}

pub(crate) fn assert_error_plan_path_contract(parsed: &serde_json::Value) {
    assert_eq!(parsed["error_stage"], ERROR_STAGE_PLAN_PATH);
    assert_eq!(
        parsed["route_explain_error_action"]
            .as_str()
            .unwrap_or_default(),
        ERROR_ACTION_ADJUST_POLICY
    );
    assert_eq!(
        parsed["route_explain_error_stage_category"]
            .as_str()
            .unwrap_or_default(),
        CATEGORY_PLANNING
    );
    assert_eq!(
        parsed["route_explain_error_retriable"]
            .as_str()
            .unwrap_or_default(),
        RETRIABLE_TRUE
    );
    assert_eq!(
        parsed["route_explain_error_retry_backoff_hint"]
            .as_str()
            .unwrap_or_default(),
        BACKOFF_SHORT
    );
    assert_eq!(
        parsed["route_explain_error_resolution_hint"]
            .as_str()
            .unwrap_or_default(),
        RESOLUTION_RELAX_POLICY
    );
    assert_eq!(
        parsed["route_explain_error_signature"]
            .as_str()
            .unwrap_or_default(),
        format!(
            "stage:{};category:{};retriable:{};action:{}",
            ERROR_STAGE_PLAN_PATH, CATEGORY_PLANNING, RETRIABLE_TRUE, ERROR_ACTION_ADJUST_POLICY
        )
    );
    assert_eq!(
        parsed["route_explain_error_route_key"]
            .as_str()
            .unwrap_or_default(),
        format!("{}:{}", CATEGORY_PLANNING, ERROR_ACTION_ADJUST_POLICY)
    );
}

pub(crate) fn assert_error_operator_health_block(
    parsed: &serde_json::Value,
    expected_action: &str,
    expected_stage: &str,
) {
    assert_eq!(
        parsed["route_explain_operator_health"],
        ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH
    );
    assert_eq!(
        parsed["route_explain_operator_selected"],
        ROUTE_EXPLAIN_ERROR_OPERATOR_SELECTED
    );
    assert_eq!(
        parsed["route_explain_operator_pressure"],
        ROUTE_EXPLAIN_ERROR_OPERATOR_PRESSURE
    );
    assert_eq!(parsed["route_explain_operator_action"], expected_action);
    assert_eq!(parsed["route_explain_operator_reason"], expected_stage);
    assert_eq!(
        parsed["route_explain_health_gate"],
        ROUTE_EXPLAIN_ERROR_HEALTH_GATE
    );
    assert_eq!(
        parsed["route_explain_health_summary"],
        expected_health_summary_error()
    );
}
