use super::options::MeshRouteExplainOptions;
use super::route_explain_envelope::insert_route_explain_envelope;
use super::route_explain_error_consts::*;
use super::route_explain_integrity::build_route_explain_contract_integrity;
use super::route_explain_json_insert::insert_json;
use super::route_explain_meta::{
    ROUTE_EXPLAIN_CONTRACT_VERSION, ROUTE_EXPLAIN_ERROR_HEALTH_GATE,
    ROUTE_EXPLAIN_ERROR_HEALTH_SUMMARY, ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH,
    ROUTE_EXPLAIN_ERROR_OPERATOR_PRESSURE, ROUTE_EXPLAIN_ERROR_OPERATOR_SELECTED,
    ROUTE_EXPLAIN_KIND_ERROR, ROUTE_EXPLAIN_NETWORK_STATE_NOT_MODIFIED,
    ROUTE_EXPLAIN_RECOVERY_FIELDS_CHECKSUM, ROUTE_EXPLAIN_RECOVERY_SCHEMA_VERSION,
    ROUTE_EXPLAIN_STATUS_ERROR,
};
use super::route_explain_recovery_projection::build_connect_recovery_projection;

struct ErrorStageProfile {
    action: &'static str,
    category: &'static str,
    resolution: &'static str,
}

struct ErrorOperatorBlock {
    summary: String,
    route_key: String,
    contract_integrity: String,
}

struct ErrorDiagnosticsBlock {
    category: String,
    retriable: String,
    action: String,
    retry_backoff_hint: String,
    resolution_hint: String,
    signature: String,
    route_key: String,
}

const ROUTE_EXPLAIN_ERROR_JSON_FIELD_COUNT: usize = 36;
const ERROR_AUTO_RECOVERY_ATTEMPTS: &str = "0";
const ERROR_AUTO_RECOVERY_FINAL_RESULT: &str = "not_applicable_error";
const ERROR_CONNECT_RETRY_BUDGET_EXHAUSTED: &str = "unknown";

#[cfg(test)]
pub(crate) fn route_explain_error_json_field_count() -> usize {
    ROUTE_EXPLAIN_ERROR_JSON_FIELD_COUNT
}

pub(crate) fn emit_route_explain_error(
    options: &MeshRouteExplainOptions,
    error_stage: &str,
    error: &str,
) -> i32 {
    let json = build_route_explain_error_json(options, error_stage, error);
    if let Some(path) = options.out_path.as_deref()
        && let Err(write_error) = std::fs::write(path, &json)
    {
        eprintln!("Не удалось записать ошибку mesh route explain: {write_error}");
        return 1;
    }
    if options.json_output {
        println!("{json}");
    } else {
        eprintln!("mesh route explain: этап '{error_stage}' завершился ошибкой: {error}");
    }
    2
}

pub(crate) fn build_route_explain_error_json(
    options: &MeshRouteExplainOptions,
    error_stage: &str,
    error: &str,
) -> String {
    build_route_explain_error_json_with_identity(
        &options.namespace,
        &options.node_name,
        error_stage,
        error,
    )
}

pub(crate) fn build_route_explain_error_json_with_identity(
    namespace: &str,
    node_name: &str,
    error_stage: &str,
    error: &str,
) -> String {
    let mut object = serde_json::Map::with_capacity(ROUTE_EXPLAIN_ERROR_JSON_FIELD_COUNT);
    let profile = error_stage_profile(error_stage);
    let retriable = if error_retriable(profile.category) {
        RETRIABLE_TRUE
    } else {
        RETRIABLE_FALSE
    };
    let operator = build_error_operator_block(profile.action, error_stage);
    let diagnostics = build_error_diagnostics_block(
        error_stage,
        profile.category,
        retriable,
        profile.action,
        retry_backoff_hint(profile.category, retriable),
        profile.resolution,
    );
    insert_route_explain_envelope(
        &mut object,
        ROUTE_EXPLAIN_STATUS_ERROR,
        ROUTE_EXPLAIN_KIND_ERROR,
        ROUTE_EXPLAIN_CONTRACT_VERSION,
        namespace,
        node_name,
    );
    insert_json(&mut object, "error_stage", error_stage);
    insert_json(&mut object, "error", error);
    insert_error_operator_json(&mut object, &operator, error_stage, profile.action);
    insert_error_diagnostics_json(&mut object, &diagnostics);
    insert_error_recovery_json(&mut object, profile.action);
    insert_json(
        &mut object,
        "route_explain_health_gate",
        ROUTE_EXPLAIN_ERROR_HEALTH_GATE,
    );
    insert_json(
        &mut object,
        "route_explain_health_summary",
        ROUTE_EXPLAIN_ERROR_HEALTH_SUMMARY,
    );
    insert_json(
        &mut object,
        "network_state",
        ROUTE_EXPLAIN_NETWORK_STATE_NOT_MODIFIED,
    );
    serde_json::Value::Object(object).to_string()
}

