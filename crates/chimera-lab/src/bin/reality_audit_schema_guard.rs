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
        "runtime_probe_mode",
        "runtime_probe_live_external_probe",
        "runtime_probe_ssh_stand_required_for_live_probe",
        "runtime_probe_direct_ok",
        "runtime_probe_datapath_ok",
        "runtime_probe_datapath_attempted",
        "runtime_probe_datapath_error",
        "runtime_probe_skipped_no_curl",
        "runtime_probe_datapath_targets_total",
        "runtime_probe_datapath_targets_ok",
        "runtime_probe_datapath_targets_failed",
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
        "runtime_probe_live_external_probe",
        "runtime_probe_ssh_stand_required_for_live_probe",
        "runtime_probe_direct_ok",
        "runtime_probe_datapath_ok",
        "runtime_probe_datapath_attempted",
        "runtime_probe_skipped_no_curl",
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
    let total = get_i64(data, "runtime_probe_datapath_targets_total");
    let ok = get_i64(data, "runtime_probe_datapath_targets_ok");
    let failed = get_i64(data, "runtime_probe_datapath_targets_failed");
    if total < 0 || ok < 0 || failed < 0 {
        return Err("reality audit field not non-negative int".to_string());
    }
    if ok > total {
        return Err("runtime_probe_datapath_targets_ok exceeds total".to_string());
    }
    if failed > total {
        return Err("runtime_probe_datapath_targets_failed exceeds total".to_string());
    }
    if ok + failed != total {
        return Err("runtime_probe_datapath_targets totals mismatch".to_string());
    }
    let datapath_error = get_str(data, "runtime_probe_datapath_error");
    if ![
        "none",
        "curl_not_found",
        "datapath_target_failed",
        "ci_snapshot",
        "unknown",
    ]
    .contains(&datapath_error)
    {
        return Err("runtime_probe_datapath_error invalid".to_string());
    }
    let datapath_attempted = get_bool(data, "runtime_probe_datapath_attempted");
    let datapath_ok = get_bool(data, "runtime_probe_datapath_ok");
    let skipped_no_curl = get_bool(data, "runtime_probe_skipped_no_curl");
    let probe_mode = get_str(data, "runtime_probe_mode");
    if !["live", "ci_snapshot"].contains(&probe_mode) {
        return Err("runtime_probe_mode invalid".to_string());
    }
    let ci_snapshot = probe_mode == "ci_snapshot";
    if get_bool(data, "runtime_probe_live_external_probe") == ci_snapshot {
        return Err("runtime_probe_live_external_probe mismatch".to_string());
    }
    if get_bool(data, "runtime_probe_ssh_stand_required_for_live_probe") != ci_snapshot {
        return Err("runtime_probe_ssh_stand_required_for_live_probe mismatch".to_string());
    }
    if skipped_no_curl && datapath_attempted {
        return Err("runtime_probe no curl but datapath attempted".to_string());
    }
    if ci_snapshot {
        if get_bool(data, "runtime_probe_direct_ok") || datapath_ok || datapath_attempted {
            return Err("runtime_probe ci_snapshot cannot report live probe success".to_string());
        }
        if skipped_no_curl {
            return Err(
                "runtime_probe ci_snapshot must not masquerade as missing curl".to_string(),
            );
        }
        if datapath_error != "ci_snapshot" {
            return Err("runtime_probe ci_snapshot requires ci_snapshot error marker".to_string());
        }
        if total != 0 || ok != 0 || failed != 0 {
            return Err("runtime_probe ci_snapshot must have zero target totals".to_string());
        }
    } else if !skipped_no_curl && !datapath_attempted {
        return Err("runtime_probe datapath must be attempted when curl is available".to_string());
    }
    if datapath_attempted && datapath_error == "curl_not_found" {
        return Err("runtime_probe datapath attempted with curl_not_found".to_string());
    }
    if datapath_ok && failed != 0 {
        return Err("runtime_probe datapath ok requires failed=0".to_string());
    }
    if datapath_attempted && total <= 0 {
        return Err("runtime_probe datapath attempted with empty target totals".to_string());
    }
    if !datapath_attempted && total != 0 {
        return Err("runtime_probe datapath not attempted with non-zero totals".to_string());
    }
    if datapath_error == "none" && !datapath_ok {
        return Err("runtime_probe datapath failed without error marker".to_string());
    }

    let runtime_probe_path_ok = probe_mode == "live"
        && get_bool(data, "runtime_probe_live_external_probe")
        && !get_bool(data, "runtime_probe_ssh_stand_required_for_live_probe")
        && get_bool(data, "runtime_probe_direct_ok")
        && datapath_ok
        && datapath_attempted
        && total > 0
        && failed == 0;

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
        m.insert("runtime_probe_mode".to_string(), json!("live"));
        m.insert("runtime_probe_live_external_probe".to_string(), json!(true));
        m.insert(
            "runtime_probe_ssh_stand_required_for_live_probe".to_string(),
            json!(false),
        );
        m.insert("runtime_probe_direct_ok".to_string(), json!(true));
        m.insert("runtime_probe_datapath_ok".to_string(), json!(true));
        m.insert("runtime_probe_datapath_attempted".to_string(), json!(true));
        m.insert("runtime_probe_datapath_error".to_string(), json!("none"));
        m.insert("runtime_probe_skipped_no_curl".to_string(), json!(false));
        m.insert("runtime_probe_datapath_targets_total".to_string(), json!(2));
        m.insert("runtime_probe_datapath_targets_ok".to_string(), json!(2));
        m.insert(
            "runtime_probe_datapath_targets_failed".to_string(),
            json!(0),
        );
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
    fn rejects_not_attempted_when_curl_available() {
        let mut payload = base();
        payload.insert("runtime_probe_datapath_attempted".to_string(), json!(false));
        payload.insert("runtime_probe_datapath_ok".to_string(), json!(false));
        payload.insert("runtime_probe_datapath_targets_total".to_string(), json!(0));
        payload.insert("runtime_probe_datapath_targets_ok".to_string(), json!(0));
        let res = validate_reality_audit(&payload);
        assert!(res.is_err());
        assert!(res.err().is_some_and(|e| e.contains("must be attempted")));
    }

    #[test]
    fn rejects_bad_datapath_error() {
        let mut payload = base();
        payload.insert("runtime_probe_datapath_error".to_string(), json!("bad"));
        let res = validate_reality_audit(&payload);
        assert!(res.is_err());
        assert!(
            res.err()
                .is_some_and(|e| e.contains("datapath_error invalid"))
        );
    }

    #[test]
    fn accepts_ci_snapshot_without_reality_closure() {
        let mut payload = base();
        payload.insert("runtime_probe_mode".to_string(), json!("ci_snapshot"));
        payload.insert(
            "runtime_probe_live_external_probe".to_string(),
            json!(false),
        );
        payload.insert(
            "runtime_probe_ssh_stand_required_for_live_probe".to_string(),
            json!(true),
        );
        payload.insert("runtime_probe_direct_ok".to_string(), json!(false));
        payload.insert("runtime_probe_datapath_ok".to_string(), json!(false));
        payload.insert("runtime_probe_datapath_attempted".to_string(), json!(false));
        payload.insert(
            "runtime_probe_datapath_error".to_string(),
            json!("ci_snapshot"),
        );
        payload.insert("runtime_probe_datapath_targets_total".to_string(), json!(0));
        payload.insert("runtime_probe_datapath_targets_ok".to_string(), json!(0));
        payload.insert(
            "runtime_probe_datapath_targets_failed".to_string(),
            json!(0),
        );
        payload.insert("runtime_evidence_closed".to_string(), json!(false));
        payload.insert("real_world_datapath_closed".to_string(), json!(false));

        assert!(validate_reality_audit(&payload).is_ok());
    }
}
