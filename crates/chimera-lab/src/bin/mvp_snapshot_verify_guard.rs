#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let snapshot_json = arg_or(&args, 1, "docs/MVP_SNAPSHOT.json");
    let verify_json = arg_or(&args, 2, "docs/MVP_VERIFY.json");
    let reality_json = arg_or(&args, 3, "docs/REALITY_AUDIT_LATEST.json");

    let snapshot = read_obj(snapshot_json);
    let verify = read_obj(verify_json);
    let reality = read_obj(reality_json);
    let truth_expected = truth_expected(&reality);

    require_str(&snapshot, "status", "ok");
    require_str(&snapshot, "kind", "mvp_snapshot");
    require_str(&verify, "status", "ok");
    require_str(&verify, "kind", "mvp_verify");

    if snapshot.get("truth_boundary") != Some(&truth_expected) {
        fail("snapshot truth_boundary mismatch");
    }
    if verify.get("truth_boundary") != Some(&truth_expected) {
        fail("verify truth_boundary mismatch");
    }

    require_bool(&snapshot, "release_ready", true);
    require_bool(&snapshot, "artifact_audit", true);
    require_str(&snapshot, "network_state", "not_modified");
    require_str(&verify, "network_state", "not_modified");
    require_bool(&verify, "artifact_audit", true);
    require_bool(&verify, "mvp_snapshot", true);

    let mvp = get_obj(&snapshot, "mvp", "snapshot mvp is not object");
    let mvp_expected: BTreeSet<&str> = ["m0", "m1", "m2", "m3", "m4", "m5", "m6"]
        .into_iter()
        .collect();
    exact_keys(mvp, &mvp_expected, "snapshot mvp keys mismatch");
    all_true(mvp, "snapshot mvp contains non-true values");

    let gate_expected: BTreeSet<&str> = [
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
    let gate = get_obj(
        &snapshot,
        "release_gate",
        "snapshot release_gate is not object",
    );
    exact_keys(gate, &gate_expected, "snapshot release_gate keys mismatch");
    all_true(gate, "snapshot release_gate contains non-true values");

    let verify_expected = [
        "smoke",
        "fuzz_smoke",
        "perf_smoke",
        "net_sim",
        "lab_doctor",
        "mvp_spec_check",
        "mvp_spec_report",
        "m5_artifacts_report",
        "m6_artifacts_report",
        "release_readiness_report",
        "report_pack",
        "artifact_audit",
        "mvp_snapshot",
    ];
    for k in verify_expected {
        require_bool(&verify, k, true);
    }

    let snapshot_top: BTreeSet<&str> = [
        "status",
        "kind",
        "message_en",
        "message_ru",
        "release_ready",
        "truth_boundary",
        "mvp",
        "release_gate",
        "artifact_audit",
        "network_state",
    ]
    .into_iter()
    .collect();
    let verify_top: BTreeSet<&str> = [
        "status",
        "kind",
        "message_en",
        "message_ru",
        "refreshed",
        "smoke",
        "fuzz_smoke",
        "perf_smoke",
        "net_sim",
        "lab_doctor",
        "mvp_spec_check",
        "mvp_spec_report",
        "m5_artifacts_report",
        "m6_artifacts_report",
        "release_readiness_report",
        "report_pack",
        "artifact_audit",
        "mvp_snapshot",
        "truth_boundary",
        "network_state",
    ]
    .into_iter()
    .collect();
    exact_top_keys(&snapshot, &snapshot_top, "snapshot top-level keys mismatch");
    exact_top_keys(&verify, &verify_top, "verify top-level keys mismatch");

    println!("mvp snapshot/verify guard: PASS");
}

fn arg_or<'a>(args: &'a [String], idx: usize, default: &'a str) -> &'a str {
    args.get(idx).map(String::as_str).unwrap_or(default)
}

fn read_obj(path: &str) -> serde_json::Map<String, Value> {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("mvp snapshot/verify guard: missing file: {path}")));
    let v: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail(&format!("mvp snapshot/verify guard: invalid json: {path}")));
    v.as_object().cloned().unwrap_or_else(|| {
        fail(&format!(
            "mvp snapshot/verify guard: root not object: {path}"
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
