#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

const EXPECTED_RECOVERY_SCHEMA: &str = "mesh_recovery_v5";
const EXPECTED_RECOVERY_CHECKSUM: &str = "auto_recovery_attempts|auto_recovery_final_result|connect_retry_budget_exhausted|connect_recovery_needed|connect_recovery_strategy|connect_recovery_projection_consistency|connect_recovery_projection_key";

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/MESH_ROUTE_EXPLAIN_ERROR.json");

    let raw = fs::read_to_string(path).unwrap_or_else(|_| {
        fail(&format!(
            "mesh route explain error guard: missing file: {path}"
        ))
    });
    let root: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail("mesh route explain error guard: invalid json"));
    let obj = root
        .as_object()
        .unwrap_or_else(|| fail("mesh route explain error guard: root not object"));
    validate_obj(obj).unwrap_or_else(|msg| fail(&msg));
    println!("mesh route explain error guard: PASS");
}

fn validate_obj(obj: &serde_json::Map<String, Value>) -> Result<(), String> {
    require_str(obj, "status", "error")?;
    require_str(obj, "kind", "mesh_route_explain_error")?;
    require_str(obj, "route_explain_contract_family", "mesh_route_explain")?;
    require_str(obj, "explain_contract_version", "mesh_explain_v1")?;
    require_str(obj, "network_state", "not_modified")?;
    require_str(obj, "namespace", "cef-public")?;
    require_str(obj, "node", "node-client")?;

    require_non_empty_str(obj, "error_stage")?;
    require_non_empty_str(obj, "error")?;

    // Operator block invariants
    require_str(obj, "route_explain_operator_health", "error")?;
    require_str(obj, "route_explain_operator_selected", "none")?;
    require_str(obj, "route_explain_operator_pressure", "unknown")?;
    require_non_empty_str(obj, "route_explain_operator_action")?;
    require_non_empty_str(obj, "route_explain_operator_reason")?;

    let operator_summary = require_non_empty_str(obj, "route_explain_operator_summary")?;
    let operator_signature = require_non_empty_str(obj, "route_explain_operator_signature")?;
    if operator_summary != operator_signature {
        return Err(
            "mesh route explain error guard: operator summary/signature mismatch".to_string(),
        );
    }

    // Recovery parity block invariants for error envelope
    require_str(
        obj,
        "route_explain_recovery_schema_version",
        EXPECTED_RECOVERY_SCHEMA,
    )?;
    require_str(
        obj,
        "route_explain_recovery_fields_checksum",
        EXPECTED_RECOVERY_CHECKSUM,
    )?;
    require_str(obj, "auto_recovery_attempts", "0")?;
    require_str(obj, "auto_recovery_final_result", "not_applicable_error")?;
    require_str(obj, "connect_retry_budget_exhausted", "unknown")?;
    require_str(obj, "connect_recovery_needed", "false")?;
    require_str(obj, "connect_recovery_strategy", "none")?;
    require_str(obj, "connect_recovery_projection_consistency", "true")?;

    let action = require_non_empty_str(obj, "route_explain_error_action")?;
    let expected_projection_key = format!("needed:false;strategy:none;action:{action}");
    require_str(
        obj,
        "connect_recovery_projection_key",
        &expected_projection_key,
    )?;
    Ok(())
}

fn require_str<'a>(
    obj: &'a serde_json::Map<String, Value>,
    key: &str,
    expected: &str,
) -> Result<&'a str, String> {
    let value = obj
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("mesh route explain error guard: missing field: {key}"))?;
    if value != expected {
        return Err(format!(
            "mesh route explain error guard: field mismatch: {key}"
        ));
    }
    Ok(value)
}