fn build_error_operator_block(action: &str, error_stage: &str) -> ErrorOperatorBlock {
    let summary = format!(
        "health:{};selected:{};pressure:{};action:{};reason:{}",
        ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH,
        ROUTE_EXPLAIN_ERROR_OPERATOR_SELECTED,
        ROUTE_EXPLAIN_ERROR_OPERATOR_PRESSURE,
        action,
        error_stage
    );
    let route_key = format!("error:{action}");
    let contract_integrity = build_route_explain_contract_integrity(
        &summary,
        &summary,
        &route_key,
        ROUTE_EXPLAIN_ERROR_HEALTH_GATE,
        ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH,
        action,
    );
    ErrorOperatorBlock {
        summary,
        route_key,
        contract_integrity,
    }
}

fn build_error_diagnostics_block(
    stage: &str,
    category: &str,
    retriable: &str,
    action: &str,
    retry_backoff_hint: &str,
    resolution_hint: &str,
) -> ErrorDiagnosticsBlock {
    let signature = format!(
        "stage:{};category:{};retriable:{};action:{}",
        stage, category, retriable, action
    );
    let route_key = format!("{category}:{action}");
    ErrorDiagnosticsBlock {
        category: category.to_string(),
        retriable: retriable.to_string(),
        action: action.to_string(),
        retry_backoff_hint: retry_backoff_hint.to_string(),
        resolution_hint: resolution_hint.to_string(),
        signature,
        route_key,
    }
}

fn insert_error_operator_json(
    object: &mut serde_json::Map<String, serde_json::Value>,
    operator: &ErrorOperatorBlock,
    error_stage: &str,
    action: &str,
) {
    insert_json(object, "route_explain_operator_summary", &operator.summary);
    insert_json(
        object,
        "route_explain_operator_signature",
        &operator.summary,
    );
    insert_json(
        object,
        "route_explain_operator_route_key",
        &operator.route_key,
    );
    insert_json(
        object,
        "route_explain_operator_health",
        ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH,
    );
    insert_json(
        object,
        "route_explain_operator_selected",
        ROUTE_EXPLAIN_ERROR_OPERATOR_SELECTED,
    );
    insert_json(object, "route_explain_operator_action", action);
    insert_json(
        object,
        "route_explain_operator_pressure",
        ROUTE_EXPLAIN_ERROR_OPERATOR_PRESSURE,
    );
    insert_json(object, "route_explain_operator_reason", error_stage);
    insert_json(
        object,
        "route_explain_contract_integrity",
        &operator.contract_integrity,
    );
}

fn insert_error_diagnostics_json(
    object: &mut serde_json::Map<String, serde_json::Value>,
    diagnostics: &ErrorDiagnosticsBlock,
) {
    insert_json(object, "route_explain_error_action", &diagnostics.action);
    insert_json(
        object,
        "route_explain_error_stage_category",
        &diagnostics.category,
    );
    insert_json(
        object,
        "route_explain_error_retriable",
        &diagnostics.retriable,
    );
    insert_json(
        object,
        "route_explain_error_retry_backoff_hint",
        &diagnostics.retry_backoff_hint,
    );
    insert_json(
        object,
        "route_explain_error_resolution_hint",
        &diagnostics.resolution_hint,
    );
    insert_json(
        object,
        "route_explain_error_signature",
        &diagnostics.signature,
    );
    insert_json(
        object,
        "route_explain_error_route_key",
        &diagnostics.route_key,
    );
}

fn insert_error_recovery_json(
    object: &mut serde_json::Map<String, serde_json::Value>,
    action: &str,
) {
    let projection = build_connect_recovery_projection(action);
    insert_json(
        object,
        "auto_recovery_attempts",
        ERROR_AUTO_RECOVERY_ATTEMPTS,
    );
    insert_json(
        object,
        "auto_recovery_final_result",
        ERROR_AUTO_RECOVERY_FINAL_RESULT,
    );
    insert_json(
        object,
        "connect_retry_budget_exhausted",
        ERROR_CONNECT_RETRY_BUDGET_EXHAUSTED,
    );
    insert_json(
        object,
        "connect_recovery_needed",
        if projection.needed { "true" } else { "false" },
    );
    insert_json(object, "connect_recovery_strategy", projection.strategy);
    insert_json(
        object,
        "connect_recovery_projection_consistency",
        if projection.consistency {
            "true"
        } else {
            "false"
        },
    );
    insert_json(object, "connect_recovery_projection_key", &projection.key);
    insert_json(
        object,
        "route_explain_recovery_schema_version",
        ROUTE_EXPLAIN_RECOVERY_SCHEMA_VERSION,
    );
    insert_json(
        object,
        "route_explain_recovery_fields_checksum",
        ROUTE_EXPLAIN_RECOVERY_FIELDS_CHECKSUM,
    );
}

