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

    let smoke_ok = kind_ok
        && status_ok
        && network_ok
        && has_targets
        && all_total > 0
        && failed_total >= 0
        && fail_threshold >= 0
        && !threshold_exceeded
        && failed_total <= fail_threshold;

    vec![
        format!(
            "runtime_probe_access_smoke_ok={}",
            if smoke_ok { "true" } else { "false" }
        ),
        format!("runtime_probe_access_failed_total={failed_total}"),
        format!("runtime_probe_access_fail_threshold={fail_threshold}"),
        format!(
            "runtime_probe_access_threshold_exceeded={}",
            if threshold_exceeded { "true" } else { "false" }
        ),
    ]
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
            "network_state":"not_modified",
            "totals":{"all":2,"failed_total":0,"fail_threshold":1,"threshold_exceeded":false},
            "targets":[{"url":"https://example.org"}]
        }));
        assert!(
            got.iter()
                .any(|l| l == "runtime_probe_access_smoke_ok=true")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_probe_access_failed_total=0")
        );
    }

    #[test]
    fn exports_false_when_threshold_exceeded() {
        let got = render_exports(&json!({
            "status":"ok",
            "kind":"probe_access",
            "network_state":"not_modified",
            "totals":{"all":2,"failed_total":2,"fail_threshold":0,"threshold_exceeded":true},
            "targets":[{"url":"https://example.org"}]
        }));
        assert!(
            got.iter()
                .any(|l| l == "runtime_probe_access_smoke_ok=false")
        );
    }
}
