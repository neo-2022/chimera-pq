#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

const EXPECTED_SCHEMA_VERSION: &str = "mesh_recovery_v5";
const EXPECTED_FIELDS_CHECKSUM: &str = "auto_recovery_attempts|auto_recovery_final_result|connect_retry_budget_exhausted|connect_recovery_needed|connect_recovery_strategy|connect_recovery_projection_consistency|connect_recovery_projection_key";
const RETRY_ACTION: &str = "retry_connect_endpoints";

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/MESH_ROUTE_EXPLAIN.json");

    let raw = fs::read_to_string(path).unwrap_or_else(|_| {
        fail(&format!(
            "mesh cli recovery schema guard: missing file: {path}"
        ))
    });
    let root: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail("mesh cli recovery schema guard: invalid json"));
    let obj = root
        .as_object()
        .unwrap_or_else(|| fail("mesh cli recovery schema guard: root not object"));
    validate_obj(obj).unwrap_or_else(|msg| fail(&msg));
    println!("mesh cli recovery schema guard: PASS");
}

fn validate_obj(obj: &serde_json::Map<String, Value>) -> Result<(), String> {
    let status = obj
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| "mesh cli recovery schema guard: status missing".to_string())?;
    if !matches!(status, "ok" | "error") {
        return Err("mesh cli recovery schema guard: status must be ok|error".to_string());
    }
    let kind = obj
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "mesh cli recovery schema guard: kind missing".to_string())?;
    let expected_kind = if status == "ok" {
        "mesh_route_explain"
    } else {
        "mesh_route_explain_error"
    };
    if kind != expected_kind {
        return Err("mesh cli recovery schema guard: kind does not match status".to_string());
    }
    require_str(
        obj,
        "route_explain_recovery_schema_version",
        EXPECTED_SCHEMA_VERSION,
    )?;
    require_str(
        obj,
        "route_explain_recovery_fields_checksum",
        EXPECTED_FIELDS_CHECKSUM,
    )?;

    let connect_recovery_needed = require_bool_text(obj, "connect_recovery_needed")?;
    let connect_recovery_strategy = obj
        .get("connect_recovery_strategy")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "mesh cli recovery schema guard: connect_recovery_strategy missing".to_string()
        })?;
    if !matches!(
        connect_recovery_strategy,
        "none" | "retry_connect_endpoints"
    ) {
        return Err(
            "mesh cli recovery schema guard: connect_recovery_strategy invalid".to_string(),
        );
    }
    let connect_recovery_projection_consistency =
        require_bool_text(obj, "connect_recovery_projection_consistency")?;
    let connect_recovery_projection_key = obj
        .get("connect_recovery_projection_key")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "mesh cli recovery schema guard: connect_recovery_projection_key missing".to_string()
        })?;
    let connect_retry_budget_exhausted = obj
        .get("connect_retry_budget_exhausted")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "mesh cli recovery schema guard: connect_retry_budget_exhausted missing".to_string()
        })?;
    if !matches!(connect_retry_budget_exhausted, "true" | "false" | "unknown") {
        return Err(
            "mesh cli recovery schema guard: connect_retry_budget_exhausted invalid".to_string(),
        );
    }

    let attempts = obj
        .get("auto_recovery_attempts")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "mesh cli recovery schema guard: auto_recovery_attempts missing".to_string()
        })?;
    let attempts_num: u32 = attempts.parse().map_err(|_| {
        "mesh cli recovery schema guard: auto_recovery_attempts not u32".to_string()
    })?;

    let final_result = obj
        .get("auto_recovery_final_result")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "mesh cli recovery schema guard: auto_recovery_final_result missing".to_string()
        })?;
    if final_result.is_empty() {
        return Err("mesh cli recovery schema guard: auto_recovery_final_result empty".to_string());
    }

    let operator_action = obj
        .get("route_explain_operator_action")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "mesh cli recovery schema guard: route_explain_operator_action missing".to_string()
        })?;
    let summary = obj
        .get("route_explain_operator_summary")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "mesh cli recovery schema guard: route_explain_operator_summary missing".to_string()
        })?;
    let summary_action = summary_action(summary)?;
    if summary_action != operator_action {
        return Err(
            "mesh cli recovery schema guard: operator action mismatch between summary and field"
                .to_string(),
        );
    }

    let expected_recovery_needed = if status == "error" {
        false
    } else {
        operator_action == RETRY_ACTION
    };
    if connect_recovery_needed != expected_recovery_needed {
        return Err(
            "mesh cli recovery schema guard: connect_recovery_needed does not match operator action"
                .to_string(),
        );
    }
    let expected_strategy = if connect_recovery_needed {
        RETRY_ACTION
    } else {
        "none"
    };
    if connect_recovery_strategy != expected_strategy {
        return Err("mesh cli recovery schema guard: connect_recovery_strategy does not match recovery-needed state".to_string());
    }
    if !connect_recovery_projection_consistency {
        return Err(
            "mesh cli recovery schema guard: connect_recovery_projection_consistency is false"
                .to_string(),
        );
    }
    let expected_projection_key = format!(
        "needed:{};strategy:{};action:{}",
        if connect_recovery_needed {
            "true"
        } else {
            "false"
        },
        expected_strategy,
        operator_action
    );
    if connect_recovery_projection_key != expected_projection_key {
        return Err(
            "mesh cli recovery schema guard: connect_recovery_projection_key mismatch".to_string(),
        );
    }

    if status == "error" {
        if attempts_num != 0 {
            return Err(
                "mesh cli recovery schema guard: error envelope must have auto_recovery_attempts=0"
                    .to_string(),
            );
        }
        if final_result != "not_applicable_error" {
            return Err(
                "mesh cli recovery schema guard: error envelope final result mismatch".to_string(),
            );
        }
        if connect_retry_budget_exhausted != "unknown" {
            return Err(
                "mesh cli recovery schema guard: error envelope retry budget marker mismatch"
                    .to_string(),
            );
        }
    }

    if status == "ok" {
        let explain = obj
            .get("explain")
            .and_then(Value::as_str)
            .ok_or_else(|| "mesh cli recovery schema guard: explain missing".to_string())?;
        for needle in [
            "auto_recovery_attempts=",
            "auto_recovery_final_result=",
            "auto_recovery_trace_consistent=",
        ] {
            if !explain.contains(needle) {
                return Err(
                    "mesh cli recovery schema guard: explain missing recovery token".to_string(),
                );
            }
        }
    }

    Ok(())
}

