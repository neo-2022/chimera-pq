#![forbid(unsafe_code)]

use serde_json::Value;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let src_md = "docs/REALITY_AUDIT_2026-05-18.md";
    let out_json = "docs/REALITY_AUDIT_LATEST.json";

    let mut source_status = "not_found";
    let mut md_claim_closed = false;
    let mut md_claim_partial = false;
    if let Ok(md) = fs::read_to_string(src_md) {
        source_status = "parsed";
        md_claim_closed = md.contains("Real OS-level datapath closure for strict M4/M5: CLOSED.");
        md_claim_partial =
            md.contains("Real OS-level datapath closure for strict M4/M5: PARTIAL / NOT CLOSED.");
    }

    let probe = load_json_obj("docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json");
    let route = load_json_obj("docs/RUNTIME_APPLY_ROUTE_SMOKE.json");
    let forced = load_json_obj("docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json");

    let runtime_probe_file_ok = probe.is_some();
    let runtime_route_file_ok = route.is_some();
    let runtime_forced_file_ok = forced.is_some();

    let runtime_probe_direct_ok = probe_bool(&probe, "direct_probe_ok");
    let runtime_probe_proxy_ok = probe_bool(&probe, "proxy_probe_ok");
    let runtime_probe_proxy_listener_detected = probe_bool(&probe, "proxy_listener_detected");
    let runtime_probe_proxy_probe_attempted = probe_bool(&probe, "proxy_probe_attempted");
    let runtime_probe_proxy_selected_from_candidates =
        probe_bool(&probe, "proxy_selected_from_candidates");
    let runtime_probe_proxy_error = probe_str(&probe, "proxy_probe_error");
    let runtime_probe_proxy_candidates = probe_str(&probe, "proxy_candidates");
    let runtime_probe_skipped_no_proxy_listener = probe_bool(&probe, "skipped_no_proxy_listener");
    let runtime_probe_blocked_targets_total = probe_i64(&probe, "proxy_blocked_targets_total");
    let runtime_probe_blocked_targets_ok = probe_i64(&probe, "proxy_blocked_targets_ok");
    let runtime_probe_blocked_targets_failed = probe_i64(&probe, "proxy_blocked_targets_failed");

    let runtime_route_apply_ok = route.as_ref().is_some_and(|r| {
        r.get("status").and_then(Value::as_str) == Some("ok")
            && r.get("kind").and_then(Value::as_str) == Some("runtime_apply_route_smoke")
            && r.get("network_state").and_then(Value::as_str) == Some("modified")
            && r.get("apply_attempt_ok").and_then(Value::as_bool) == Some(true)
            && r.get("policy_rule_ok").and_then(Value::as_bool) == Some(true)
    });
    let runtime_route_rollback_ok = route
        .as_ref()
        .and_then(|r| r.get("rollback_ok").and_then(Value::as_bool))
        .unwrap_or(false);

    let runtime_forced_recover_ok = forced
        .as_ref()
        .and_then(|r| r.get("recover_ok").and_then(Value::as_bool))
        .unwrap_or(false);
    let runtime_forced_clean_ok = forced
        .as_ref()
        .and_then(|r| r.get("down_state_clean").and_then(Value::as_bool))
        .unwrap_or(false);

    let runtime_probe_path_ok = runtime_probe_direct_ok
        && (runtime_probe_proxy_ok || runtime_probe_skipped_no_proxy_listener)
        && (!runtime_probe_proxy_probe_attempted
            || runtime_probe_blocked_targets_total == 0
            || runtime_probe_blocked_targets_ok >= 1);
    let runtime_evidence_closed = runtime_probe_path_ok
        && runtime_route_apply_ok
        && runtime_route_rollback_ok
        && runtime_forced_recover_ok
        && runtime_forced_clean_ok;
    let closed = md_claim_closed && runtime_evidence_closed;

    let generated_at = now_utc_z();
    let payload = serde_json::json!({
      "status":"ok",
      "kind":"reality_audit",
      "message_en":"Reality audit snapshot refreshed.",
      "message_ru":"Снимок reality audit обновлен.",
      "real_world_datapath_closed":closed,
      "source_markdown":src_md,
      "source_status":source_status,
      "md_claim_closed":md_claim_closed,
      "md_claim_partial_not_closed":md_claim_partial,
      "runtime_probe_file_ok":runtime_probe_file_ok,
      "runtime_probe_direct_ok":runtime_probe_direct_ok,
      "runtime_probe_proxy_ok":runtime_probe_proxy_ok,
      "runtime_probe_proxy_listener_detected":runtime_probe_proxy_listener_detected,
      "runtime_probe_proxy_probe_attempted":runtime_probe_proxy_probe_attempted,
      "runtime_probe_proxy_selected_from_candidates":runtime_probe_proxy_selected_from_candidates,
      "runtime_probe_proxy_error":runtime_probe_proxy_error,
      "runtime_probe_proxy_candidates":runtime_probe_proxy_candidates,
      "runtime_probe_skipped_no_proxy_listener":runtime_probe_skipped_no_proxy_listener,
      "runtime_probe_blocked_targets_total":runtime_probe_blocked_targets_total,
      "runtime_probe_blocked_targets_ok":runtime_probe_blocked_targets_ok,
      "runtime_probe_blocked_targets_failed":runtime_probe_blocked_targets_failed,
      "runtime_route_file_ok":runtime_route_file_ok,
      "runtime_route_apply_ok":runtime_route_apply_ok,
      "runtime_route_rollback_ok":runtime_route_rollback_ok,
      "runtime_forced_file_ok":runtime_forced_file_ok,
      "runtime_forced_recover_ok":runtime_forced_recover_ok,
      "runtime_forced_clean_ok":runtime_forced_clean_ok,
      "runtime_evidence_closed":runtime_evidence_closed,
      "network_state":"not_modified",
      "generated_at":generated_at
    });
    let text = serde_json::to_string(&payload)
        .unwrap_or_else(|_| fail("reality audit refresh: json encode failure"));
    fs::write(out_json, text).unwrap_or_else(|_| fail("reality audit refresh: write failed"));
    println!("reality audit refresh: {out_json} (real_world_datapath_closed={closed})");
}

fn load_json_obj(path: &str) -> Option<serde_json::Map<String, Value>> {
    let raw = fs::read_to_string(path).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    let o = v.as_object()?;
    if o.get("status").and_then(Value::as_str) != Some("ok") {
        return None;
    }
    Some(o.clone())
}

fn probe_bool(map: &Option<serde_json::Map<String, Value>>, key: &str) -> bool {
    map.as_ref()
        .and_then(|m| m.get(key).and_then(Value::as_bool))
        .unwrap_or(false)
}

fn probe_i64(map: &Option<serde_json::Map<String, Value>>, key: &str) -> i64 {
    map.as_ref()
        .and_then(|m| m.get(key).and_then(Value::as_i64))
        .unwrap_or(0)
}

fn probe_str(map: &Option<serde_json::Map<String, Value>>, key: &str) -> String {
    map.as_ref()
        .and_then(|m| m.get(key).and_then(Value::as_str))
        .unwrap_or("unknown")
        .to_string()
}

fn now_utc_z() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| fail("time went backwards"))
        .as_secs() as i64;
    format_utc(secs)
}

fn format_utc(ts: i64) -> String {
    let days = ts.div_euclid(86_400);
    let sod = ts.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let hh = sod / 3600;
    let mm = (sod % 3600) / 60;
    let ss = sod % 60;
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i32) + (era as i32) * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let y = y + if m <= 2 { 1 } else { 0 };
    (y, m as u32, d as u32)
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
