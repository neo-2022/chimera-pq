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
    let normalized = if source_key == "proxy_probe_error" {
        normalize_proxy_probe_error(raw)
    } else {
        raw
    };
    let escaped = normalized.replace('\'', "'\"'\"'");
    out.push(format!("{shell_key}='{escaped}'"));
}

fn normalize_proxy_probe_error(value: &str) -> &str {
    match value {
        "none" | "proxy_listener_not_found" | "proxy_connect_or_upstream_failed" | "unknown" => {
            value
        }
        _ => "unknown",
    }
}

fn render_exports(parsed: &Value) -> Vec<String> {
    let mut out = Vec::with_capacity(12);
    let totals = normalize_blocked_target_totals(
        parsed
            .get("proxy_blocked_targets_total")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        parsed
            .get("proxy_blocked_targets_ok")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        parsed
            .get("proxy_blocked_targets_failed")
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
        "proxy_listener_detected",
        "runtime_real_world_proxy_listener_detected",
        false,
    );
    emit_bool(
        &mut out,
        parsed,
        "proxy_probe_attempted",
        "runtime_real_world_proxy_probe_attempted",
        false,
    );
    emit_bool(
        &mut out,
        parsed,
        "proxy_probe_ok",
        "runtime_real_world_proxy_probe_ok",
        false,
    );
    emit_bool(
        &mut out,
        parsed,
        "proxy_selected_from_candidates",
        "runtime_real_world_proxy_selected_from_candidates",
        false,
    );
    emit_string(
        &mut out,
        parsed,
        "proxy_probe_error",
        "runtime_real_world_proxy_probe_error",
        "unknown",
    );
    emit_string(
        &mut out,
        parsed,
        "proxy_candidates",
        "runtime_real_world_proxy_candidates",
        "",
    );
    emit_bool(
        &mut out,
        parsed,
        "skipped_no_curl",
        "runtime_real_world_skipped_no_curl",
        false,
    );
    emit_bool(
        &mut out,
        parsed,
        "skipped_no_proxy_listener",
        "runtime_real_world_skipped_no_proxy_listener",
        false,
    );
    emit_i64(
        &mut out,
        &serde_json::json!({ "v": totals.0 }),
        "v",
        "runtime_real_world_proxy_blocked_targets_total",
        0,
    );
    emit_i64(
        &mut out,
        &serde_json::json!({ "v": totals.1 }),
        "v",
        "runtime_real_world_proxy_blocked_targets_ok",
        0,
    );
    emit_i64(
        &mut out,
        &serde_json::json!({ "v": totals.2 }),
        "v",
        "runtime_real_world_proxy_blocked_targets_failed",
        0,
    );
    out
}

fn normalize_blocked_target_totals(
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
    use super::{normalize_blocked_target_totals, normalize_proxy_probe_error, render_exports};
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
                .any(|l| l == "runtime_real_world_proxy_probe_error='unknown'")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_proxy_blocked_targets_total=0")
        );
    }

    #[test]
    fn maps_present_fields() {
        let got = render_exports(&json!({
            "direct_probe_ok": true,
            "proxy_listener_detected": true,
            "proxy_probe_attempted": true,
            "proxy_probe_ok": false,
            "proxy_selected_from_candidates": true,
            "proxy_candidates": "socks5h://127.0.0.1:11080,http://127.0.0.1:1080",
            "proxy_probe_error": "none",
            "skipped_no_curl": false,
            "skipped_no_proxy_listener": false,
            "proxy_blocked_targets_total": 3,
            "proxy_blocked_targets_ok": 2,
            "proxy_blocked_targets_failed": 1
        }));
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_direct_probe_ok=true")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_proxy_listener_detected=true")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_proxy_probe_attempted=true")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_proxy_probe_ok=false")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_proxy_selected_from_candidates=true")
        );
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_proxy_probe_error='none'")
        );
        assert!(got
            .iter()
            .any(|l| l == "runtime_real_world_proxy_candidates='socks5h://127.0.0.1:11080,http://127.0.0.1:1080'"));
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_proxy_blocked_targets_total=3")
        );
    }

    #[test]
    fn escapes_single_quotes_for_shell_eval() {
        let got = render_exports(&json!({
            "proxy_probe_error": "bad'quote"
        }));
        assert!(
            got.iter()
                .any(|l| l == "runtime_real_world_proxy_probe_error='unknown'")
        );
    }

    #[test]
    fn normalize_proxy_probe_error_allows_only_known_values() {
        assert_eq!(normalize_proxy_probe_error("none"), "none");
        assert_eq!(
            normalize_proxy_probe_error("proxy_listener_not_found"),
            "proxy_listener_not_found"
        );
        assert_eq!(
            normalize_proxy_probe_error("proxy_connect_or_upstream_failed"),
            "proxy_connect_or_upstream_failed"
        );
        assert_eq!(normalize_proxy_probe_error("unknown"), "unknown");
        assert_eq!(normalize_proxy_probe_error("something_else"), "unknown");
    }

    #[test]
    fn normalize_blocked_target_totals_clamps_negative_and_overflow() {
        assert_eq!(normalize_blocked_target_totals(-1, -2, -3), (0, 0, 0));
        assert_eq!(normalize_blocked_target_totals(2, 7, 1), (2, 2, 0));
        assert_eq!(normalize_blocked_target_totals(3, 2, 9), (3, 2, 1));
    }
}