fn require_non_empty_str<'a>(
    obj: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a str, String> {
    let value = obj
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("mesh route explain error guard: missing field: {key}"))?;
    if value.is_empty() {
        return Err(format!(
            "mesh route explain error guard: empty string field: {key}"
        ));
    }
    Ok(value)
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::validate_obj;
    use serde_json::{Map, Value};

    fn base_error(action: &str) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("status".into(), Value::String("error".into()));
        m.insert(
            "kind".into(),
            Value::String("mesh_route_explain_error".into()),
        );
        m.insert(
            "route_explain_contract_family".into(),
            Value::String("mesh_route_explain".into()),
        );
        m.insert(
            "explain_contract_version".into(),
            Value::String("mesh_explain_v1".into()),
        );
        m.insert("network_state".into(), Value::String("not_modified".into()));
        m.insert("namespace".into(), Value::String("cef-public".into()));
        m.insert("node".into(), Value::String("node-client".into()));
        m.insert("error_stage".into(), Value::String("policy_parse".into()));
        m.insert(
            "error".into(),
            Value::String("mesh policy max_peers must be > 0".into()),
        );
        m.insert(
            "route_explain_operator_health".into(),
            Value::String("error".into()),
        );
        m.insert(
            "route_explain_operator_selected".into(),
            Value::String("none".into()),
        );
        m.insert(
            "route_explain_operator_pressure".into(),
            Value::String("unknown".into()),
        );
        m.insert(
            "route_explain_operator_action".into(),
            Value::String(action.into()),
        );
        m.insert(
            "route_explain_operator_reason".into(),
            Value::String("policy_parse".into()),
        );
        let summary = format!(
            "health:error;selected:none;pressure:unknown;action:{action};reason:policy_parse"
        );
        m.insert(
            "route_explain_operator_summary".into(),
            Value::String(summary.clone()),
        );
        m.insert(
            "route_explain_operator_signature".into(),
            Value::String(summary),
        );
        m.insert(
            "route_explain_recovery_schema_version".into(),
            Value::String("mesh_recovery_v5".into()),
        );
        m.insert(
            "route_explain_recovery_fields_checksum".into(),
            Value::String("auto_recovery_attempts|auto_recovery_final_result|connect_retry_budget_exhausted|connect_recovery_needed|connect_recovery_strategy|connect_recovery_projection_consistency|connect_recovery_projection_key".into()),
        );
        m.insert("auto_recovery_attempts".into(), Value::String("0".into()));
        m.insert(
            "auto_recovery_final_result".into(),
            Value::String("not_applicable_error".into()),
        );
        m.insert(
            "connect_retry_budget_exhausted".into(),
            Value::String("unknown".into()),
        );
        m.insert(
            "connect_recovery_needed".into(),
            Value::String("false".into()),
        );
        m.insert(
            "connect_recovery_strategy".into(),
            Value::String("none".into()),
        );
        m.insert(
            "connect_recovery_projection_consistency".into(),
            Value::String("true".into()),
        );
        m.insert(
            "route_explain_error_action".into(),
            Value::String(action.into()),
        );
        m.insert(
            "connect_recovery_projection_key".into(),
            Value::String(format!("needed:false;strategy:none;action:{action}")),
        );
        m
    }

    #[test]
    fn validate_accepts_valid_error_contract() {
        let m = base_error("fix_policy_payload");
        assert!(validate_obj(&m).is_ok());
    }

    #[test]
    fn validate_rejects_mismatched_projection_key() {
        let mut m = base_error("fix_policy_payload");
        m.insert(
            "connect_recovery_projection_key".into(),
            Value::String("needed:false;strategy:none;action:wrong_action".into()),
        );
        let err = match validate_obj(&m) {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("field mismatch: connect_recovery_projection_key"));
    }

    #[test]
    fn validate_rejects_recovery_schema_mismatch() {
        let mut m = base_error("fix_policy_payload");
        m.insert(
            "route_explain_recovery_schema_version".into(),
            Value::String("mesh_recovery_v4".into()),
        );
        let err = match validate_obj(&m) {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("field mismatch: route_explain_recovery_schema_version"));
    }

    #[test]
    fn validate_rejects_blank_error_stage() {
        let mut m = base_error("fix_policy_payload");
        m.insert("error_stage".into(), Value::String(String::new()));
        let err = match validate_obj(&m) {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("empty string field: error_stage"));
    }
}
