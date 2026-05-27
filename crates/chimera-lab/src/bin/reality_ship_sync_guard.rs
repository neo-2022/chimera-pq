#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let reality_json = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/REALITY_AUDIT_LATEST.json");
    let ship_json = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("docs/SHIP_READINESS_REPORT.json");
    let probe_json = args
        .get(3)
        .map(String::as_str)
        .unwrap_or("docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json");

    let reality = read_obj(reality_json);
    let ship = read_obj(ship_json);
    let probe = read_obj(probe_json);

    require_str(&reality, "status", "ok");
    require_str(&reality, "kind", "reality_audit");
    require_str(&ship, "status", "ok");
    require_str(&ship, "kind", "ship_readiness_report");
    require_str(&probe, "status", "ok");
    require_str(&probe, "kind", "runtime_real_world_probe_smoke");

    eq_bool_cross(
        &ship,
        "runtime_real_world_direct_probe_ok",
        &reality,
        "runtime_probe_direct_ok",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_proxy_listener_detected",
        &reality,
        "runtime_probe_proxy_listener_detected",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_proxy_probe_attempted",
        &reality,
        "runtime_probe_proxy_probe_attempted",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_proxy_probe_ok",
        &reality,
        "runtime_probe_proxy_ok",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_proxy_selected_from_candidates",
        &reality,
        "runtime_probe_proxy_selected_from_candidates",
    );
    eq_str_cross(
        &ship,
        "runtime_real_world_proxy_probe_error",
        &reality,
        "runtime_probe_proxy_error",
    );
    eq_str_cross(
        &ship,
        "runtime_real_world_proxy_candidates",
        &reality,
        "runtime_probe_proxy_candidates",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_skipped_no_proxy_listener",
        &reality,
        "runtime_probe_skipped_no_proxy_listener",
    );
    eq_i64_cross(
        &ship,
        "runtime_real_world_proxy_blocked_targets_total",
        &reality,
        "runtime_probe_blocked_targets_total",
    );
    eq_i64_cross(
        &ship,
        "runtime_real_world_proxy_blocked_targets_ok",
        &reality,
        "runtime_probe_blocked_targets_ok",
    );
    eq_i64_cross(
        &ship,
        "runtime_real_world_proxy_blocked_targets_failed",
        &reality,
        "runtime_probe_blocked_targets_failed",
    );

    eq_bool_cross(
        &reality,
        "runtime_probe_direct_ok",
        &probe,
        "direct_probe_ok",
    );
    eq_bool_cross(
        &reality,
        "runtime_probe_proxy_listener_detected",
        &probe,
        "proxy_listener_detected",
    );
    eq_bool_cross(
        &reality,
        "runtime_probe_proxy_probe_attempted",
        &probe,
        "proxy_probe_attempted",
    );
    eq_bool_cross(&reality, "runtime_probe_proxy_ok", &probe, "proxy_probe_ok");
    eq_bool_cross(
        &reality,
        "runtime_probe_proxy_selected_from_candidates",
        &probe,
        "proxy_selected_from_candidates",
    );
    eq_str_cross(
        &reality,
        "runtime_probe_proxy_error",
        &probe,
        "proxy_probe_error",
    );
    eq_str_cross(
        &reality,
        "runtime_probe_proxy_candidates",
        &probe,
        "proxy_candidates",
    );
    eq_bool_cross(
        &reality,
        "runtime_probe_skipped_no_proxy_listener",
        &probe,
        "skipped_no_proxy_listener",
    );
    eq_i64_cross(
        &reality,
        "runtime_probe_blocked_targets_total",
        &probe,
        "proxy_blocked_targets_total",
    );
    eq_i64_cross(
        &reality,
        "runtime_probe_blocked_targets_ok",
        &probe,
        "proxy_blocked_targets_ok",
    );
    eq_i64_cross(
        &reality,
        "runtime_probe_blocked_targets_failed",
        &probe,
        "proxy_blocked_targets_failed",
    );
    if let Err(msg) = validate_proxy_logic(&ship) {
        fail(&msg);
    }

    println!("reality ship sync guard: PASS");
}

fn read_obj(path: &str) -> serde_json::Map<String, Value> {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("reality ship sync guard: missing file: {path}")));
    let v: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail(&format!("reality ship sync guard: invalid json: {path}")));
    v.as_object()
        .cloned()
        .unwrap_or_else(|| fail("reality ship sync guard: root not object"))
}
fn get_bool(obj: &serde_json::Map<String, Value>, key: &str) -> bool {
    obj.get(key).and_then(Value::as_bool).unwrap_or(false)
}
fn get_i64(obj: &serde_json::Map<String, Value>, key: &str) -> i64 {
    obj.get(key).and_then(Value::as_i64).unwrap_or(0)
}
fn require_str(obj: &serde_json::Map<String, Value>, key: &str, expected: &str) {
    if obj.get(key).and_then(Value::as_str) != Some(expected) {
        fail("reality ship sync guard: envelope mismatch");
    }
}
fn eq_bool_cross(
    a: &serde_json::Map<String, Value>,
    ak: &str,
    b: &serde_json::Map<String, Value>,
    bk: &str,
) {
    if get_bool(a, ak) != get_bool(b, bk) {
        fail("reality ship sync guard: bool mismatch");
    }
}
fn eq_i64_cross(
    a: &serde_json::Map<String, Value>,
    ak: &str,
    b: &serde_json::Map<String, Value>,
    bk: &str,
) {
    if get_i64(a, ak) != get_i64(b, bk) {
        fail("reality ship sync guard: int mismatch");
    }
}
fn eq_str_cross(
    a: &serde_json::Map<String, Value>,
    ak: &str,
    b: &serde_json::Map<String, Value>,
    bk: &str,
) {
    if a.get(ak).and_then(Value::as_str).unwrap_or("")
        != b.get(bk).and_then(Value::as_str).unwrap_or("")
    {
        fail("reality ship sync guard: str mismatch");
    }
}

