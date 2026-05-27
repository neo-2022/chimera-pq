#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let report_json = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/SHIP_READINESS_REPORT.json");
    let report_md = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("docs/SHIP_READINESS_REPORT.md");

    let report = read_obj(report_json);
    let md = fs::read_to_string(report_md)
        .unwrap_or_else(|_| fail(&format!("ship structure guard: missing file: {report_md}")));

    let expected_steps: BTreeSet<&str> = [
        "baseline_freeze",
        "cleanroom_handoff_check",
        "benchmark_regression_gate",
        "cef_track_report",
        "cef_track_guard",
        "cef_track_sync_guard",
        "cef_gap_map_guard",
        "cef_consistency_guard",
        "cef_phase1_smoke",
        "mesh_auto_smoke",
        "mesh_auto_adaptive_trace_guard",
        "mesh_cli_recovery_schema_guard_selfcheck",
        "mesh_cli_recovery_schema_guard",
        "release_readiness_report_json",
        "release_readiness_report_ru",
        "report_pack_json",
        "report_pack_md",
        "runtime_apply_dns_smoke",
        "runtime_apply_route_smoke_selfcheck",
        "runtime_apply_route_smoke",
        "runtime_apply_route_existing_tun_smoke_selfcheck",
        "runtime_apply_route_existing_tun_smoke",
        "runtime_apply_route_multi_cidr_smoke_selfcheck",
        "runtime_apply_route_multi_cidr_smoke",
        "runtime_route_policy_validation_smoke_selfcheck",
        "runtime_route_policy_validation_smoke",
        "runtime_route_duplicate_cidr_validation_smoke_selfcheck",
        "runtime_route_duplicate_cidr_validation_smoke",
        "runtime_tun_name_validation_smoke_selfcheck",
        "runtime_tun_name_validation_smoke",
        "runtime_resolv_conf_validation_smoke_selfcheck",
        "runtime_resolv_conf_validation_smoke",
        "runtime_datapath_multiflow_smoke_selfcheck",
        "runtime_datapath_multiflow_smoke",
        "runtime_policy_precedence_smoke_selfcheck",
        "runtime_policy_precedence_smoke",
        "runtime_forced_stop_rollback_smoke_selfcheck",
        "runtime_forced_stop_rollback_smoke",
        "rust_no_hardcode_guard_selfcheck",
        "rust_no_hardcode_guard",
        "runtime_real_world_probe_smoke_selfcheck",
        "runtime_real_world_probe_smoke",
        "runtime_real_world_probe_schema_guard_selfcheck",
        "runtime_real_world_probe_schema_guard",
        "probe_access_smoke_selfcheck",
        "probe_access_smoke",
        "reality_audit_refresh_selfcheck",
        "reality_audit_refresh",
        "reality_audit_schema_guard_selfcheck",
        "reality_audit_schema_guard",
        "reality_ship_sync_guard_selfcheck",
        "reality_ship_sync_guard",
        "freshness_check",
    ]
    .into_iter()
    .collect();

    let steps = report
        .get("steps")
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail("steps is not object"));
    let actual: BTreeSet<&str> = steps.keys().map(String::as_str).collect();
    if actual != expected_steps {
        if !expected_steps.is_subset(&actual) {
            fail("missing steps:");
        }
        fail("unexpected steps:");
    }
    if steps.values().any(|v| v.as_bool() != Some(true)) {
        fail("non-true steps:");
    }

    let checks = [
        "- Baseline freeze: `true`",
        "- Clean-room handoff check: `true`",
        "- CEF track report: `true`",
        "- CEF track guard: `true`",
        "- CEF track sync guard: `true`",
        "- Benchmark regression gate: `true`",
        "- CEF gap map guard: `true`",
        "- CEF consistency guard: `true`",
        "- Mesh auto smoke: `true`",
        "- Mesh auto adaptive trace guard: `true`",
        "- Mesh CLI recovery schema guard selfcheck: `true`",
        "- Mesh CLI recovery schema guard: `true`",
        "- Release readiness JSON: `true`",
        "- Release readiness RU markdown: `true`",
        "- Report pack JSON: `true`",
        "- Report pack markdown: `true`",
    ];
    let mut prev = 0usize;
    for item in checks {
        let ln = line_once(&md, item);
        if ln <= prev {
            fail("ship structure guard: check order mismatch");
        }
        prev = ln;
    }

    println!("ship structure guard: PASS");
}

fn read_obj(path: &str) -> serde_json::Map<String, Value> {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("ship structure guard: missing file: {path}")));
    let v: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail(&format!("ship structure guard: invalid json: {path}")));
    v.as_object()
        .cloned()
        .unwrap_or_else(|| fail("ship structure guard: root not object"))
}

fn line_once(md: &str, line: &str) -> usize {
    let mut found = None;
    for (i, l) in md.lines().enumerate() {
        if l == line {
            if found.is_some() {
                fail("ship structure guard: duplicate check line");
            }
            found = Some(i + 1);
        }
    }
    found.unwrap_or_else(|| fail("ship structure guard: missing check line"))
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
