#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let reality_json = arg_or(&args, 1, "docs/REALITY_AUDIT_LATEST.json");
    let ship_json = arg_or(&args, 2, "docs/SHIP_READINESS_REPORT.json");
    let release_json = arg_or(&args, 3, "docs/RELEASE_READINESS_REPORT.json");
    let pack_json = arg_or(&args, 4, "docs/REPORT_PACK.json");
    let snapshot_json = arg_or(&args, 5, "docs/MVP_SNAPSHOT.json");
    let verify_json = arg_or(&args, 6, "docs/MVP_VERIFY.json");
    let release_audit_json = arg_or(&args, 7, "docs/release_readiness_audit.json");
    let ship_md = arg_or(&args, 8, "docs/SHIP_READINESS_REPORT.md");
    let pack_md = arg_or(&args, 9, "docs/REPORT_PACK.md");
    let rt_probe_json = arg_or(&args, 10, "docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json");

    let reality = read_obj(reality_json);
    let ship = read_obj(ship_json);
    let release = read_obj(release_json);
    let pack = read_obj(pack_json);
    let snapshot = read_obj(snapshot_json);
    let verify = read_obj(verify_json);
    let release_audit = read_obj(release_audit_json);
    let ship_md_raw = read_text(ship_md);
    let pack_md_raw = read_text(pack_md);
    let probe = read_obj(rt_probe_json);

    let expected_real_world = get_bool(&reality, "real_world_datapath_closed");
    for obj in [&ship, &release, &pack, &snapshot, &verify, &release_audit] {
        let truth = obj
            .get("truth_boundary")
            .and_then(Value::as_object)
            .unwrap_or_else(|| fail("reality truth guard: missing truth_boundary"));
        if truth.get("lab_scope_only").and_then(Value::as_bool) != Some(true)
            || truth
                .get("real_world_datapath_closed")
                .and_then(Value::as_bool)
                != Some(expected_real_world)
        {
            fail("reality truth guard: truth boundary mismatch");
        }
    }

    let expected_md = format!(
        "Real OS-level datapath closure (strict M4/M5): `{}`",
        if expected_real_world { "true" } else { "false" }
    );
    if !ship_md_raw.contains(&expected_md) || !pack_md_raw.contains(&expected_md) {
        fail("reality truth guard: markdown truth boundary mismatch");
    }

    require_field(&probe, "status", "ok");
    require_field(&probe, "kind", "runtime_real_world_probe_smoke");
    require_bool_field(&reality, "runtime_probe_file_ok", true);
    eq_bool_cross(
        &reality,
        "runtime_probe_direct_ok",
        &probe,
        "direct_probe_ok",
    );
    eq_bool_cross(
        &reality,
        "runtime_probe_datapath_attempted",
        &probe,
        "datapath_probe_attempted",
    );
    eq_bool_cross(
        &reality,
        "runtime_probe_datapath_ok",
        &probe,
        "datapath_probe_ok",
    );
    eq_str_cross(
        &reality,
        "runtime_probe_datapath_error",
        &probe,
        "datapath_probe_error",
    );
    eq_bool_cross(
        &reality,
        "runtime_probe_skipped_no_curl",
        &probe,
        "skipped_no_curl",
    );
    eq_i64_cross(
        &reality,
        "runtime_probe_datapath_targets_total",
        &probe,
        "datapath_targets_total",
    );
    eq_i64_cross(
        &reality,
        "runtime_probe_datapath_targets_ok",
        &probe,
        "datapath_targets_ok",
    );

    eq_bool_cross(
        &ship,
        "runtime_real_world_direct_probe_ok",
        &reality,
        "runtime_probe_direct_ok",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_datapath_probe_attempted",
        &reality,
        "runtime_probe_datapath_attempted",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_datapath_probe_ok",
        &reality,
        "runtime_probe_datapath_ok",
    );
    eq_str_cross(
        &ship,
        "runtime_real_world_datapath_probe_error",
        &reality,
        "runtime_probe_datapath_error",
    );
    eq_i64_cross(
        &ship,
        "runtime_real_world_datapath_targets_total",
        &reality,
        "runtime_probe_datapath_targets_total",
    );
    eq_i64_cross(
        &ship,
        "runtime_real_world_datapath_targets_ok",
        &reality,
        "runtime_probe_datapath_targets_ok",
    );
    eq_i64_cross(
        &ship,
        "runtime_real_world_datapath_targets_failed",
        &reality,
        "runtime_probe_datapath_targets_failed",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_skipped_no_curl",
        &reality,
        "runtime_probe_skipped_no_curl",
    );
    if let Err(msg) = validate_datapath_logic(&ship) {
        fail(&msg);
    }

    println!("reality truth guard: PASS");
}

fn arg_or<'a>(args: &'a [String], idx: usize, default: &'a str) -> &'a str {
    args.get(idx).map(String::as_str).unwrap_or(default)
}

fn read_text(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("reality truth guard: missing file: {path}")))
}

fn read_obj(path: &str) -> serde_json::Map<String, Value> {
    let raw = read_text(path);
    let v: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail(&format!("reality truth guard: invalid json: {path}")));
    v.as_object()
        .cloned()
        .unwrap_or_else(|| fail(&format!("reality truth guard: root not object: {path}")))
}

