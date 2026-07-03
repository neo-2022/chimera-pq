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
    if get_bool(&ship, "github_release_ssh_runtime_slice_proven")
        != get_bool(&release, "github_release_ssh_runtime_slice_proven")
        || get_bool(&ship, "github_release_ssh_runtime_slice_proven")
            != get_bool(&pack, "github_release_ssh_runtime_slice_proven")
        || get_bool(&ship, "github_release_ssh_runtime_slice_proven")
            != get_bool(&reality, "github_release_ssh_runtime_slice_proven")
    {
        fail("ship nonregression guard: GitHub SSH runtime slice proof mismatch");
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
        require_bool_field(&rt_route, "counts_for_release", true);
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
        require_bool_field(&rt_route_multi, "apply_attempt_ok", true);
        require_bool_field(&rt_route_multi, "policy_rule_ok", true);
        require_bool_field(&rt_route_multi, "counts_for_release", true);
        if get_bool(&rt_route_multi, "skipped_no_tun") {
            fail("ship nonregression guard: route multi-cidr skipped_no_tun is not releasable");
        }
    }
    if get_bool(&ship, "runtime_forced_stop_rollback_smoke_ok") {
        require_field(&rt_forced, "status", "ok");
        require_field(&rt_forced, "kind", "runtime_forced_stop_rollback_smoke");
        require_field(&rt_forced, "network_state", "modified");
        require_bool_field(&rt_forced, "apply_attempt_ok", true);
        require_bool_field(&rt_forced, "recover_ok", true);
        require_bool_field(&rt_forced, "down_state_clean", true);
        require_bool_field(&rt_forced, "counts_for_release", true);
        if get_bool(&rt_forced, "skipped_no_tun") {
            fail("ship nonregression guard: forced-stop skipped_no_tun is not releasable");
        }
    }
    require_field(&rt_probe, "status", "ok");
    require_field(&rt_probe, "kind", "runtime_real_world_probe_smoke");
    require_field(&rt_probe, "network_state", "not_modified");
    eq_str_cross(
        &ship,
        "runtime_real_world_probe_mode",
        &rt_probe,
        "probe_mode",
    );
    eq_str_cross(
        &ship,
        "runtime_real_world_evidence_kind",
        &rt_probe,
        "evidence_kind",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_chimera_datapath_evidence",
        &rt_probe,
        "chimera_datapath_evidence",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_live_external_probe",
        &rt_probe,
        "live_external_probe",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_ssh_stand_required_for_live_probe",
        &rt_probe,
        "ssh_stand_required_for_live_probe",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_datapath_probe_attempted",
        &rt_probe,
        "datapath_probe_attempted",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_datapath_probe_ok",
        &rt_probe,
        "datapath_probe_ok",
    );
    eq_str_cross(
        &ship,
        "runtime_real_world_datapath_probe_error",
        &rt_probe,
        "datapath_probe_error",
    );
    eq_i64_cross(
        &ship,
        "runtime_real_world_datapath_targets_total",
        &rt_probe,
        "datapath_targets_total",
    );
    eq_i64_cross(
        &ship,
        "runtime_real_world_datapath_targets_ok",
        &rt_probe,
        "datapath_targets_ok",
    );
    eq_i64_cross(
        &ship,
        "runtime_real_world_datapath_targets_failed",
        &rt_probe,
        "datapath_targets_failed",
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
        "runtime_real_world_external_reachability_probe_attempted",
        &rt_probe,
        "external_reachability_probe_attempted",
    );
    eq_bool_cross(
        &ship,
        "runtime_real_world_external_reachability_probe_ok",
        &rt_probe,
        "external_reachability_probe_ok",
    );
    eq_i64_cross(
        &ship,
        "runtime_real_world_external_reachability_targets_total",
        &rt_probe,
        "external_reachability_targets_total",
    );
    eq_i64_cross(
        &ship,
        "runtime_real_world_external_reachability_targets_ok",
        &rt_probe,
        "external_reachability_targets_ok",
    );
    eq_i64_cross(
        &ship,
        "runtime_real_world_external_reachability_targets_failed",
        &rt_probe,
        "external_reachability_targets_failed",
    );
    validate_datapath_logic(&ship)
        .unwrap_or_else(|msg| fail(&format!("ship nonregression guard: {msg}")));

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
fn require_step_true(root: &serde_json::Map<String, Value>, step: &str) {
    let steps = root
        .get("steps")
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail("ship nonregression guard: steps missing"));
    if steps.get(step).and_then(Value::as_bool) != Some(true) {
        fail(&format!("ship nonregression guard: step not true: {step}"));
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
            "ship nonregression guard: str mismatch {ak} vs {bk}"
        ));
    }
}