fn validate_proxy_logic(ship: &serde_json::Map<String, Value>) -> Result<(), String> {
    let proxy_attempted = get_bool(ship, "runtime_real_world_proxy_probe_attempted");
    let proxy_listener = get_bool(ship, "runtime_real_world_proxy_listener_detected");
    let proxy_ok = get_bool(ship, "runtime_real_world_proxy_probe_ok");
    let proxy_selected = get_bool(ship, "runtime_real_world_proxy_selected_from_candidates");
    let proxy_candidates = ship
        .get("runtime_real_world_proxy_candidates")
        .and_then(Value::as_str)
        .unwrap_or("");
    let proxy_error = ship
        .get("runtime_real_world_proxy_probe_error")
        .and_then(Value::as_str)
        .unwrap_or("");
    let total = get_i64(ship, "runtime_real_world_proxy_blocked_targets_total");
    let ok = get_i64(ship, "runtime_real_world_proxy_blocked_targets_ok");
    let failed = get_i64(ship, "runtime_real_world_proxy_blocked_targets_failed");
    if ![
        "none",
        "proxy_listener_not_found",
        "proxy_connect_or_upstream_failed",
        "unknown",
    ]
    .contains(&proxy_error)
    {
        return Err("reality ship sync guard: proxy error value is invalid".to_string());
    }
    if ok + failed != total {
        return Err("reality ship sync guard: proxy totals mismatch".to_string());
    }
    if proxy_attempted && !proxy_listener {
        return Err("reality ship sync guard: proxy attempted without listener".to_string());
    }
    if !proxy_attempted && proxy_error != "proxy_listener_not_found" {
        return Err(
            "reality ship sync guard: proxy not attempted must be listener_not_found".to_string(),
        );
    }
    if proxy_attempted && proxy_error == "proxy_listener_not_found" {
        return Err("reality ship sync guard: proxy attempted with listener_not_found".to_string());
    }
    if proxy_ok && failed != 0 {
        return Err("reality ship sync guard: proxy ok with failed targets".to_string());
    }
    if proxy_attempted && total <= 0 {
        return Err(
            "reality ship sync guard: proxy attempted with empty target totals".to_string(),
        );
    }
    if !proxy_attempted && total != 0 {
        return Err(
            "reality ship sync guard: proxy not attempted with non-zero totals".to_string(),
        );
    }
    if proxy_selected && !proxy_attempted {
        return Err(
            "reality ship sync guard: proxy selected_from_candidates requires attempted"
                .to_string(),
        );
    }
    if proxy_candidates.trim().is_empty() {
        return Err("reality ship sync guard: proxy candidates is empty".to_string());
    }
    Ok(())
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::validate_proxy_logic;
    use serde_json::{Map, Value, json};

    fn base_ship() -> Map<String, Value> {
        let mut m = Map::new();
        m.insert(
            "runtime_real_world_proxy_probe_attempted".to_string(),
            json!(true),
        );
        m.insert(
            "runtime_real_world_proxy_listener_detected".to_string(),
            json!(true),
        );
        m.insert("runtime_real_world_proxy_probe_ok".to_string(), json!(true));
        m.insert(
            "runtime_real_world_proxy_selected_from_candidates".to_string(),
            json!(false),
        );
        m.insert(
            "runtime_real_world_proxy_candidates".to_string(),
            json!("socks5h://127.0.0.1:11080,http://127.0.0.1:1080"),
        );
        m.insert(
            "runtime_real_world_proxy_probe_error".to_string(),
            json!("none"),
        );
        m.insert(
            "runtime_real_world_proxy_blocked_targets_total".to_string(),
            json!(1),
        );
        m.insert(
            "runtime_real_world_proxy_blocked_targets_ok".to_string(),
            json!(1),
        );
        m.insert(
            "runtime_real_world_proxy_blocked_targets_failed".to_string(),
            json!(0),
        );
        m
    }

    #[test]
    fn accepts_valid_proxy_logic() {
        let payload = base_ship();
        assert!(validate_proxy_logic(&payload).is_ok());
    }

    #[test]
    fn rejects_invalid_proxy_error() {
        let mut payload = base_ship();
        payload.insert(
            "runtime_real_world_proxy_probe_error".to_string(),
            json!("bad"),
        );
        let res = validate_proxy_logic(&payload);
        assert!(res.is_err());
        assert!(
            res.err()
                .is_some_and(|e| e.contains("error value is invalid"))
        );
    }

    #[test]
    fn rejects_not_attempted_with_non_zero_totals() {
        let mut payload = base_ship();
        payload.insert(
            "runtime_real_world_proxy_probe_attempted".to_string(),
            json!(false),
        );
        payload.insert(
            "runtime_real_world_proxy_listener_detected".to_string(),
            json!(false),
        );
        payload.insert(
            "runtime_real_world_proxy_probe_error".to_string(),
            json!("proxy_listener_not_found"),
        );
        let res = validate_proxy_logic(&payload);
        assert!(res.is_err());
        assert!(
            res.err()
                .is_some_and(|e| e.contains("not attempted with non-zero totals"))
        );
    }
}