fn get_bool(obj: &serde_json::Map<String, Value>, key: &str) -> bool {
    obj.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn get_i64(obj: &serde_json::Map<String, Value>, key: &str) -> i64 {
    obj.get(key).and_then(Value::as_i64).unwrap_or(0)
}

fn require_field(obj: &serde_json::Map<String, Value>, key: &str, expected: &str) {
    if obj.get(key).and_then(Value::as_str) != Some(expected) {
        fail(&format!("reality truth guard: {key} mismatch"));
    }
}

fn require_bool_field(obj: &serde_json::Map<String, Value>, key: &str, expected: bool) {
    if obj.get(key).and_then(Value::as_bool) != Some(expected) {
        fail(&format!("reality truth guard: {key} mismatch"));
    }
}

fn eq_bool_cross(
    a: &serde_json::Map<String, Value>,
    ak: &str,
    b: &serde_json::Map<String, Value>,
    bk: &str,
) {
    if get_bool(a, ak) != get_bool(b, bk) {
        fail(&format!("reality truth guard: bool mismatch {ak} vs {bk}"));
    }
}

fn eq_i64_cross(
    a: &serde_json::Map<String, Value>,
    ak: &str,
    b: &serde_json::Map<String, Value>,
    bk: &str,
) {
    if get_i64(a, ak) != get_i64(b, bk) {
        fail(&format!("reality truth guard: int mismatch {ak} vs {bk}"));
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
        fail(&format!("reality truth guard: str mismatch {ak} vs {bk}"));
    }
}

fn validate_datapath_logic(ship: &serde_json::Map<String, Value>) -> Result<(), String> {
    let attempted = get_bool(ship, "runtime_real_world_datapath_probe_attempted");
    let ok_flag = get_bool(ship, "runtime_real_world_datapath_probe_ok");
    let skipped_no_curl = get_bool(ship, "runtime_real_world_skipped_no_curl");
    let error = ship
        .get("runtime_real_world_datapath_probe_error")
        .and_then(Value::as_str)
        .unwrap_or("");
    let total = get_i64(ship, "runtime_real_world_datapath_targets_total");
    let ok = get_i64(ship, "runtime_real_world_datapath_targets_ok");
    let failed = get_i64(ship, "runtime_real_world_datapath_targets_failed");
    if ![
        "none",
        "curl_not_found",
        "datapath_target_failed",
        "unknown",
    ]
    .contains(&error)
    {
        return Err("reality truth guard: datapath error value is invalid".to_string());
    }
    if ok + failed != total {
        return Err("reality truth guard: datapath totals mismatch".to_string());
    }
    if skipped_no_curl && attempted {
        return Err("reality truth guard: no curl but datapath attempted".to_string());
    }
    if !skipped_no_curl && !attempted {
        return Err(
            "reality truth guard: datapath must be attempted when curl is available".to_string(),
        );
    }
    if attempted && total <= 0 {
        return Err("reality truth guard: datapath attempted with empty target totals".to_string());
    }
    if !attempted && total != 0 {
        return Err("reality truth guard: datapath not attempted with non-zero totals".to_string());
    }
    if ok_flag && failed != 0 {
        return Err("reality truth guard: datapath ok with failed targets".to_string());
    }
    if attempted && error == "curl_not_found" {
        return Err("reality truth guard: datapath attempted with curl_not_found".to_string());
    }
    Ok(())
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::validate_datapath_logic;
    use serde_json::{Map, Value, json};

    fn base_ship() -> Map<String, Value> {
        let mut m = Map::new();
        m.insert(
            "runtime_real_world_datapath_probe_attempted".to_string(),
            json!(true),
        );
        m.insert(
            "runtime_real_world_datapath_probe_ok".to_string(),
            json!(true),
        );
        m.insert(
            "runtime_real_world_datapath_probe_error".to_string(),
            json!("none"),
        );
        m.insert(
            "runtime_real_world_datapath_targets_total".to_string(),
            json!(1),
        );
        m.insert(
            "runtime_real_world_datapath_targets_ok".to_string(),
            json!(1),
        );
        m.insert(
            "runtime_real_world_datapath_targets_failed".to_string(),
            json!(0),
        );
        m.insert(
            "runtime_real_world_skipped_no_curl".to_string(),
            json!(false),
        );
        m
    }

    #[test]
    fn accepts_valid_datapath_logic() {
        let payload = base_ship();
        assert!(validate_datapath_logic(&payload).is_ok());
    }

    #[test]
    fn rejects_invalid_datapath_error() {
        let mut payload = base_ship();
        payload.insert(
            "runtime_real_world_datapath_probe_error".to_string(),
            json!("bad"),
        );
        let res = validate_datapath_logic(&payload);
        assert!(res.is_err());
        assert!(
            res.err()
                .is_some_and(|e| e.contains("error value is invalid"))
        );
    }

    #[test]
    fn rejects_attempted_with_empty_totals() {
        let mut payload = base_ship();
        payload.insert(
            "runtime_real_world_datapath_targets_total".to_string(),
            json!(0),
        );
        payload.insert(
            "runtime_real_world_datapath_targets_ok".to_string(),
            json!(0),
        );
        let res = validate_datapath_logic(&payload);
        assert!(res.is_err());
        assert!(res.err().is_some_and(|e| e.contains("empty target totals")));
    }
}
