use super::route_explain_contract::explain_value;
use super::route_explain_meta::{
    ROUTE_EXPLAIN_RECOVERY_FIELDS_CHECKSUM, ROUTE_EXPLAIN_RECOVERY_SCHEMA_VERSION,
};

pub(crate) struct AutoRecoveryProjection {
    pub(crate) attempts: usize,
    pub(crate) final_result: String,
    pub(crate) retry_budget_exhausted: bool,
    pub(crate) schema_version: &'static str,
    pub(crate) fields_checksum: &'static str,
}

pub(crate) fn project_auto_recovery(lines: &[String]) -> AutoRecoveryProjection {
    let attempts = explain_value(lines, "auto_recovery_attempts=")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let final_result = explain_value(lines, "auto_recovery_final_result=")
        .unwrap_or("not_triggered")
        .to_string();
    let retry_budget_exhausted = attempts > 0
        && matches!(
            final_result.as_str(),
            "no_candidate_after_relax_health"
                | "no_candidate_after_relax"
                | "no_candidate_after_last_chance_health"
        );
    AutoRecoveryProjection {
        attempts,
        final_result,
        retry_budget_exhausted,
        schema_version: ROUTE_EXPLAIN_RECOVERY_SCHEMA_VERSION,
        fields_checksum: ROUTE_EXPLAIN_RECOVERY_FIELDS_CHECKSUM,
    }
}
