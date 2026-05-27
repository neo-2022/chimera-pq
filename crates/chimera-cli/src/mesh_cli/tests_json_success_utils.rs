use super::route_explain_meta::ROUTE_EXPLAIN_STATUS_OK;
use super::tests_contract_constants::BOOL_FALSE;
use super::tests_json_utils::{
    assert_health_summary_shape, expected_health_summary_ok, expected_integrity_all_true,
    expected_status_ok, summary_field,
};

pub(crate) fn assert_success_health_projection_consistency(parsed: &serde_json::Value) {
    assert_eq!(
        parsed["status"].as_str().unwrap_or(""),
        ROUTE_EXPLAIN_STATUS_OK
    );
    assert_eq!(
        parsed["route_explain_health_gate"].as_str().unwrap_or(""),
        ROUTE_EXPLAIN_STATUS_OK
    );
    let summary = parsed["route_explain_health_summary"]
        .as_str()
        .unwrap_or("");
    assert_eq!(summary, expected_health_summary_ok());
    assert_health_summary_shape(summary);
    assert_eq!(summary_field(summary, "table"), Some(expected_status_ok()));
    assert_eq!(summary_field(summary, "degraded"), Some(BOOL_FALSE));
    assert_eq!(
        summary_field(summary, "pressure_projection"),
        Some(expected_status_ok())
    );
}

pub(crate) fn assert_success_operator_health_block(
    parsed: &serde_json::Value,
    expected_selected: &str,
    expected_pressure: &str,
    expected_action: &str,
    expected_reason: &str,
) {
    assert_eq!(
        parsed["route_explain_operator_health"],
        ROUTE_EXPLAIN_STATUS_OK
    );
    assert_eq!(parsed["route_explain_operator_selected"], expected_selected);
    assert_eq!(parsed["route_explain_operator_pressure"], expected_pressure);
    assert_eq!(parsed["route_explain_operator_action"], expected_action);
    assert_eq!(parsed["route_explain_operator_reason"], expected_reason);
    assert_eq!(
        parsed["route_explain_contract_integrity"],
        expected_integrity_all_true()
    );
    assert_success_health_projection_consistency(parsed);
}