fn validate_datapath_logic(ship: &serde_json::Map<String, Value>) -> Result<(), String> {
    let legacy_mode = !ship.contains_key("runtime_real_world_datapath_probe_attempted")
        && !ship.contains_key("runtime_real_world_datapath_probe_ok")
        && !ship.contains_key("runtime_real_world_datapath_probe_error")
        && !ship.contains_key("runtime_real_world_datapath_targets_total")
        && !ship.contains_key("runtime_real_world_datapath_targets_ok")
        && !ship.contains_key("runtime_real_world_datapath_targets_failed");
    if legacy_mode {
        return Err("legacy proxy runtime proof is not releasable".to_string());
    }

    let attempted = if legacy_mode {
        get_bool(ship, "runtime_real_world_proxy_probe_attempted")
    } else {
        get_bool(ship, "runtime_real_world_datapath_probe_attempted")
    };
    let ok_flag = if legacy_mode {
        get_bool(ship, "runtime_real_world_proxy_probe_ok")
    } else {
        get_bool(ship, "runtime_real_world_datapath_probe_ok")
    };
    let skipped_no_curl = if legacy_mode {
        get_bool(ship, "runtime_real_world_skipped_no_proxy_listener")
    } else {
        get_bool(ship, "runtime_real_world_skipped_no_curl")
    };
    let error = if legacy_mode {
        get_str(ship, "runtime_real_world_proxy_probe_error")
    } else {
        get_str(ship, "runtime_real_world_datapath_probe_error")
    };
    let total = if legacy_mode {
        get_i64(ship, "runtime_real_world_proxy_blocked_targets_total")
    } else {
        get_i64(ship, "runtime_real_world_datapath_targets_total")
    };
    let ok = if legacy_mode {
        get_i64(ship, "runtime_real_world_proxy_blocked_targets_ok")
    } else {
        get_i64(ship, "runtime_real_world_datapath_targets_ok")
    };
    let failed = if legacy_mode {
        get_i64(ship, "runtime_real_world_proxy_blocked_targets_failed")
    } else {
        get_i64(ship, "runtime_real_world_datapath_targets_failed")
    };
    let evidence_kind = get_str(ship, "runtime_real_world_evidence_kind");
    let chimera_evidence = get_bool(ship, "runtime_real_world_chimera_datapath_evidence");
    let datapath_release_ok = ship
        .get("runtime_real_world_datapath_release_ok")
        .and_then(Value::as_bool)
        .ok_or_else(|| "invalid runtime_real_world_datapath_release_ok".to_string())?;
    if legacy_mode {
        if !["none", "proxy_listener_not_found", "unknown"].contains(&error) {
            return Err("datapath error value is invalid".to_string());
        }
    } else if ![
        "none",
        "curl_not_found",
        "datapath_target_failed",
        "chimera_datapath_evidence_missing",
        "ci_snapshot",
        "unknown",
    ]
    .contains(&error)
    {
        return Err("datapath error value is invalid".to_string());
    }
    if ok + failed != total {
        return Err("datapath totals mismatch".to_string());
    }
    if !legacy_mode
        && ![
            "external_reachability_without_system_proxy",
            "ci_snapshot_contract",
            "chimera_transparent_datapath",
        ]
        .contains(&evidence_kind)
    {
        return Err("evidence kind value is invalid".to_string());
    }
    if ok_flag && !legacy_mode && !chimera_evidence {
        return Err("datapath ok without CHIMERA evidence".to_string());
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
    if !legacy_mode && external_ok + external_failed != external_total {
        return Err("external reachability totals mismatch".to_string());
    }
    let probe_mode = get_str(ship, "runtime_real_world_probe_mode");
    if !["live", "ci_snapshot"].contains(&probe_mode) {
        return Err("probe mode value is invalid".to_string());
    }
    let ci_snapshot = probe_mode == "ci_snapshot";
    if get_bool(ship, "runtime_real_world_live_external_probe") == ci_snapshot {
        return Err("live external probe flag mismatch".to_string());
    }
    if get_bool(ship, "runtime_real_world_ssh_stand_required_for_live_probe") != ci_snapshot {
        return Err("ssh stand required flag mismatch".to_string());
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
        return Err("datapath release flag does not match CHIMERA evidence".to_string());
    }
    if get_str(ship, "status") == "ok" && !datapath_release_ok {
        return Err("status ok requires CHIMERA datapath release evidence".to_string());
    }
    if ci_snapshot {
        if attempted || ok_flag || skipped_no_curl || external_attempted || external_ok_flag {
            return Err("ci_snapshot cannot report live probe attempt".to_string());
        }
        if total != 0 || ok != 0 || failed != 0 || external_total != 0 {
            return Err("ci_snapshot must have zero probe totals".to_string());
        }
        if error != "ci_snapshot" {
            return Err("ci_snapshot requires ci_snapshot error marker".to_string());
        }
        if !legacy_mode && (evidence_kind != "ci_snapshot_contract" || chimera_evidence) {
            return Err("ci_snapshot evidence fields mismatch".to_string());
        }
        if get_bool(ship, "runtime_real_world_direct_probe_ok") {
            return Err("ci_snapshot cannot report direct probe success".to_string());
        }
        return Ok(());
    }
    if skipped_no_curl && attempted {
        return Err("no curl but datapath attempted".to_string());
    }
    if !legacy_mode && evidence_kind == "external_reachability_without_system_proxy" {
        if chimera_evidence || attempted || ok_flag {
            return Err("external reachability must not masquerade as datapath".to_string());
        }
        if !skipped_no_curl && !external_attempted {
            return Err(
                "external reachability must be attempted when curl is available".to_string(),
            );
        }
        if external_attempted && external_total <= 0 {
            return Err("external reachability attempted with empty target totals".to_string());
        }
        if external_ok_flag && external_failed != 0 {
            return Err("external reachability ok with failed targets".to_string());
        }
        if !skipped_no_curl && error != "chimera_datapath_evidence_missing" {
            return Err(
                "external reachability requires missing datapath evidence marker".to_string(),
            );
        }
        return Ok(());
    }
    if !skipped_no_curl && !attempted {
        return Err("datapath must be attempted when CHIMERA evidence is available".to_string());
    }
    if attempted && total <= 0 {
        return Err("datapath attempted with empty target totals".to_string());
    }
    if !attempted && total != 0 {
        return Err("datapath not attempted with non-zero totals".to_string());
    }
    if ok_flag && failed != 0 {
        return Err("datapath ok with failed targets".to_string());
    }
    if attempted && error == "curl_not_found" {
        return Err("datapath attempted with curl_not_found".to_string());
    }
    Ok(())
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
