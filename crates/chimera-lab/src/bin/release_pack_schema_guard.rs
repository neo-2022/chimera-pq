#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let release_json = arg_or(&args, 1, "docs/RELEASE_READINESS_REPORT.json");
    let pack_json = arg_or(&args, 2, "docs/REPORT_PACK.json");
    let reality_json = arg_or(&args, 3, "docs/REALITY_AUDIT_LATEST.json");

    let release = read_obj(release_json);
    let pack = read_obj(pack_json);
    let reality = read_obj(reality_json);

    let truth_expected = truth_expected(&reality);

    require_str(&release, "status", "ok");
    require_str(&release, "kind", "release_readiness_report");
    require_str(&pack, "status", "ok");
    require_str(&pack, "kind", "report_pack");

    if release.get("truth_boundary") != Some(&truth_expected) {
        fail("release truth_boundary mismatch");
    }
    if pack.get("truth_boundary") != Some(&truth_expected) {
        fail("pack truth_boundary mismatch");
    }

    require_bool(&release, "release_ok", true);
    require_str(&release, "network_state", "not_modified");
    require_str(&pack, "network_state", "not_modified");

    let milestones = get_obj(&release, "milestones", "milestones is not object");
    let milestones_expected: BTreeSet<&str> = [
        "m0_workspace",
        "m1_local_tunnel",
        "m2_crypto_session",
        "m3_carrier_validation",
        "m4_routing_determinism",
        "m5_doctor_and_config",
        "m6_hardening",
    ]
    .into_iter()
    .collect();
    exact_keys(milestones, &milestones_expected, "milestones mismatch");
    all_true(milestones, "milestones contain non-true values");

    let release_gate_expected: BTreeSet<&str> = [
        "clean_clone_builds",
        "client_gateway_run_linux",
        "encrypted_tunnel_carries_traffic",
        "policy_routing_direct_gateway_block",
        "dns_binding_works",
        "route_explain_works",
        "shutdown_restores_network_state",
        "security_tests_pass",
        "parser_fuzz_smoke_passes",
        "no_raw_secrets_in_logs",
        "benchmark_report_exists",
        "operations_guide_exists",
        "runtime_apply_dns_verified",
        "runtime_apply_route_verified",
        "runtime_route_policy_validation_verified",
        "runtime_tun_name_validation_verified",
        "runtime_forced_stop_rollback_verified",
    ]
    .into_iter()
    .collect();
    let rel_gate = get_obj(&release, "release_gate", "release_gate is not object");
    let pack_gate = get_obj(&pack, "release_gate", "release_gate is not object");
    exact_keys(
        rel_gate,
        &release_gate_expected,
        "release_gate keys mismatch",
    );
    exact_keys(
        pack_gate,
        &release_gate_expected,
        "pack release_gate keys mismatch",
    );
    all_true(rel_gate, "release_gate contains non-true values");
    all_true(pack_gate, "pack release_gate contains non-true values");
    if rel_gate != pack_gate {
        fail("release_gate differs between release and pack");
    }

    let artifacts = get_obj(&release, "artifacts", "artifacts is not object");
    let artifacts_expected: BTreeSet<&str> = [
        "m5_report",
        "m6_report",
        "benchmark",
        "cef_phase1_smoke",
        "mesh_route_explain",
        "mesh_auto_adaptive_trace",
    ]
    .into_iter()
    .collect();
    exact_keys(artifacts, &artifacts_expected, "artifacts mismatch");
    all_true(artifacts, "artifacts contain non-true values");

    let pack_top_expected: BTreeSet<&str> = [
        "status",
        "kind",
        "message_en",
        "message_ru",
        "mvp_spec_report",
        "m5_artifacts_report",
        "m6_artifacts_report",
        "release_readiness_report",
        "cef_phase1_smoke",
        "mesh_route_explain",
        "mesh_auto_adaptive_trace",
        "truth_boundary",
        "release_gate",
        "network_state",
    ]
    .into_iter()
    .collect();
    exact_top_keys(&pack, &pack_top_expected, "pack top-level keys mismatch");
    for k in [
        "mvp_spec_report",
        "m5_artifacts_report",
        "m6_artifacts_report",
        "release_readiness_report",
        "cef_phase1_smoke",
        "mesh_route_explain",
        "mesh_auto_adaptive_trace",
    ] {
        require_bool(&pack, k, true);
    }

    let release_top_expected: BTreeSet<&str> = [
        "status",
        "kind",
        "message_en",
        "message_ru",
        "release_ok",
        "truth_boundary",
        "milestones",
        "release_gate",
        "artifacts",
        "network_state",
    ]
    .into_iter()
    .collect();
    exact_top_keys(
        &release,
        &release_top_expected,
        "release top-level keys mismatch",
    );

    println!("release-pack schema guard: PASS");
}

fn arg_or<'a>(args: &'a [String], idx: usize, default: &'a str) -> &'a str {
    args.get(idx).map(String::as_str).unwrap_or(default)
}

fn read_obj(path: &str) -> serde_json::Map<String, Value> {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("release-pack schema guard: missing file: {path}")));
    let v: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail(&format!("release-pack schema guard: invalid json: {path}")));
    v.as_object().cloned().unwrap_or_else(|| {
        fail(&format!(
            "release-pack schema guard: root not object: {path}"
        ))
    })
}

fn truth_expected(reality: &serde_json::Map<String, Value>) -> Value {
    let real_world = reality
        .get("real_world_datapath_closed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    serde_json::json!({"lab_scope_only": true, "real_world_datapath_closed": real_world})
}

fn require_str(obj: &serde_json::Map<String, Value>, key: &str, expected: &str) {
    if obj.get(key).and_then(Value::as_str) != Some(expected) {
        fail(&format!("{key} mismatch"));
    }
}

fn require_bool(obj: &serde_json::Map<String, Value>, key: &str, expected: bool) {
    if obj.get(key).and_then(Value::as_bool) != Some(expected) {
        fail(&format!("{key} is not {expected}"));
    }
}

fn get_obj<'a>(
    root: &'a serde_json::Map<String, Value>,
    key: &str,
    err: &str,
) -> &'a serde_json::Map<String, Value> {
    root.get(key)
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail(err))
}

fn exact_keys(obj: &serde_json::Map<String, Value>, expected: &BTreeSet<&str>, msg: &str) {
    let keys: BTreeSet<&str> = obj.keys().map(String::as_str).collect();
    if keys != *expected {
        fail(msg);
    }
}

fn exact_top_keys(obj: &serde_json::Map<String, Value>, expected: &BTreeSet<&str>, msg: &str) {
    let keys: BTreeSet<&str> = obj.keys().map(String::as_str).collect();
    if keys != *expected {
        fail(msg);
    }
}

fn all_true(obj: &serde_json::Map<String, Value>, msg: &str) {
    if obj.values().any(|v| v.as_bool() != Some(true)) {
        fail(msg);
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