fn require_str(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    expected: &str,
) -> Result<(), String> {
    if obj.get(key).and_then(Value::as_str) != Some(expected) {
        return Err(format!(
            "mesh cli recovery schema guard: field mismatch: {key}"
        ));
    }
    Ok(())
}

fn require_bool_text(obj: &serde_json::Map<String, Value>, key: &str) -> Result<bool, String> {
    match obj.get(key).and_then(Value::as_str) {
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        _ => Err(format!(
            "mesh cli recovery schema guard: expected bool text for {key}"
        )),
    }
}

fn summary_action(summary: &str) -> Result<&str, String> {
    for part in summary.split(';') {
        if let Some(action) = part.strip_prefix("action:") {
            return Ok(action);
        }
    }
    Err("mesh cli recovery schema guard: action token missing in summary".to_string())
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::validate_obj;
    use serde_json::{Map, Value};

    fn base(status: &str, kind: &str, action: &str) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("status".into(), Value::String(status.into()));
        m.insert("kind".into(), Value::String(kind.into()));
        m.insert(
            "route_explain_recovery_schema_version".into(),
            Value::String("mesh_recovery_v5".into()),
        );
        m.insert(
            "route_explain_recovery_fields_checksum".into(),
            Value::String("auto_recovery_attempts|auto_recovery_final_result|connect_retry_budget_exhausted|connect_recovery_needed|connect_recovery_strategy|connect_recovery_projection_consistency|connect_recovery_projection_key".into()),
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
            "connect_recovery_projection_key".into(),
            Value::String(format!("needed:false;strategy:none;action:{action}")),
        );
        m.insert("auto_recovery_attempts".into(), Value::String("0".into()));
        m.insert(
            "route_explain_operator_action".into(),
            Value::String(action.into()),
        );
        m.insert(
            "route_explain_operator_summary".into(),
            Value::String(format!(
                "health:{status};selected:none;pressure:unknown;action:{action};reason:r"
            )),
        );
        m
    }

    #[test]
    fn validate_accepts_error_envelope_parity_shape() {
        let mut m = base("error", "mesh_route_explain_error", "fix_policy_payload");
        m.insert(
            "connect_retry_budget_exhausted".into(),
            Value::String("unknown".into()),
        );
        m.insert(
            "auto_recovery_final_result".into(),
            Value::String("not_applicable_error".into()),
        );
        assert!(validate_obj(&m).is_ok());
    }

    #[test]
    fn validate_accepts_ok_envelope_with_explain_tokens() {
        let mut m = base("ok", "mesh_route_explain", "use_selected_path");
        m.insert(
            "connect_retry_budget_exhausted".into(),
            Value::String("false".into()),
        );
        m.insert(
            "auto_recovery_final_result".into(),
            Value::String("not_triggered".into()),
        );
        m.insert(
            "explain".into(),
            Value::String("auto_recovery_attempts=0 | auto_recovery_final_result=not_triggered | auto_recovery_trace_consistent=true".into()),
        );
        assert!(validate_obj(&m).is_ok());
    }

    #[test]
    fn validate_rejects_error_wrong_final_result() {
        let mut m = base("error", "mesh_route_explain_error", "fix_policy_payload");
        m.insert(
            "connect_retry_budget_exhausted".into(),
            Value::String("unknown".into()),
        );
        m.insert(
            "auto_recovery_final_result".into(),
            Value::String("not_triggered".into()),
        );
        let err = match validate_obj(&m) {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("error envelope final result mismatch"));
    }

    #[test]
    fn validate_rejects_ok_without_explain() {
        let mut m = base("ok", "mesh_route_explain", "use_selected_path");
        m.insert(
            "connect_retry_budget_exhausted".into(),
            Value::String("false".into()),
        );
        m.insert(
            "auto_recovery_final_result".into(),
            Value::String("not_triggered".into()),
        );
        let err = match validate_obj(&m) {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("explain missing"));
    }

    #[test]
    fn validate_rejects_kind_status_mismatch() {
        let mut m = base("ok", "mesh_route_explain_error", "use_selected_path");
        m.insert(
            "connect_retry_budget_exhausted".into(),
            Value::String("false".into()),
        );
        m.insert(
            "auto_recovery_final_result".into(),
            Value::String("not_triggered".into()),
        );
        m.insert(
            "explain".into(),
            Value::String("auto_recovery_attempts=0 | auto_recovery_final_result=not_triggered | auto_recovery_trace_consistent=true".into()),
        );
        let err = match validate_obj(&m) {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("kind does not match status"));
    }
}
