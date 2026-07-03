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
    let ship_status = ship.get("status").and_then(Value::as_str).unwrap_or("");
    if !["ok", "fail"].contains(&ship_status) {
        fail("reality ship sync guard: envelope mismatch");
    }
    require_str(&ship, "kind", "ship_readiness_report");
    require_str(&probe, "status", "ok");
    require_str(&probe, "kind", "runtime_real_world_probe_smoke");
    eq_bool_cross(
        &ship,
        "github_release_ssh_runtime_slice_proven",
        &reality,
        "github_release_ssh_runtime_slice_proven",
    );

    eq_bool_cross(
        &ship,
        "runtime_real_world_direct_probe_ok",
        &reality,
        "runtime_probe_direct_ok",
    );
    eq_str_cross(
        &ship,
        "runtime_real_world_probe_mode",
        &reality,
        "runtime_probe_mode",
    );
    eq_str_cross(
        &ship,
        "runtime_real_world_evidence_kind",
        &reality,
        "runtime_probe_evidence_kind",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_chimera_datapath_evidence",
        &reality,
        "runtime_probe_chimera_datapath_evidence",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_live_external_probe",
        &reality,
        "runtime_probe_live_external_probe",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_ssh_stand_required_for_live_probe",
        &reality,
        "runtime_probe_ssh_stand_required_for_live_probe",
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
    eq_bool_cross(
        &ship,
        "runtime_real_world_skipped_no_curl",
        &reality,
        "runtime_probe_skipped_no_curl",
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
        &reality,
        "runtime_probe_direct_ok",
        &probe,
        "direct_probe_ok",
    );
    eq_str_cross(&reality, "runtime_probe_mode", &probe, "probe_mode");
    eq_str_cross(
        &reality,
        "runtime_probe_evidence_kind",
        &probe,
        "evidence_kind",
    );
    eq_bool_cross(
        &reality,
        "runtime_probe_chimera_datapath_evidence",
        &probe,
        "chimera_datapath_evidence",
    );
    eq_bool_cross(
        &reality,
        "runtime_probe_live_external_probe",
        &probe,
        "live_external_probe",
    );
    eq_bool_cross(
        &reality,
        "runtime_probe_ssh_stand_required_for_live_probe",
        &probe,
        "ssh_stand_required_for_live_probe",
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
    eq_i64_cross(
        &reality,
        "runtime_probe_datapath_targets_failed",
        &probe,
        "datapath_targets_failed",
    );
    if let Err(msg) = validate_datapath_logic(&ship) {
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

fn validate_datapath_logic(ship: &serde_json::Map<String, Value>) -> Result<(), String> {
    let evidence_kind = ship
        .get("runtime_real_world_evidence_kind")
        .and_then(Value::as_str)
        .unwrap_or("");
    let chimera_evidence = get_bool(ship, "runtime_real_world_chimera_datapath_evidence");
    let datapath_release_ok = ship
        .get("runtime_real_world_datapath_release_ok")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            "reality ship sync guard: invalid runtime_real_world_datapath_release_ok".to_string()
        })?;
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
        "chimera_datapath_evidence_missing",
        "ci_snapshot",
        "unknown",
    ]
    .contains(&error)
    {
        return Err("reality ship sync guard: datapath error value is invalid".to_string());
    }
    if ok + failed != total {
        return Err("reality ship sync guard: datapath totals mismatch".to_string());
    }
    if ![
        "external_reachability_without_system_proxy",
        "ci_snapshot_contract",
        "chimera_transparent_datapath",
    ]
    .contains(&evidence_kind)
    {
        return Err("reality ship sync guard: evidence kind value is invalid".to_string());
    }
    if ok_flag && !chimera_evidence {
        return Err("reality ship sync guard: datapath ok without CHIMERA evidence".to_string());
    }
    let external_attempted = get_bool(
        ship,
        "runtime_real_world_external_reachability_probe_attempted",
    );
    let external_ok_flag = get_bool(ship, "runtime_real_world_external_reachability_probe_ok");
    let external_total = get_i64(
        ship,
        "runtime_real_world_external_reachability_targets_total",
    );
    let external_ok = get_i64(ship, "runtime_real_world_external_reachability_targets_ok");
    let external_failed = get_i64(
        ship,
        "runtime_real_world_external_reachability_targets_failed",
    );
    if external_ok + external_failed != external_total {
        return Err("reality ship sync guard: external reachability totals mismatch".to_string());
    }
    let probe_mode = ship
        .get("runtime_real_world_probe_mode")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !["live", "ci_snapshot"].contains(&probe_mode) {
        return Err("reality ship sync guard: probe mode value is invalid".to_string());
    }
    let ci_snapshot = probe_mode == "ci_snapshot";
    if get_bool(ship, "runtime_real_world_live_external_probe") == ci_snapshot {
        return Err("reality ship sync guard: live external probe flag mismatch".to_string());
    }
    if get_bool(ship, "runtime_real_world_ssh_stand_required_for_live_probe") != ci_snapshot {
        return Err("reality ship sync guard: ssh stand required flag mismatch".to_string());
    }
    let expected_datapath_release_ok = !ci_snapshot
        && evidence_kind == "chimera_transparent_datapath"
        && chimera_evidence
        && attempted
        && ok_flag
        && !skipped_no_curl
        && total > 0
        && ok == total
        && failed == 0;
    if datapath_release_ok != expected_datapath_release_ok {
        return Err(
            "reality ship sync guard: datapath release flag does not match CHIMERA evidence"
                .to_string(),
        );
    }
    if ship.get("status").and_then(Value::as_str) == Some("ok") && !datapath_release_ok {
        return Err(
            "reality ship sync guard: status ok requires CHIMERA datapath release evidence"
                .to_string(),
        );
    }
    if ci_snapshot {
        if attempted || ok_flag || skipped_no_curl || external_attempted || external_ok_flag {
            return Err(
                "reality ship sync guard: ci_snapshot cannot report live probe attempt".to_string(),
            );
        }
        if total != 0 || ok != 0 || failed != 0 || external_total != 0 {
            return Err("reality ship sync guard: ci_snapshot must have zero totals".to_string());
        }
        if error != "ci_snapshot" {
            return Err(
                "reality ship sync guard: ci_snapshot requires ci_snapshot error marker"
                    .to_string(),
            );
        }
        if evidence_kind != "ci_snapshot_contract" || chimera_evidence {
            return Err(
                "reality ship sync guard: ci_snapshot evidence fields mismatch".to_string(),
            );
        }
        return Ok(());
    }
    if skipped_no_curl && attempted {
        return Err("reality ship sync guard: no curl but datapath attempted".to_string());
    }
    if evidence_kind == "external_reachability_without_system_proxy" {
        if chimera_evidence || attempted || ok_flag {
            return Err(
                "reality ship sync guard: external reachability must not masquerade as datapath"
                    .to_string(),
            );
        }
        if !skipped_no_curl && !external_attempted {
            return Err(
                "reality ship sync guard: external reachability must be attempted when curl is available"
                    .to_string(),
            );
        }
        if external_attempted && external_total <= 0 {
            return Err(
                "reality ship sync guard: external reachability attempted with empty totals"
                    .to_string(),
            );
        }
        if external_ok_flag && external_failed != 0 {
            return Err(
                "reality ship sync guard: external reachability ok with failed targets".to_string(),
            );
        }
        if !skipped_no_curl && error != "chimera_datapath_evidence_missing" {
            return Err(
                "reality ship sync guard: external reachability requires missing datapath evidence marker"
                    .to_string(),
            );
        }
        return Ok(());
    }
    if !skipped_no_curl && !attempted {
        return Err(
            "reality ship sync guard: datapath must be attempted when CHIMERA evidence is available"
                .to_string(),
        );
    }
    if attempted && total <= 0 {
        return Err("reality ship sync guard: datapath attempted with empty totals".to_string());
    }
    if !attempted && total != 0 {
        return Err(
            "reality ship sync guard: datapath not attempted with non-zero totals".to_string(),
        );
    }
    if ok_flag && failed != 0 {
        return Err("reality ship sync guard: datapath ok with failed targets".to_string());
    }
    if attempted && error == "curl_not_found" {
        return Err("reality ship sync guard: datapath attempted with curl_not_found".to_string());
    }
    if error == "none" && !ok_flag {
        return Err("reality ship sync guard: datapath failed without error marker".to_string());
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
        m.insert("runtime_real_world_probe_mode".to_string(), json!("live"));
        m.insert(
            "runtime_real_world_evidence_kind".to_string(),
            json!("chimera_transparent_datapath"),
        );
        m.insert(
            "runtime_real_world_chimera_datapath_evidence".to_string(),
            json!(true),
        );
        m.insert(
            "runtime_real_world_datapath_release_ok".to_string(),
            json!(true),
        );
        m.insert(
            "runtime_real_world_live_external_probe".to_string(),
            json!(true),
        );
        m.insert(
            "runtime_real_world_ssh_stand_required_for_live_probe".to_string(),
            json!(false),
        );
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
        m.insert(
            "runtime_real_world_external_reachability_probe_attempted".to_string(),
            json!(false),
        );
        m.insert(
            "runtime_real_world_external_reachability_probe_ok".to_string(),
            json!(false),
        );
        m.insert(
            "runtime_real_world_external_reachability_targets_total".to_string(),
            json!(0),
        );
        m.insert(
            "runtime_real_world_external_reachability_targets_ok".to_string(),
            json!(0),
        );
        m.insert(
            "runtime_real_world_external_reachability_targets_failed".to_string(),
            json!(0),
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
    fn rejects_not_attempted_with_non_zero_totals() {
        let mut payload = base_ship();
        payload.insert(
            "runtime_real_world_datapath_probe_attempted".to_string(),
            json!(false),
        );
        payload.insert(
            "runtime_real_world_datapath_probe_ok".to_string(),
            json!(false),
        );
        payload.insert(
            "runtime_real_world_datapath_release_ok".to_string(),
            json!(false),
        );
        payload.insert(
            "runtime_real_world_datapath_probe_error".to_string(),
            json!("curl_not_found"),
        );
        payload.insert(
            "runtime_real_world_skipped_no_curl".to_string(),
            json!(true),
        );
        let res = validate_datapath_logic(&payload);
        assert!(res.is_err());
        assert!(
            res.err()
                .is_some_and(|e| e.contains("not attempted with non-zero totals"))
        );
    }
}
