pub(crate) fn build_route_explain_contract_integrity(
    operator_summary: &str,
    operator_signature: &str,
    operator_route_key: &str,
    health_gate: &str,
    operator_health: &str,
    operator_action: &str,
) -> String {
    format!(
        "signature_match:{};route_key_match:{};health_gate_match:{}",
        bool_text(operator_summary == operator_signature),
        bool_text(operator_route_key == format!("{operator_health}:{operator_action}")),
        bool_text(health_gate == operator_health)
    )
}

fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

#[cfg(test)]
mod tests {
    use super::build_route_explain_contract_integrity;
    use crate::mesh_cli::route_explain_meta::{
        ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH, ROUTE_EXPLAIN_ERROR_OPERATOR_PRESSURE,
        ROUTE_EXPLAIN_ERROR_OPERATOR_SELECTED,
    };
    use crate::mesh_cli::tests_contract_constants::{
        INTEGRITY_ALL_TRUE, INTEGRITY_ROUTE_KEY_FALSE,
    };

    #[test]
    fn route_explain_contract_integrity_marks_fully_consistent_state() {
        let summary =
            "health:ok;selected:n1;pressure:saturated;action:use_selected_path;reason:none";
        let value = build_route_explain_contract_integrity(
            summary,
            summary,
            "ok:use_selected_path",
            "ok",
            "ok",
            "use_selected_path",
        );
        assert_eq!(value, INTEGRITY_ALL_TRUE);
    }

    #[test]
    fn route_explain_contract_integrity_marks_inconsistent_state() {
        let summary = format!(
            "health:{};selected:{};pressure:{};action:inspect_error;reason:plan_path",
            ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH,
            ROUTE_EXPLAIN_ERROR_OPERATOR_SELECTED,
            ROUTE_EXPLAIN_ERROR_OPERATOR_PRESSURE
        );
        let value = build_route_explain_contract_integrity(
            &summary,
            &summary,
            "error:adjust_policy_or_peers",
            ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH,
            ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH,
            "inspect_error",
        );
        assert_eq!(value, INTEGRITY_ROUTE_KEY_FALSE);
    }
}