#[cfg(test)]
pub(crate) fn error_action_hint(error_stage: &str) -> &'static str {
    error_stage_profile(error_stage).action
}

#[cfg(test)]
pub(crate) fn error_stage_category(error_stage: &str) -> &'static str {
    error_stage_profile(error_stage).category
}

pub(crate) fn error_retriable(stage_category: &str) -> bool {
    matches!(
        stage_category,
        CATEGORY_RUNTIME | CATEGORY_PLANNING | CATEGORY_HEALTH
    )
}

pub(crate) fn retry_backoff_hint(stage_category: &str, retriable: &str) -> &'static str {
    if retriable == RETRIABLE_FALSE {
        return BACKOFF_AFTER_FIX;
    }
    match stage_category {
        CATEGORY_RUNTIME => BACKOFF_IMMEDIATE,
        CATEGORY_PLANNING | CATEGORY_HEALTH => BACKOFF_SHORT,
        _ => BACKOFF_AFTER_FIX,
    }
}

pub(crate) fn resolution_hint(action: &str) -> &'static str {
    match action {
        ACTION_FIX_PEER_SPEC => RESOLUTION_CHECK_PEER_SPEC_FORMAT,
        ACTION_FIX_TABLE_POLICY => RESOLUTION_ADJUST_TABLE_POLICY_BOUNDS,
        ACTION_FIX_POLICY_PAYLOAD => RESOLUTION_FIX_POLICY_PAYLOAD_SYNTAX,
        ACTION_ADJUST_POLICY_OR_PEERS => RESOLUTION_RELAX_POLICY_OR_ADD_CANDIDATES,
        ACTION_INSPECT_DISCOVERY_INPUT => RESOLUTION_INSPECT_DISCOVERY_RECORDS,
        ACTION_INSPECT_FAILOVER_INPUT => RESOLUTION_INSPECT_FAILOVER_EVENT,
        ACTION_INSPECT_HEALTH_INPUTS => RESOLUTION_INSPECT_HEALTH_STATE,
        ACTION_INSPECT_RUNTIME_BOOTSTRAP => RESOLUTION_INSPECT_RUNTIME_NAMESPACE,
        _ => RESOLUTION_INSPECT_ERROR_DETAILS,
    }
}

fn error_stage_profile(error_stage: &str) -> ErrorStageProfile {
    let (action, category) = error_stage_action_category(error_stage);
    ErrorStageProfile {
        action,
        category,
        resolution: resolution_hint(action),
    }
}

fn error_stage_action_category(error_stage: &str) -> (&'static str, &'static str) {
    match error_stage {
        STAGE_OPTIONS_PARSE => (ACTION_INSPECT_ERROR, CATEGORY_INPUT),
        STAGE_PEER_SPEC => (ACTION_FIX_PEER_SPEC, CATEGORY_INPUT),
        STAGE_SIMULATION_INPUT => (ACTION_INSPECT_DISCOVERY_INPUT, CATEGORY_INPUT),
        STAGE_PEER_TABLE_POLICY => (ACTION_FIX_TABLE_POLICY, CATEGORY_POLICY),
        STAGE_POLICY_PARSE => (ACTION_FIX_POLICY_PAYLOAD, CATEGORY_POLICY),
        STAGE_PLAN_PATH => (ACTION_ADJUST_POLICY_OR_PEERS, CATEGORY_PLANNING),
        STAGE_DISCOVERY_MERGE => (ACTION_INSPECT_DISCOVERY_INPUT, CATEGORY_PLANNING),
        STAGE_FAILOVER_PLAN => (ACTION_INSPECT_FAILOVER_INPUT, CATEGORY_PLANNING),
        STAGE_HEALTH_STATE_UPDATE | STAGE_RESELECTION_PLAN => {
            (ACTION_INSPECT_HEALTH_INPUTS, CATEGORY_HEALTH)
        }
        STAGE_RUNTIME_BOOTSTRAP => (ACTION_INSPECT_RUNTIME_BOOTSTRAP, CATEGORY_RUNTIME),
        _ => (ACTION_INSPECT_ERROR, CATEGORY_UNKNOWN),
    }
}
