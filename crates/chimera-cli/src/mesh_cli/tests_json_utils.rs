use super::route_explain_integrity::build_route_explain_contract_integrity;
use super::route_explain_meta::{
    ROUTE_EXPLAIN_RECOVERY_FIELDS_CHECKSUM, ROUTE_EXPLAIN_RECOVERY_SCHEMA_VERSION,
};
use super::tests_contract_constants::{
    BOOL_FALSE, BOOL_TRUE, INTEGRITY_ALL_TRUE, SUCCESS_OPERATOR_ACTION, SUCCESS_OPERATOR_REASON,
    SUCCESS_PRESSURE, SUCCESS_SELECTED_NODE, UNKNOWN_VALUE,
};
pub(crate) use super::tests_json_utils_contract::{
    expected_contract_family, expected_contract_version, expected_kind_error, expected_kind_ok,
    expected_network_state, expected_status_error, expected_status_ok,
};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn temp_out_file(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| unreachable!("time should be monotonic"))
        .as_nanos();
    path.push(format!("chimera_mesh_cli_{name}_{ts}.json"));
    path
}

pub(crate) fn with_json_out_args(mut args: Vec<String>, out: &Path) -> Vec<String> {
    args.extend([
        "--json".to_string(),
        "--out".to_string(),
        out.to_string_lossy().to_string(),
    ]);
    args
}

pub(crate) fn expected_recovery_schema_version() -> &'static str {
    ROUTE_EXPLAIN_RECOVERY_SCHEMA_VERSION
}

pub(crate) fn expected_recovery_fields_checksum() -> &'static str {
    ROUTE_EXPLAIN_RECOVERY_FIELDS_CHECKSUM
}

pub(crate) fn base_route_explain_args(policy_payload: &str) -> Vec<String> {
    vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        policy_payload.to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
    ]
}

pub(crate) fn json_value<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let marker = format!("\"{key}\":\"");
    let start = json.find(&marker)? + marker.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

pub(crate) fn summary_field<'a>(summary: &'a str, key: &str) -> Option<&'a str> {
    for part in summary.split(';') {
        let (k, v) = part.split_once(':')?;
        if k == key {
            return Some(v);
        }
    }
    None
}

pub(crate) fn assert_route_explain_envelope(
    parsed: &serde_json::Value,
    expected_status: &str,
    expected_kind: &str,
) {
    assert_eq!(parsed["status"].as_str().unwrap_or(""), expected_status);
    assert_eq!(parsed["kind"].as_str().unwrap_or(""), expected_kind);
    assert_eq!(
        parsed["route_explain_contract_family"]
            .as_str()
            .unwrap_or(""),
        expected_contract_family()
    );
    assert_eq!(
        parsed["explain_contract_version"].as_str().unwrap_or(""),
        expected_contract_version()
    );
    assert_eq!(
        parsed["network_state"].as_str().unwrap_or(""),
        expected_network_state()
    );
}

pub(crate) fn assert_route_explain_envelope_presence(parsed: &serde_json::Value) {
    for key in [
        "status",
        "kind",
        "route_explain_contract_family",
        "explain_contract_version",
        "namespace",
        "node",
        "network_state",
    ] {
        assert!(parsed.get(key).is_some(), "missing key: {key}");
        assert!(!parsed[key].as_str().unwrap_or_default().is_empty());
    }
}

pub(crate) fn assert_route_explain_contract_blocks_presence(parsed: &serde_json::Value) {
    assert_route_explain_envelope_presence(parsed);
    assert_operator_block_presence(parsed);
    assert_operator_route_alignment(parsed);
    for key in [
        "route_explain_health_gate",
        "route_explain_health_summary",
        "route_explain_contract_integrity",
    ] {
        assert!(parsed.get(key).is_some(), "missing key: {key}");
        assert!(!parsed[key].as_str().unwrap_or_default().is_empty());
    }
}

