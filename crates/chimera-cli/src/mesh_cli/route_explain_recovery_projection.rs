pub(crate) struct ConnectRecoveryProjection<'a> {
    pub(crate) needed: bool,
    pub(crate) strategy: &'a str,
    pub(crate) consistency: bool,
    pub(crate) key: String,
}

const RECOVERY_ACTION_RETRY_CONNECT_ENDPOINTS: &str = "retry_connect_endpoints";
const RECOVERY_STRATEGY_NONE: &str = "none";

pub(crate) fn build_connect_recovery_projection(
    route_explain_operator_action: &str,
) -> ConnectRecoveryProjection<'static> {
    let needed = route_explain_operator_action == RECOVERY_ACTION_RETRY_CONNECT_ENDPOINTS;
    let strategy = if needed {
        RECOVERY_ACTION_RETRY_CONNECT_ENDPOINTS
    } else {
        RECOVERY_STRATEGY_NONE
    };
    let consistency = needed == (strategy == RECOVERY_ACTION_RETRY_CONNECT_ENDPOINTS);
    let key = format!(
        "needed:{};strategy:{};action:{}",
        if needed { "true" } else { "false" },
        strategy,
        route_explain_operator_action
    );

    ConnectRecoveryProjection {
        needed,
        strategy,
        consistency,
        key,
    }
}

#[cfg(test)]
mod tests {
    use super::{RECOVERY_ACTION_RETRY_CONNECT_ENDPOINTS, build_connect_recovery_projection};

    #[test]
    fn projection_maps_retry_action_to_needed_true() {
        let projection = build_connect_recovery_projection(RECOVERY_ACTION_RETRY_CONNECT_ENDPOINTS);
        assert!(projection.needed);
        assert_eq!(projection.strategy, RECOVERY_ACTION_RETRY_CONNECT_ENDPOINTS);
        assert!(projection.consistency);
        assert_eq!(
            projection.key,
            "needed:true;strategy:retry_connect_endpoints;action:retry_connect_endpoints"
        );
    }

    #[test]
    fn projection_maps_other_actions_to_none_strategy() {
        let projection = build_connect_recovery_projection("use_selected_path");
        assert!(!projection.needed);
        assert_eq!(projection.strategy, "none");
        assert!(projection.consistency);
        assert_eq!(
            projection.key,
            "needed:false;strategy:none;action:use_selected_path"
        );
    }
}
