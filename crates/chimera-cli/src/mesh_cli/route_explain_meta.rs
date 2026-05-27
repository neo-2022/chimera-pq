pub(crate) const ROUTE_EXPLAIN_CONTRACT_FAMILY: &str = "mesh_route_explain";
pub(crate) const ROUTE_EXPLAIN_STATUS_OK: &str = "ok";
pub(crate) const ROUTE_EXPLAIN_STATUS_ERROR: &str = "error";
pub(crate) const ROUTE_EXPLAIN_KIND_OK: &str = "mesh_route_explain";
pub(crate) const ROUTE_EXPLAIN_KIND_ERROR: &str = "mesh_route_explain_error";
pub(crate) const ROUTE_EXPLAIN_CONTRACT_VERSION: &str = "mesh_explain_v1";
pub(crate) const ROUTE_EXPLAIN_RECOVERY_SCHEMA_VERSION: &str = "mesh_recovery_v5";
pub(crate) const ROUTE_EXPLAIN_RECOVERY_FIELDS_CHECKSUM: &str = "auto_recovery_attempts|auto_recovery_final_result|connect_retry_budget_exhausted|connect_recovery_needed|connect_recovery_strategy|connect_recovery_projection_consistency|connect_recovery_projection_key";
pub(crate) const ROUTE_EXPLAIN_NETWORK_STATE_NOT_MODIFIED: &str = "not_modified";
pub(crate) const ROUTE_EXPLAIN_ERROR_HEALTH_GATE: &str = "error";
pub(crate) const ROUTE_EXPLAIN_ERROR_HEALTH_SUMMARY: &str =
    "table:error;degraded:unknown;pressure_projection:unknown";
pub(crate) const ROUTE_EXPLAIN_ERROR_OPERATOR_HEALTH: &str = "error";
pub(crate) const ROUTE_EXPLAIN_ERROR_OPERATOR_SELECTED: &str = "none";
pub(crate) const ROUTE_EXPLAIN_ERROR_OPERATOR_PRESSURE: &str = "unknown";
