#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let reality_json = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/REALITY_AUDIT_LATEST.json");
    let data = read_obj(reality_json);

    if let Err(msg) = validate_reality_audit(&data) {
        fail(&msg);
    }

    println!("reality audit schema guard: PASS");
}

fn validate_reality_audit(data: &serde_json::Map<String, Value>) -> Result<(), String> {
    let required_keys: BTreeSet<&str> = [
        "status",
        "kind",
        "message_en",
        "message_ru",
        "real_world_datapath_closed",
        "source_markdown",
        "source_status",
        "md_claim_closed",
        "md_claim_partial_not_closed",
        "runtime_probe_file_ok",
        "runtime_probe_direct_ok",
        "runtime_probe_proxy_ok",
        "runtime_probe_proxy_listener_detected",
        "runtime_probe_proxy_probe_attempted",
        "runtime_probe_proxy_selected_from_candidates",
        "runtime_probe_proxy_error",
        "runtime_probe_proxy_candidates",
        "runtime_probe_skipped_no_proxy_listener",
        "runtime_probe_blocked_targets_total",
        "runtime_probe_blocked_targets_ok",
        "runtime_probe_blocked_targets_failed",
        "runtime_route_file_ok",
        "runtime_route_apply_ok",
        "runtime_route_rollback_ok",
        "runtime_forced_file_ok",
        "runtime_forced_recover_ok",
        "runtime_forced_clean_ok",
        "runtime_evidence_closed",
        "network_state",
        "generated_at",
    ]
    .into_iter()
    .collect();
    let keys: BTreeSet<&str> = data.keys().map(String::as_str).collect();
    if keys != required_keys {
        return Err("reality audit keys mismatch".to_string());
    }

    require_str(data, "status", "ok")?;
    require_str(data, "kind", "reality_audit")?;
    require_str(data, "network_state", "not_modified")?;
    let source_status = get_str(data, "source_status");
    if source_status != "parsed" && source_status != "not_found" {
        return Err("reality audit source_status invalid".to_string());
    }

    let bool_keys = [
        "real_world_datapath_closed",
        "md_claim_closed",
        "md_claim_partial_not_closed",
        "runtime_probe_file_ok",
        "runtime_probe_direct_ok",
        "runtime_probe_proxy_ok",
        "runtime_probe_proxy_listener_detected",
        "runtime_probe_proxy_probe_attempted",
        "runtime_probe_proxy_selected_from_candidates",
        "runtime_probe_skipped_no_proxy_listener",
        "runtime_route_file_ok",
        "runtime_route_apply_ok",
        "runtime_route_rollback_ok",
        "runtime_forced_file_ok",
        "runtime_forced_recover_ok",
        "runtime_forced_clean_ok",
        "runtime_evidence_closed",
    ];
    for k in bool_keys {
        if data.get(k).and_then(Value::as_bool).is_none() {
            return Err("reality audit field not bool".to_string());
        }
    }
    let total = get_i64(data, "runtime_probe_blocked_targets_total");
    let ok = get_i64(data, "runtime_probe_blocked_targets_ok");
    let failed = get_i64(data, "runtime_probe_blocked_targets_failed");
    if total < 0 || ok < 0 || failed < 0 {
        return Err("reality audit field not non-negative int".to_string());
    }
    if ok > total {
        return Err("runtime_probe_blocked_targets_ok exceeds total".to_string());
    }
    if failed > total {
        return Err("runtime_probe_blocked_targets_failed exceeds total".to_string());
    }
    if ok + failed != total {
        return Err("runtime_probe_blocked_targets totals mismatch".to_string());
    }
    let proxy_error = get_str(data, "runtime_probe_proxy_error");
    if ![
        "none",
        "proxy_listener_not_found",
        "proxy_connect_or_upstream_failed",
        "unknown",
    ]
    .contains(&proxy_error)
    {
        return Err("runtime_probe_proxy_error invalid".to_string());
    }
    if get_str(data, "runtime_probe_proxy_candidates")
        .trim()
        .is_empty()
    {
        return Err("runtime_probe_proxy_candidates is empty".to_string());
    }
    let proxy_attempted = get_bool(data, "runtime_probe_proxy_probe_attempted");
    let proxy_listener = get_bool(data, "runtime_probe_proxy_listener_detected");
    let proxy_ok = get_bool(data, "runtime_probe_proxy_ok");
    let proxy_selected = get_bool(data, "runtime_probe_proxy_selected_from_candidates");
    let skipped_no_proxy_listener = get_bool(data, "runtime_probe_skipped_no_proxy_listener");
    if proxy_attempted && !proxy_listener {
        return Err("runtime_probe proxy attempted without listener".to_string());
    }
    if proxy_listener && skipped_no_proxy_listener {
        return Err(
            "runtime_probe listener detected but skipped_no_proxy_listener=true".to_string(),
        );
    }
    if !proxy_attempted && !skipped_no_proxy_listener {
        return Err(
            "runtime_probe not attempted must set skipped_no_proxy_listener=true".to_string(),
        );
    }
    if proxy_attempted && skipped_no_proxy_listener {
        return Err("runtime_probe attempted must set skipped_no_proxy_listener=false".to_string());
    }
    if !proxy_attempted && proxy_error != "proxy_listener_not_found" {
        return Err("runtime_probe proxy not attempted must be listener_not_found".to_string());
    }
    if proxy_attempted && proxy_error == "proxy_listener_not_found" {
        return Err("runtime_probe proxy attempted with listener_not_found".to_string());
    }
    if proxy_ok && ok != total {
        return Err("runtime_probe proxy ok requires ok==total".to_string());
    }
    if proxy_selected && !proxy_attempted {
        return Err("runtime_probe selected_from_candidates requires attempted".to_string());
    }

    let runtime_probe_path_ok = get_bool(data, "runtime_probe_direct_ok")
        && (get_bool(data, "runtime_probe_proxy_ok")
            || get_bool(data, "runtime_probe_skipped_no_proxy_listener"))
        && (!get_bool(data, "runtime_probe_proxy_probe_attempted") || total == 0 || ok >= 1);

    let runtime_evidence_expected = runtime_probe_path_ok
        && get_bool(data, "runtime_route_apply_ok")
        && get_bool(data, "runtime_route_rollback_ok")
        && get_bool(data, "runtime_forced_recover_ok")
        && get_bool(data, "runtime_forced_clean_ok");
    if get_bool(data, "runtime_evidence_closed") != runtime_evidence_expected {
        return Err("runtime_evidence_closed mismatch".to_string());
    }

    let real_world_expected =
        get_bool(data, "md_claim_closed") && get_bool(data, "runtime_evidence_closed");
    if get_bool(data, "real_world_datapath_closed") != real_world_expected {
        return Err("real_world_datapath_closed mismatch".to_string());
    }

    if source_status == "parsed" && !get_str(data, "source_markdown").ends_with(".md") {
        return Err("source_markdown must point to .md when parsed".to_string());
    }
    if !get_str(data, "generated_at").ends_with('Z') {
        return Err("generated_at must be UTC Z string".to_string());
    }
    Ok(())
}

