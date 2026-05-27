#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let ship_json = arg_or(&args, 1, "docs/SHIP_READINESS_REPORT.json");
    let release_json = arg_or(&args, 2, "docs/RELEASE_READINESS_REPORT.json");
    let pack_json = arg_or(&args, 3, "docs/REPORT_PACK.json");
    let rt_dns_json = arg_or(&args, 4, "docs/RUNTIME_APPLY_DNS_SMOKE.json");
    let rt_route_json = arg_or(&args, 5, "docs/RUNTIME_APPLY_ROUTE_SMOKE.json");
    let rt_route_multi_cidr_json =
        arg_or(&args, 6, "docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json");
    let rt_forced_stop_json = arg_or(&args, 7, "docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json");
    let rt_probe_json = arg_or(&args, 8, "docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json");
    let reality_json = arg_or(&args, 9, "docs/REALITY_AUDIT_LATEST.json");

    let ship = read_obj(ship_json);
    let release = read_obj(release_json);
    let pack = read_obj(pack_json);
    let rt_dns = read_obj(rt_dns_json);
    let rt_route = read_obj(rt_route_json);
    let rt_route_multi = read_obj(rt_route_multi_cidr_json);
    let rt_forced = read_obj(rt_forced_stop_json);
    let rt_probe = read_obj(rt_probe_json);
    let reality = read_obj(reality_json);

    let expected_real_world = get_bool(&reality, "real_world_datapath_closed");
    for obj in [&ship, &release, &pack] {
        let truth = obj
            .get("truth_boundary")
            .and_then(Value::as_object)
            .unwrap_or_else(|| fail("ship nonregression guard: missing truth_boundary"));
        if truth.get("lab_scope_only").and_then(Value::as_bool) != Some(true)
            || truth
                .get("real_world_datapath_closed")
                .and_then(Value::as_bool)
                != Some(expected_real_world)
        {
            fail("ship nonregression guard: truth_boundary mismatch");
        }
    }

    if get_bool(&ship, "release_ok") != get_bool(&release, "release_ok") {
        fail("ship nonregression guard: release_ok mismatch");
    }

    if get_bool(&ship, "runtime_apply_smoke_modified") {
        require_field(&rt_dns, "status", "ok");
        require_field(&rt_dns, "kind", "runtime_apply_dns_smoke");
        require_field(&rt_dns, "network_state", "modified");
        require_bool_field(&rt_dns, "rollback_ok", true);
    }
    if get_bool(&ship, "runtime_apply_route_smoke_modified") {
        require_field(&rt_route, "status", "ok");
        require_field(&rt_route, "kind", "runtime_apply_route_smoke");
        require_field(&rt_route, "network_state", "modified");
        require_bool_field(&rt_route, "apply_attempt_ok", true);
        require_bool_field(&rt_route, "rollback_ok", true);
    }
    if get_bool(&ship, "runtime_apply_route_multi_cidr_smoke_ok") {
        require_field(&rt_route_multi, "status", "ok");
        require_field(
            &rt_route_multi,
            "kind",
            "runtime_apply_route_multi_cidr_smoke",
        );
        require_field(&rt_route_multi, "network_state", "modified");
        require_bool_field(&rt_route_multi, "rollback_ok", true);
        let applied = get_bool(&rt_route_multi, "apply_attempt_ok")
            && get_bool(&rt_route_multi, "policy_rule_ok");
        let skipped = get_bool(&rt_route_multi, "skipped_no_tun");
        if !applied && !skipped {
            fail("ship nonregression guard: route multi-cidr neither applied nor skipped");
        }
    }
    if get_bool(&ship, "runtime_forced_stop_rollback_smoke_ok") {
        require_field(&rt_forced, "status", "ok");
        require_field(&rt_forced, "kind", "runtime_forced_stop_rollback_smoke");
        require_field(&rt_forced, "network_state", "modified");
        require_bool_field(&rt_forced, "apply_attempt_ok", true);
        require_bool_field(&rt_forced, "recover_ok", true);
        require_bool_field(&rt_forced, "down_state_clean", true);
    }
    if get_bool(&ship, "runtime_real_world_probe_smoke_ok") {
        require_field(&rt_probe, "status", "ok");
        require_field(&rt_probe, "kind", "runtime_real_world_probe_smoke");
        require_field(&rt_probe, "network_state", "not_modified");
        eq_bool_cross(
            &ship,
            "runtime_real_world_proxy_listener_detected",
            &rt_probe,
            "proxy_listener_detected",
        );
        eq_bool_cross(
            &ship,
            "runtime_real_world_proxy_probe_attempted",
            &rt_probe,
            "proxy_probe_attempted",
        );
        eq_bool_cross(
            &ship,
            "runtime_real_world_proxy_probe_ok",
            &rt_probe,
            "proxy_probe_ok",
        );
        eq_bool_cross(
            &ship,
            "runtime_real_world_proxy_selected_from_candidates",
            &rt_probe,
            "proxy_selected_from_candidates",
        );
        eq_str_cross(
            &ship,
            "runtime_real_world_proxy_candidates",
            &rt_probe,
            "proxy_candidates",
        );
        eq_str_cross(
            &ship,
            "runtime_real_world_proxy_probe_error",
            &rt_probe,
            "proxy_probe_error",
        );
        eq_i64_cross(
            &ship,
            "runtime_real_world_proxy_blocked_targets_total",
            &rt_probe,
            "proxy_blocked_targets_total",
        );
        eq_i64_cross(
            &ship,
            "runtime_real_world_proxy_blocked_targets_ok",
            &rt_probe,
            "proxy_blocked_targets_ok",
        );
        eq_i64_cross(
            &ship,
            "runtime_real_world_proxy_blocked_targets_failed",
            &rt_probe,
            "proxy_blocked_targets_failed",
        );
        eq_bool_cross(
            &ship,
            "runtime_real_world_direct_probe_ok",
            &rt_probe,
            "direct_probe_ok",
        );
        eq_bool_cross(
            &ship,
            "runtime_real_world_skipped_no_curl",
            &rt_probe,
            "skipped_no_curl",
        );
        eq_bool_cross(
            &ship,
            "runtime_real_world_skipped_no_proxy_listener",
            &rt_probe,
            "skipped_no_proxy_listener",
        );
        let proxy_attempted = get_bool(&ship, "runtime_real_world_proxy_probe_attempted");
        let proxy_listener = get_bool(&ship, "runtime_real_world_proxy_listener_detected");
        let proxy_ok = get_bool(&ship, "runtime_real_world_proxy_probe_ok");
        let proxy_selected = get_bool(&ship, "runtime_real_world_proxy_selected_from_candidates");
        let proxy_candidates = get_str(&ship, "runtime_real_world_proxy_candidates");
        let proxy_error = get_str(&ship, "runtime_real_world_proxy_probe_error");
        let total = get_i64(&ship, "runtime_real_world_proxy_blocked_targets_total");
        let ok = get_i64(&ship, "runtime_real_world_proxy_blocked_targets_ok");
        let failed = get_i64(&ship, "runtime_real_world_proxy_blocked_targets_failed");
        if ![
            "none",
            "proxy_listener_not_found",
            "proxy_connect_or_upstream_failed",
            "unknown",
        ]
        .contains(&proxy_error)
        {
            fail("ship nonregression guard: proxy error value is invalid");
        }
        if ok + failed != total {
            fail("ship nonregression guard: proxy totals mismatch");
        }
        if proxy_attempted && !proxy_listener {
            fail("ship nonregression guard: proxy attempted without listener");
        }
        if !proxy_attempted && proxy_error != "proxy_listener_not_found" {
            fail("ship nonregression guard: proxy not attempted must be listener_not_found");
        }
        if proxy_attempted && proxy_error == "proxy_listener_not_found" {
            fail("ship nonregression guard: proxy attempted with listener_not_found");
        }
        if proxy_ok && failed != 0 {
            fail("ship nonregression guard: proxy ok with failed targets");
        }
        if proxy_selected && !proxy_attempted {
            fail("ship nonregression guard: proxy selected_from_candidates requires attempted");
        }
        if proxy_candidates.trim().is_empty() {
            fail("ship nonregression guard: proxy candidates is empty");
        }
        if proxy_attempted && total <= 0 {
            fail("ship nonregression guard: proxy attempted with empty target totals");
        }
        if !proxy_attempted && total != 0 {
            fail("ship nonregression guard: proxy not attempted with non-zero totals");
        }
    }

    require_step_true(&ship, "report_pack_json");
    require_step_true(&ship, "report_pack_md");
    require_step_true(&ship, "release_readiness_report_json");
    require_step_true(&ship, "release_readiness_report_ru");
    require_step_true(&ship, "cef_phase1_smoke");
    require_step_true(&ship, "mesh_auto_smoke");
    require_step_true(&ship, "mesh_auto_adaptive_trace_guard");
    require_bool_field(&pack, "cef_phase1_smoke", true);
    require_bool_field(&pack, "mesh_route_explain", true);
    require_bool_field(&pack, "mesh_auto_adaptive_trace", true);

    println!("ship nonregression guard: PASS");
}

