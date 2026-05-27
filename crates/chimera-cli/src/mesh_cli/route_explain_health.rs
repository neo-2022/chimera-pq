use super::route_explain_meta::ROUTE_EXPLAIN_STATUS_OK;

pub(crate) struct RouteExplainHealth {
    pub(crate) gate: &'static str,
    pub(crate) summary: String,
}

pub(crate) struct RouteExplainOperatorSummary {
    pub(crate) line: String,
    pub(crate) signature: String,
    pub(crate) route_key: String,
    pub(crate) health: String,
    pub(crate) selected: String,
    pub(crate) pressure: String,
    pub(crate) action: String,
    pub(crate) reason: String,
}

const HEALTH_WARN_GATE: &str = "warn:route_explain_health";
const OPERATOR_SELECTED_NONE: &str = "none";
const OPERATOR_REASON_NONE: &str = "none";
const OPERATOR_ACTION_USE_SELECTED_PATH: &str = "use_selected_path";
const OPERATOR_ACTION_INSPECT_ROUTE_EXPLAIN_HEALTH: &str = "inspect_route_explain_health";
const OPERATOR_ACTION_RETRY_CONNECT_ENDPOINTS: &str = "retry_connect_endpoints";
const OPERATOR_REASON_CONNECT_PLAN_EXHAUSTED_OR_UNREACHABLE: &str =
    "connect_plan_exhausted_or_unreachable";
const DEGRADED_FALSE: &str = "false";
const PRESSURE_ACTION_WAIT_OR_RECOVER_HEALTH: &str = "wait_or_recover_health";

pub(crate) fn build_route_explain_health(
    table_runtime_consistency_gate: &str,
    preemptive_degraded_path: &str,
    selection_pressure_projection_gate: &str,
) -> RouteExplainHealth {
    let table_ok = table_runtime_consistency_gate == ROUTE_EXPLAIN_STATUS_OK;
    let degraded_ok = preemptive_degraded_path == DEGRADED_FALSE;
    let pressure_projection_ok = selection_pressure_projection_gate == ROUTE_EXPLAIN_STATUS_OK;
    let gate = if table_ok && degraded_ok && pressure_projection_ok {
        ROUTE_EXPLAIN_STATUS_OK
    } else {
        HEALTH_WARN_GATE
    };
    let summary = format!(
        "table:{};degraded:{};pressure_projection:{}",
        table_runtime_consistency_gate,
        preemptive_degraded_path,
        selection_pressure_projection_gate
    );

    RouteExplainHealth { gate, summary }
}