pub(crate) fn expected_integrity_all_true() -> &'static str {
    let expected = INTEGRITY_ALL_TRUE;
    let operator_summary = expected_operator_summary(
        expected_status_ok(),
        SUCCESS_SELECTED_NODE,
        SUCCESS_PRESSURE,
        SUCCESS_OPERATOR_ACTION,
        SUCCESS_OPERATOR_REASON,
    );
    let operator_route_key = expected_operator_route_key_ok(SUCCESS_OPERATOR_ACTION);
    let computed = build_route_explain_contract_integrity(
        &operator_summary,
        &operator_summary,
        &operator_route_key,
        expected_status_ok(),
        expected_status_ok(),
        SUCCESS_OPERATOR_ACTION,
    );
    assert_eq!(computed, expected);
    expected
}

pub(crate) fn expected_integrity_all_true_text_field() -> String {
    format!(
        "route_explain_contract_integrity={}",
        expected_integrity_all_true()
    )
}

pub(crate) fn expected_operator_summary(
    health: &str,
    selected: &str,
    pressure: &str,
    action: &str,
    reason: &str,
) -> String {
    format!(
        "health:{};selected:{};pressure:{};action:{};reason:{}",
        health, selected, pressure, action, reason
    )
}

pub(crate) fn expected_operator_route_key(health: &str, action: &str) -> String {
    format!("{health}:{action}")
}

pub(crate) fn expected_operator_route_key_ok(action: &str) -> String {
    expected_operator_route_key(expected_status_ok(), action)
}

pub(crate) fn expected_operator_route_key_error(action: &str) -> String {
    expected_operator_route_key(expected_status_error(), action)
}

pub(crate) fn expected_health_summary_ok() -> String {
    format!(
        "table:{};degraded:{};pressure_projection:{}",
        expected_status_ok(),
        BOOL_FALSE,
        expected_status_ok()
    )
}

pub(crate) fn expected_health_summary_error() -> String {
    format!(
        "table:{};degraded:{};pressure_projection:{}",
        expected_status_error(),
        UNKNOWN_VALUE,
        UNKNOWN_VALUE
    )
}

pub(crate) fn expected_table_runtime_consistency_summary(gate: &str, all_true: &str) -> String {
    format!("gate={gate};all_true={all_true}")
}

pub(crate) fn expected_preemptive_shadow_degraded_summary(
    path: &str,
    reason: &str,
    gate: &str,
    all_true: &str,
) -> String {
    format!("path={path};reason={reason};gate={gate};all_true={all_true}")
}

pub(crate) fn assert_operator_summary_invariants(parsed: &serde_json::Value) {
    assert_eq!(
        parsed["route_explain_contract_family"]
            .as_str()
            .unwrap_or(""),
        expected_contract_family()
    );
    assert_eq!(
        parsed["explain_contract_version"].as_str().unwrap_or(""),
        expected_contract_version()
    );
    let summary = parsed["route_explain_operator_summary"]
        .as_str()
        .unwrap_or("");
    let signature = parsed["route_explain_operator_signature"]
        .as_str()
        .unwrap_or("");
    assert_eq!(summary, signature);
    let action_from_summary = summary_field(summary, "action").unwrap_or("");
    let reason_from_summary = summary_field(summary, "reason").unwrap_or("");
    let health_from_summary = summary_field(summary, "health").unwrap_or("");
    let selected_from_summary = summary_field(summary, "selected").unwrap_or("");
    let pressure_from_summary = summary_field(summary, "pressure").unwrap_or("");
    assert_eq!(
        parsed["route_explain_operator_action"]
            .as_str()
            .unwrap_or(""),
        action_from_summary
    );
    assert_eq!(
        parsed["route_explain_operator_reason"]
            .as_str()
            .unwrap_or(""),
        reason_from_summary
    );
    assert_eq!(
        parsed["route_explain_operator_health"]
            .as_str()
            .unwrap_or(""),
        health_from_summary
    );
    assert_eq!(
        parsed["route_explain_operator_selected"]
            .as_str()
            .unwrap_or(""),
        selected_from_summary
    );
    assert_eq!(
        parsed["route_explain_operator_pressure"]
            .as_str()
            .unwrap_or(""),
        pressure_from_summary
    );
    assert_eq!(
        parsed["route_explain_operator_route_key"]
            .as_str()
            .unwrap_or(""),
        format!("{health_from_summary}:{action_from_summary}")
    );
    assert_eq!(
        parsed["route_explain_contract_integrity"]
            .as_str()
            .unwrap_or(""),
        format!(
            "signature_match:{};route_key_match:{};health_gate_match:{}",
            bool_text(summary == signature),
            bool_text(
                parsed["route_explain_operator_route_key"]
                    .as_str()
                    .unwrap_or("")
                    == format!("{health_from_summary}:{action_from_summary}")
            ),
            bool_text(
                parsed["route_explain_health_gate"].as_str().unwrap_or("") == health_from_summary
            )
        )
    );
    if parsed["status"] == expected_status_ok() {
        assert_eq!(
            parsed["route_explain_health_gate"].as_str().unwrap_or(""),
            expected_status_ok()
        );
        assert_eq!(health_from_summary, expected_status_ok());
    } else if parsed["status"] == expected_status_error() {
        assert_eq!(
            parsed["route_explain_health_gate"].as_str().unwrap_or(""),
            expected_status_error()
        );
        assert_eq!(health_from_summary, expected_status_error());
    }
}