fn arg_or<'a>(args: &'a [String], idx: usize, default: &'a str) -> &'a str {
    args.get(idx).map(String::as_str).unwrap_or(default)
}

fn read_obj(path: &str) -> serde_json::Map<String, Value> {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("ship nonregression guard: missing file: {path}")));
    let v: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail(&format!("ship nonregression guard: invalid json: {path}")));
    v.as_object().cloned().unwrap_or_else(|| {
        fail(&format!(
            "ship nonregression guard: root not object: {path}"
        ))
    })
}

fn get_bool(obj: &serde_json::Map<String, Value>, key: &str) -> bool {
    obj.get(key).and_then(Value::as_bool).unwrap_or(false)
}
fn get_i64(obj: &serde_json::Map<String, Value>, key: &str) -> i64 {
    obj.get(key).and_then(Value::as_i64).unwrap_or(0)
}
fn get_str<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> &'a str {
    obj.get(key).and_then(Value::as_str).unwrap_or("")
}

fn require_field(obj: &serde_json::Map<String, Value>, key: &str, expected: &str) {
    if get_str(obj, key) != expected {
        fail(&format!("ship nonregression guard: {key} mismatch"));
    }
}
fn require_bool_field(obj: &serde_json::Map<String, Value>, key: &str, expected: bool) {
    if obj.get(key).and_then(Value::as_bool) != Some(expected) {
        fail(&format!("ship nonregression guard: {key} mismatch"));
    }
}
fn require_step_true(ship: &serde_json::Map<String, Value>, key: &str) {
    let steps = ship
        .get("steps")
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail("ship nonregression guard: missing steps object"));
    if steps.get(key).and_then(Value::as_bool) != Some(true) {
        fail(&format!("ship nonregression guard: step not true: {key}"));
    }
}

fn eq_bool_cross(
    a: &serde_json::Map<String, Value>,
    ak: &str,
    b: &serde_json::Map<String, Value>,
    bk: &str,
) {
    if get_bool(a, ak) != get_bool(b, bk) {
        fail(&format!(
            "ship nonregression guard: bool mismatch {ak} vs {bk}"
        ));
    }
}
fn eq_i64_cross(
    a: &serde_json::Map<String, Value>,
    ak: &str,
    b: &serde_json::Map<String, Value>,
    bk: &str,
) {
    if get_i64(a, ak) != get_i64(b, bk) {
        fail(&format!(
            "ship nonregression guard: int mismatch {ak} vs {bk}"
        ));
    }
}
fn eq_str_cross(
    a: &serde_json::Map<String, Value>,
    ak: &str,
    b: &serde_json::Map<String, Value>,
    bk: &str,
) {
    if get_str(a, ak) != get_str(b, bk) {
        fail(&format!(
            "ship nonregression guard: string mismatch {ak} vs {bk}"
        ));
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
