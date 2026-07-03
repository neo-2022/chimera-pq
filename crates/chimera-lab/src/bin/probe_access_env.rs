#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: probe_access_env <json_path>");
        std::process::exit(2);
    }
    let parsed = read_json(&args[1]).unwrap_or(Value::Null);
    for line in render_exports(&parsed) {
        println!("{line}");
    }
}

fn read_json(path: &str) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn render_exports(parsed: &Value) -> Vec<String> {
    let kind_ok = parsed
        .get("kind")
        .and_then(Value::as_str)
        .map(|v| v == "probe_access")
        .unwrap_or(false);
    let status_ok = parsed
        .get("status")
        .and_then(Value::as_str)
        .map(|v| v == "ok")
        .unwrap_or(false);
    let network_ok = parsed
        .get("network_state")
        .and_then(Value::as_str)
        .map(|v| v == "not_modified")
        .unwrap_or(false);
    let has_targets = parsed
        .get("targets")
        .and_then(Value::as_array)
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    let totals = parsed.get("totals").and_then(Value::as_object);
    let failed_total = totals
        .and_then(|o| o.get("failed_total"))
        .and_then(Value::as_i64)
        .unwrap_or(1);
    let fail_threshold = totals
        .and_then(|o| o.get("fail_threshold"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let threshold_exceeded = totals
        .and_then(|o| o.get("threshold_exceeded"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let all_total = totals
        .and_then(|o| o.get("all"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let mode = resolve_probe_mode(parsed);
    let ci_snapshot = mode == "ci_snapshot";
    let ci_snapshot_targets_ok = ci_snapshot && redaction_marker_ok(parsed);

    let smoke_ok = kind_ok
        && status_ok
        && network_ok
        && redaction_marker_ok(parsed)
        && has_targets
        && all_total > 0
        && failed_total >= 0
        && fail_threshold >= 0
        && !threshold_exceeded
        && failed_total <= fail_threshold
        && (!ci_snapshot || ci_snapshot_targets_ok);

    vec![
        format!(
            "runtime_probe_access_smoke_ok={}",
            if smoke_ok { "true" } else { "false" }
        ),
        format!("runtime_probe_access_mode='{mode}'"),
        format!(
            "runtime_probe_access_live_external_probe={}",
            if ci_snapshot { "false" } else { "true" }
        ),
        format!(
            "runtime_probe_access_ssh_stand_required_for_live_probe={}",
            if ci_snapshot { "true" } else { "false" }
        ),
        format!(
            "runtime_probe_access_ci_snapshot_targets_ok={}",
            if ci_snapshot_targets_ok {
                "true"
            } else {
                "false"
            }
        ),
        format!("runtime_probe_access_failed_total={failed_total}"),
        format!("runtime_probe_access_fail_threshold={fail_threshold}"),
        format!(
            "runtime_probe_access_threshold_exceeded={}",
            if threshold_exceeded { "true" } else { "false" }
        ),
    ]
}

fn resolve_probe_mode(parsed: &Value) -> &'static str {
    match parsed.get("target_profile").and_then(Value::as_str) {
        Some("ci_snapshot") => "ci_snapshot",
        _ => "live",
    }
}

fn redaction_marker_ok(parsed: &Value) -> bool {
    parsed
        .get("redaction")
        .and_then(Value::as_str)
        .is_some_and(|value| value == "raw_targets_redacted")
}

#[cfg(test)]
mod tests {
    use super::render_exports;
    use serde_json::json;

    #[test]
    fn exports_ok_when_contract_is_green() {
        let got = render_exports(&json!({
            "status":"ok",
            "kind":"probe_access",
            "redaction":"raw_targets_redacted",
            "target_profile":"live",
            "network_state":"not_modified",
            "totals":{"all":2,"failed_total":0,"fail_threshold":1,"threshold_exceeded":false},
            "targets":[{"url":"target#1"}]
        }));
        assert!(
            got.iter()
                .any(|l| l == "runtime_probe_access_smoke_ok=true")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_probe_access_failed_total=0")
        );
        assert!(got.iter().any(|l| l == "runtime_probe_access_mode='live'"));
        assert!(
            got.iter()
                .any(|l| l == "runtime_probe_access_live_external_probe=true")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_probe_access_ci_snapshot_targets_ok=false")
        );
    }

    #[test]
    fn exports_ci_snapshot_metadata_when_requested() {
        let got = render_exports(&json!({
            "status":"ok",
            "kind":"probe_access",
            "redaction":"raw_targets_redacted",
            "target_profile":"ci_snapshot",
            "network_state":"not_modified",
            "totals":{"all":2,"failed_total":0,"fail_threshold":0,"threshold_exceeded":false},
            "targets":[
                {"url":"target#1"},
                {"url":"target#2"}
            ]
        }));
        assert!(
            got.iter()
                .any(|l| l == "runtime_probe_access_smoke_ok=true")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_probe_access_mode='ci_snapshot'")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_probe_access_live_external_probe=false")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_probe_access_ssh_stand_required_for_live_probe=true")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_probe_access_ci_snapshot_targets_ok=true")
        );
    }

    #[test]
    fn exports_false_when_threshold_exceeded() {
        let got = render_exports(&json!({
            "status":"ok",
            "kind":"probe_access",
            "redaction":"raw_targets_redacted",
            "target_profile":"live",
            "network_state":"not_modified",
            "totals":{"all":2,"failed_total":2,"fail_threshold":0,"threshold_exceeded":true},
            "targets":[{"url":"target#1"}]
        }));
        assert!(
            got.iter()
                .any(|l| l == "runtime_probe_access_smoke_ok=false")
        );
    }
}