pub(crate) fn assert_operator_block_presence(parsed: &serde_json::Value) {
    for key in [
        "route_explain_operator_summary",
        "route_explain_operator_signature",
        "route_explain_operator_route_key",
        "route_explain_operator_health",
        "route_explain_operator_selected",
        "route_explain_operator_pressure",
        "route_explain_operator_action",
        "route_explain_operator_reason",
        "route_explain_contract_integrity",
    ] {
        assert!(parsed.get(key).is_some(), "missing key: {key}");
        assert!(!parsed[key].as_str().unwrap_or_default().is_empty());
    }
    let summary = parsed["route_explain_operator_summary"]
        .as_str()
        .unwrap_or_default();
    for field in ["health", "selected", "pressure", "action", "reason"] {
        assert!(
            summary_field(summary, field).is_some(),
            "missing summary field: {field}"
        );
    }
}

pub(crate) fn assert_operator_route_alignment(parsed: &serde_json::Value) {
    let health = parsed["route_explain_operator_health"]
        .as_str()
        .unwrap_or("");
    let action = parsed["route_explain_operator_action"]
        .as_str()
        .unwrap_or("");
    let route_key = parsed["route_explain_operator_route_key"]
        .as_str()
        .unwrap_or("");
    let health_gate = parsed["route_explain_health_gate"].as_str().unwrap_or("");

    assert_eq!(route_key, format!("{health}:{action}"));
    assert_eq!(health_gate, health);
}

fn bool_text(value: bool) -> &'static str {
    if value { BOOL_TRUE } else { BOOL_FALSE }
}

pub(crate) fn assert_health_summary_shape(summary: &str) {
    assert!(summary.contains("table:"));
    assert!(summary.contains(";degraded:"));
    assert!(summary.contains(";pressure_projection:"));
}

pub(crate) fn assert_health_pressure_projection_consistency(parsed: &serde_json::Value) {
    let gate = parsed["route_explain_health_gate"].as_str().unwrap_or("");
    let summary = parsed["route_explain_health_summary"]
        .as_str()
        .unwrap_or("");
    assert_health_summary_shape(summary);
    let table = summary_field(summary, "table").unwrap_or("");
    let projection = summary_field(summary, "pressure_projection").unwrap_or("");

    match parsed["status"].as_str().unwrap_or("") {
        status if status == expected_status_ok() => {
            assert_eq!(gate, expected_status_ok());
            assert_eq!(table, expected_status_ok());
            assert_eq!(projection, expected_status_ok());
        }
        status if status == expected_status_error() => {
            assert_eq!(gate, expected_status_error());
            assert_eq!(table, expected_status_error());
            assert_eq!(projection, UNKNOWN_VALUE);
        }
        _ => {}
    }
}
