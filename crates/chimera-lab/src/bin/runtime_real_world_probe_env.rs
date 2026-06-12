#![forbid(unsafe_code)]

use std::env;
use std::fs;

use serde_json::Value;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: runtime_real_world_probe_env <json_path>");
        std::process::exit(2);
    }

    let json_path = &args[1];
    let parsed = match read_json(json_path) {
        Some(value) => value,
        None => Value::Null,
    };

    for line in render_exports(&parsed) {
        println!("{line}");
    }
}

fn read_json(path: &str) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn emit_bool(
    out: &mut Vec<String>,
    parsed: &Value,
    source_key: &str,
    shell_key: &str,
    default_value: bool,
) {
    let value = parsed
        .get(source_key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default_value);
    out.push(format!(
        "{shell_key}={}",
        if value { "true" } else { "false" }
    ));
}

fn emit_i64(
    out: &mut Vec<String>,
    parsed: &Value,
    source_key: &str,
    shell_key: &str,
    default_value: i64,
) {
    let value = parsed
        .get(source_key)
        .and_then(|v| v.as_i64())
        .unwrap_or(default_value);
    out.push(format!("{shell_key}={value}"));
}

fn emit_string(
    out: &mut Vec<String>,
    parsed: &Value,
    source_key: &str,
    shell_key: &str,
    default_value: &str,
) {
    let raw = parsed
        .get(source_key)
        .and_then(|v| v.as_str())
        .unwrap_or(default_value);
    let normalized = if source_key == "datapath_probe_error" {
        normalize_datapath_probe_error(raw)
    } else {
        raw
    };
    let escaped = normalized.replace('\'', "'\"'\"'");
    out.push(format!("{shell_key}='{escaped}'"));
}

fn normalize_datapath_probe_error(value: &str) -> &str {
    match value {
        "none" | "curl_not_found" | "datapath_target_failed" | "unknown" => value,
        _ => "unknown",
    }
}

fn render_exports(parsed: &Value) -> Vec<String> {
    let mut out = Vec::with_capacity(8);
    let totals = normalize_datapath_target_totals(
        parsed
            .get("datapath_targets_total")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        parsed
            .get("datapath_targets_ok")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        parsed
            .get("datapath_targets_failed")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
    );
    emit_bool(
        &mut out,
        parsed,
        "direct_probe_ok",
        "runtime_real_world_direct_probe_ok",
        false,
    );
    emit_bool(
        &mut out,
        parsed,
        "datapath_probe_attempted",
        "runtime_real_world_datapath_probe_attempted",
        false,
    );
    emit_bool(
        &mut out,
        parsed,
        "datapath_probe_ok",
        "runtime_real_world_datapath_probe_ok",
        false,
    );
    emit_string(
        &mut out,
        parsed,
        "datapath_probe_error",
        "runtime_real_world_datapath_probe_error",
        "unknown",
    );
    emit_bool(
        &mut out,
        parsed,
        "skipped_no_curl",
        "runtime_real_world_skipped_no_curl",
        false,
    );
    emit_i64(
        &mut out,
        &serde_json::json!({ "v": totals.0 }),
        "v",
        "runtime_real_world_datapath_targets_total",
        0,
    );
    emit_i64(
        &mut out,
        &serde_json::json!({ "v": totals.1 }),
        "v",
        "runtime_real_world_datapath_targets_ok",
        0,
    );
    emit_i64(
        &mut out,
        &serde_json::json!({ "v": totals.2 }),
        "v",
        "runtime_real_world_datapath_targets_failed",
        0,
    );
    out
}

fn normalize_datapath_target_totals(
    total_raw: i64,
    ok_raw: i64,
    failed_raw: i64,
) -> (i64, i64, i64) {
    let mut total = total_raw.max(0);
    let mut ok = ok_raw.max(0);
    let mut failed = failed_raw.max(0);
    if ok > total {
        ok = total;
    }
    if failed > total {
        failed = total;
    }
    if ok + failed > total {
        failed = total - ok;
    }
    if failed < 0 {
        failed = 0;
        if ok > total {
            ok = total;
        }
    }
    if ok + failed > total {
        total = ok + failed;
    }
    (total, ok, failed)
}

#[cfg(test)]
mod tests {
    use super::{normalize_datapath_probe_error, normalize_datapath_target_totals, render_exports};
    use serde_json::json;

    #[test]
    fn defaults_when_fields_missing() {
        let got = render_exports(&json!({}));
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_direct_probe_ok=false")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_datapath_probe_error='unknown'")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_datapath_targets_total=0")
        );
    }

    #[test]
    fn maps_present_fields() {
        let got = render_exports(&json!({
            "direct_probe_ok": true,
            "datapath_probe_attempted": true,
            "datapath_probe_ok": false,
            "datapath_probe_error": "datapath_target_failed",
            "skipped_no_curl": false,
            "datapath_targets_total": 3,
            "datapath_targets_ok": 2,
            "datapath_targets_failed": 1
        }));
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_direct_probe_ok=true")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_datapath_probe_attempted=true")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_datapath_probe_ok=false")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_datapath_probe_error='datapath_target_failed'")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_datapath_targets_total=3")
        );
    }

    #[test]
    fn escapes_single_quotes_for_shell_eval() {
        let got = render_exports(&json!({
            "datapath_probe_error": "bad'quote"
        }));
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_datapath_probe_error='unknown'")
        );
    }

    #[test]
    fn normalize_datapath_probe_error_allows_only_known_values() {
        assert_eq!(normalize_datapath_probe_error("none"), "none");
        assert_eq!(
            normalize_datapath_probe_error("curl_not_found"),
            "curl_not_found"
        );
        assert_eq!(
            normalize_datapath_probe_error("datapath_target_failed"),
            "datapath_target_failed"
        );
        assert_eq!(normalize_datapath_probe_error("unknown"), "unknown");
        assert_eq!(normalize_datapath_probe_error("something_else"), "unknown");
    }

    #[test]
    fn normalize_datapath_target_totals_clamps_negative_and_overflow() {
        assert_eq!(normalize_datapath_target_totals(-1, -2, -3), (0, 0, 0));
        assert_eq!(normalize_datapath_target_totals(2, 7, 1), (2, 2, 0));
        assert_eq!(normalize_datapath_target_totals(3, 2, 9), (3, 2, 1));
    }
}