fn read_obj(path: &str) -> serde_json::Map<String, Value> {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("reality audit schema guard: missing file: {path}")));
    let v: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail(&format!("reality audit schema guard: invalid json: {path}")));
    v.as_object()
        .cloned()
        .unwrap_or_else(|| fail("reality audit schema guard: root not object"))
}
fn get_bool(obj: &serde_json::Map<String, Value>, key: &str) -> bool {
    obj.get(key).and_then(Value::as_bool).unwrap_or(false)
}
fn get_i64(obj: &serde_json::Map<String, Value>, key: &str) -> i64 {
    obj.get(key).and_then(Value::as_i64).unwrap_or(-1)
}
fn get_str<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> &'a str {
    obj.get(key).and_then(Value::as_str).unwrap_or("")
}
fn require_str(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    expected: &str,
) -> Result<(), String> {
    if get_str(obj, key) != expected {
        return Err("reality audit field mismatch".to_string());
    }
    Ok(())
}
fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::validate_reality_audit;
    use serde_json::{Map, Value, json};

    fn base() -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("status".to_string(), json!("ok"));
        m.insert("kind".to_string(), json!("reality_audit"));
        m.insert("message_en".to_string(), json!("ok"));
        m.insert("message_ru".to_string(), json!("ok"));
        m.insert("real_world_datapath_closed".to_string(), json!(false));
        m.insert(
            "source_markdown".to_string(),
            json!("docs/REALITY_AUDIT_2026-05-18.md"),
        );
        m.insert("source_status".to_string(), json!("parsed"));
        m.insert("md_claim_closed".to_string(), json!(false));
        m.insert("md_claim_partial_not_closed".to_string(), json!(true));
        m.insert("runtime_probe_file_ok".to_string(), json!(true));
        m.insert("runtime_probe_direct_ok".to_string(), json!(true));
        m.insert("runtime_probe_proxy_ok".to_string(), json!(false));
        m.insert(
            "runtime_probe_proxy_listener_detected".to_string(),
            json!(false),
        );
        m.insert(
            "runtime_probe_proxy_probe_attempted".to_string(),
            json!(false),
        );
        m.insert(
            "runtime_probe_proxy_selected_from_candidates".to_string(),
            json!(false),
        );
        m.insert(
            "runtime_probe_proxy_error".to_string(),
            json!("proxy_listener_not_found"),
        );
        m.insert(
            "runtime_probe_proxy_candidates".to_string(),
            json!("socks5h://127.0.0.1:11080,http://127.0.0.1:1080"),
        );
        m.insert(
            "runtime_probe_skipped_no_proxy_listener".to_string(),
            json!(true),
        );
        m.insert("runtime_probe_blocked_targets_total".to_string(), json!(0));
        m.insert("runtime_probe_blocked_targets_ok".to_string(), json!(0));
        m.insert("runtime_probe_blocked_targets_failed".to_string(), json!(0));
        m.insert("runtime_route_file_ok".to_string(), json!(true));
        m.insert("runtime_route_apply_ok".to_string(), json!(true));
        m.insert("runtime_route_rollback_ok".to_string(), json!(true));
        m.insert("runtime_forced_file_ok".to_string(), json!(true));
        m.insert("runtime_forced_recover_ok".to_string(), json!(true));
        m.insert("runtime_forced_clean_ok".to_string(), json!(true));
        m.insert("runtime_evidence_closed".to_string(), json!(true));
        m.insert("network_state".to_string(), json!("not_modified"));
        m.insert("generated_at".to_string(), json!("2026-05-19T12:00:00Z"));
        m
    }

    #[test]
    fn accepts_valid_payload() {
        let payload = base();
        assert!(validate_reality_audit(&payload).is_ok());
    }

    #[test]
    fn rejects_attempted_without_listener() {
        let mut payload = base();
        payload.insert(
            "runtime_probe_proxy_probe_attempted".to_string(),
            json!(true),
        );
        payload.insert(
            "runtime_probe_proxy_listener_detected".to_string(),
            json!(false),
        );
        payload.insert("runtime_probe_blocked_targets_total".to_string(), json!(1));
        payload.insert("runtime_probe_blocked_targets_ok".to_string(), json!(1));
        payload.insert("runtime_probe_proxy_ok".to_string(), json!(true));
        payload.insert("runtime_probe_proxy_error".to_string(), json!("none"));
        payload.insert(
            "runtime_probe_skipped_no_proxy_listener".to_string(),
            json!(false),
        );
        let res = validate_reality_audit(&payload);
        assert!(res.is_err());
        assert!(
            res.err()
                .is_some_and(|e| e.contains("attempted without listener"))
        );
    }

    #[test]
    fn rejects_bad_proxy_error() {
        let mut payload = base();
        payload.insert("runtime_probe_proxy_error".to_string(), json!("bad"));
        let res = validate_reality_audit(&payload);
        assert!(res.is_err());
        assert!(res.err().is_some_and(|e| e.contains("proxy_error invalid")));
    }
}