pub(crate) fn build_route_explain_operator_summary(
    health_gate: &str,
    selected_peer: &str,
    selection_pressure_level: &str,
    selection_pressure_action_hint: &str,
    degraded_reason: &str,
    connect_retry_budget_exhausted: bool,
) -> RouteExplainOperatorSummary {
    let selected = if selected_peer.is_empty() {
        OPERATOR_SELECTED_NONE
    } else {
        selected_peer
    };
    let connect_plan_exhausted = health_gate != ROUTE_EXPLAIN_STATUS_OK
        && selected == OPERATOR_SELECTED_NONE
        && selection_pressure_action_hint == PRESSURE_ACTION_WAIT_OR_RECOVER_HEALTH;
    let connect_retry_exhausted = health_gate != ROUTE_EXPLAIN_STATUS_OK
        && selected == OPERATOR_SELECTED_NONE
        && connect_retry_budget_exhausted;
    let connect_recovery_needed = connect_plan_exhausted || connect_retry_exhausted;
    let action = if health_gate == ROUTE_EXPLAIN_STATUS_OK {
        OPERATOR_ACTION_USE_SELECTED_PATH
    } else if connect_recovery_needed {
        OPERATOR_ACTION_RETRY_CONNECT_ENDPOINTS
    } else if selection_pressure_action_hint == OPERATOR_REASON_NONE {
        OPERATOR_ACTION_INSPECT_ROUTE_EXPLAIN_HEALTH
    } else {
        selection_pressure_action_hint
    };
    let reason = if health_gate == ROUTE_EXPLAIN_STATUS_OK {
        OPERATOR_REASON_NONE
    } else if connect_recovery_needed {
        OPERATOR_REASON_CONNECT_PLAN_EXHAUSTED_OR_UNREACHABLE
    } else {
        degraded_reason
    };
    let line = format!(
        "health:{};selected:{};pressure:{};action:{};reason:{}",
        health_gate, selected, selection_pressure_level, action, reason
    );
    let signature = line.clone();
    let route_key = format!("{health_gate}:{action}");

    RouteExplainOperatorSummary {
        line,
        signature,
        route_key,
        health: health_gate.to_string(),
        selected: selected.to_string(),
        pressure: selection_pressure_level.to_string(),
        action: action.to_string(),
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEGRADED_FALSE, HEALTH_WARN_GATE, OPERATOR_ACTION_INSPECT_ROUTE_EXPLAIN_HEALTH,
        OPERATOR_ACTION_RETRY_CONNECT_ENDPOINTS, OPERATOR_ACTION_USE_SELECTED_PATH,
        OPERATOR_REASON_CONNECT_PLAN_EXHAUSTED_OR_UNREACHABLE, OPERATOR_REASON_NONE,
        OPERATOR_SELECTED_NONE, PRESSURE_ACTION_WAIT_OR_RECOVER_HEALTH, build_route_explain_health,
        build_route_explain_operator_summary,
    };
    use crate::mesh_cli::route_explain_meta::ROUTE_EXPLAIN_STATUS_OK;

    #[test]
    fn route_explain_health_warns_when_any_gate_is_not_ok() {
        let health = build_route_explain_health(
            "warn:runtime_consistency_mismatch",
            "true",
            "warn:pressure_projection_mismatch",
        );

        assert_eq!(health.gate, HEALTH_WARN_GATE);
        assert_eq!(
            health.summary,
            "table:warn:runtime_consistency_mismatch;degraded:true;pressure_projection:warn:pressure_projection_mismatch"
        );
    }

    #[test]
    fn route_explain_health_is_ok_when_all_gates_are_ok() {
        let health = build_route_explain_health(
            ROUTE_EXPLAIN_STATUS_OK,
            DEGRADED_FALSE,
            ROUTE_EXPLAIN_STATUS_OK,
        );

        assert_eq!(health.gate, ROUTE_EXPLAIN_STATUS_OK);
        assert_eq!(
            health.summary,
            format!(
                "table:{};degraded:{};pressure_projection:{}",
                ROUTE_EXPLAIN_STATUS_OK, DEGRADED_FALSE, ROUTE_EXPLAIN_STATUS_OK
            )
        );
    }

    #[test]
    fn operator_summary_prefers_selected_path_when_health_is_ok() {
        let summary = build_route_explain_operator_summary(
            ROUTE_EXPLAIN_STATUS_OK,
            "node-a",
            "healthy",
            OPERATOR_REASON_NONE,
            OPERATOR_REASON_NONE,
            false,
        );

        assert_eq!(
            summary.line,
            format!(
                "health:{};selected:node-a;pressure:healthy;action:{};reason:{}",
                ROUTE_EXPLAIN_STATUS_OK, OPERATOR_ACTION_USE_SELECTED_PATH, OPERATOR_REASON_NONE
            )
        );
    }

    #[test]
    fn operator_summary_points_to_health_when_warn_has_no_specific_action() {
        let summary = build_route_explain_operator_summary(
            HEALTH_WARN_GATE,
            "",
            "unknown",
            OPERATOR_REASON_NONE,
            "warn:runtime_consistency_mismatch",
            false,
        );

        assert_eq!(
            summary.line,
            format!(
                "health:{};selected:{};pressure:unknown;action:{};reason:warn:runtime_consistency_mismatch",
                HEALTH_WARN_GATE,
                OPERATOR_SELECTED_NONE,
                OPERATOR_ACTION_INSPECT_ROUTE_EXPLAIN_HEALTH
            )
        );
    }

    #[test]
    fn operator_summary_retries_connect_when_warn_and_no_selected_peer() {
        let summary = build_route_explain_operator_summary(
            HEALTH_WARN_GATE,
            "",
            "empty",
            PRESSURE_ACTION_WAIT_OR_RECOVER_HEALTH,
            "warn:no_candidate",
            false,
        );

        assert_eq!(
            summary.line,
            format!(
                "health:{};selected:{};pressure:empty;action:{};reason:{}",
                HEALTH_WARN_GATE,
                OPERATOR_SELECTED_NONE,
                OPERATOR_ACTION_RETRY_CONNECT_ENDPOINTS,
                OPERATOR_REASON_CONNECT_PLAN_EXHAUSTED_OR_UNREACHABLE
            )
        );
    }

    #[test]
    fn operator_summary_retries_connect_when_retry_budget_exhausted() {
        let summary = build_route_explain_operator_summary(
            HEALTH_WARN_GATE,
            "",
            "constrained",
            OPERATOR_REASON_NONE,
            "warn:degraded",
            true,
        );
        assert_eq!(
            summary.line,
            format!(
                "health:{};selected:{};pressure:constrained;action:{};reason:{}",
                HEALTH_WARN_GATE,
                OPERATOR_SELECTED_NONE,
                OPERATOR_ACTION_RETRY_CONNECT_ENDPOINTS,
                OPERATOR_REASON_CONNECT_PLAN_EXHAUSTED_OR_UNREACHABLE
            )
        );
    }
}
