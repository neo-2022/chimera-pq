pub(crate) const RETRIABLE_TRUE: &str = "true";
pub(crate) const RETRIABLE_FALSE: &str = "false";

pub(crate) const CATEGORY_INPUT: &str = "input";
pub(crate) const CATEGORY_POLICY: &str = "policy";
pub(crate) const CATEGORY_PLANNING: &str = "planning";
pub(crate) const CATEGORY_HEALTH: &str = "health";
pub(crate) const CATEGORY_RUNTIME: &str = "runtime";
pub(crate) const CATEGORY_UNKNOWN: &str = "unknown";

pub(crate) const BACKOFF_AFTER_FIX: &str = "after_fix";
pub(crate) const BACKOFF_IMMEDIATE: &str = "immediate";
pub(crate) const BACKOFF_SHORT: &str = "short_backoff";

pub(crate) const ACTION_FIX_PEER_SPEC: &str = "fix_peer_spec";
pub(crate) const ACTION_FIX_TABLE_POLICY: &str = "fix_table_policy";
pub(crate) const ACTION_FIX_POLICY_PAYLOAD: &str = "fix_policy_payload";
pub(crate) const ACTION_ADJUST_POLICY_OR_PEERS: &str = "adjust_policy_or_peers";
pub(crate) const ACTION_INSPECT_DISCOVERY_INPUT: &str = "inspect_discovery_input";
pub(crate) const ACTION_INSPECT_FAILOVER_INPUT: &str = "inspect_failover_input";
pub(crate) const ACTION_INSPECT_HEALTH_INPUTS: &str = "inspect_health_inputs";
pub(crate) const ACTION_INSPECT_RUNTIME_BOOTSTRAP: &str = "inspect_runtime_bootstrap";
pub(crate) const ACTION_INSPECT_ERROR: &str = "inspect_error";

pub(crate) const RESOLUTION_CHECK_PEER_SPEC_FORMAT: &str = "check_peer_spec_format";
pub(crate) const RESOLUTION_ADJUST_TABLE_POLICY_BOUNDS: &str = "adjust_table_policy_bounds";
pub(crate) const RESOLUTION_FIX_POLICY_PAYLOAD_SYNTAX: &str = "fix_policy_payload_syntax";
pub(crate) const RESOLUTION_RELAX_POLICY_OR_ADD_CANDIDATES: &str = "relax_policy_or_add_candidates";
pub(crate) const RESOLUTION_INSPECT_DISCOVERY_RECORDS: &str = "inspect_discovery_records";
pub(crate) const RESOLUTION_INSPECT_FAILOVER_EVENT: &str = "inspect_failover_event";
pub(crate) const RESOLUTION_INSPECT_HEALTH_STATE: &str = "inspect_health_state";
pub(crate) const RESOLUTION_INSPECT_RUNTIME_NAMESPACE: &str = "inspect_runtime_namespace";
pub(crate) const RESOLUTION_INSPECT_ERROR_DETAILS: &str = "inspect_error_details";

pub(crate) const STAGE_PEER_SPEC: &str = "peer_spec";
pub(crate) const STAGE_PEER_TABLE_POLICY: &str = "peer_table_policy";
pub(crate) const STAGE_POLICY_PARSE: &str = "policy_parse";
pub(crate) const STAGE_OPTIONS_PARSE: &str = "options_parse";
pub(crate) const STAGE_SIMULATION_INPUT: &str = "simulation_input";
pub(crate) const STAGE_PLAN_PATH: &str = "plan_path";
pub(crate) const STAGE_DISCOVERY_MERGE: &str = "discovery_merge";
pub(crate) const STAGE_FAILOVER_PLAN: &str = "failover_plan";
pub(crate) const STAGE_HEALTH_STATE_UPDATE: &str = "health_state_update";
pub(crate) const STAGE_RESELECTION_PLAN: &str = "reselection_plan";
pub(crate) const STAGE_RUNTIME_BOOTSTRAP: &str = "runtime_bootstrap";
