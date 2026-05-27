#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;
use std::time::UNIX_EPOCH;

fn main() {
    let args: Vec<String> = env::args().collect();
    let report_json = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/SHIP_READINESS_REPORT.json");
    let max_age_sec = args
        .get(2)
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1800);

    let report = read_obj(report_json);
    let report_ts = report
        .get("generated_at")
        .and_then(Value::as_str)
        .unwrap_or_else(|| fail("ship readiness freshness guard: missing generated_at"));
    let report_epoch = parse_rfc3339_z(report_ts);

    let fresh_artifacts = [
        "docs/SHIP_READINESS_REPORT.md",
        "docs/CEF_TRACK_REPORT.json",
        "docs/CEF_TRACK_REPORT.md",
        "docs/RELEASE_READINESS_REPORT.json",
        "docs/RELEASE_READINESS_REPORT.md",
        "docs/RELEASE_READINESS_REPORT_RU.md",
        "docs/REPORT_PACK.json",
        "docs/REPORT_PACK.md",
        "docs/CEF_PHASE1_SMOKE.json",
        "docs/BENCHMARK_REGRESSION_GATE.json",
        "docs/benchmark_latest.json",
        "docs/benchmark_baseline.json",
        "docs/RUNTIME_APPLY_DNS_SMOKE.json",
        "docs/RUNTIME_APPLY_ROUTE_SMOKE.json",
        "docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json",
        "docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json",
        "docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json",
        "docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json",
        "docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json",
        "docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json",
        "docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json",
        "docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json",
        "docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json",
        "docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json",
        "docs/REALITY_AUDIT_LATEST.json",
    ];
    let required_artifacts = [
        "docs/CEF_GAP_MAP_2026-05-18.md",
        "docs/release_readiness_audit.json",
        "docs/MVP_HANDOFF_CHECKLIST.md",
        "docs/SECOND_MACHINE_REPORT.md",
    ];

    for artifact in required_artifacts {
        if fs::metadata(artifact).is_err() {
            fail(&format!(
                "ship readiness freshness guard: missing required artifact: {artifact}"
            ));
        }
    }

    for artifact in fresh_artifacts {
        let meta = fs::metadata(artifact).unwrap_or_else(|_| {
            fail(&format!(
                "ship readiness freshness guard: missing fresh artifact: {artifact}"
            ))
        });
        let mtime_epoch = modified_epoch(&meta);
        let delta = report_epoch - mtime_epoch;
        if delta < -30 || delta > max_age_sec {
            fail(&format!(
                "ship readiness freshness guard: stale/out-of-window artifact: {artifact} (delta={delta}s, max_age_sec={max_age_sec})"
            ));
        }
    }

    println!("ship readiness freshness guard: PASS");
}

fn read_obj(path: &str) -> serde_json::Map<String, Value> {
    let raw = fs::read_to_string(path).unwrap_or_else(|_| {
        fail(&format!(
            "ship readiness freshness guard: missing file: {path}"
        ))
    });
    let v: Value = serde_json::from_str(&raw).unwrap_or_else(|_| {
        fail(&format!(
            "ship readiness freshness guard: invalid json: {path}"
        ))
    });
    v.as_object()
        .cloned()
        .unwrap_or_else(|| fail("ship readiness freshness guard: root not object"))
}

fn modified_epoch(meta: &fs::Metadata) -> i64 {
    let t = meta
        .modified()
        .unwrap_or_else(|_| fail("ship readiness freshness guard: cannot read mtime"));
    t.duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| fail("ship readiness freshness guard: mtime before epoch"))
        .as_secs() as i64
}

fn parse_rfc3339_z(ts: &str) -> i64 {
    if ts.len() != 20 || !ts.ends_with('Z') {
        fail("ship readiness freshness guard: bad generated_at format");
    }
    let y = ts[0..4]
        .parse::<i32>()
        .unwrap_or_else(|_| fail("ship readiness freshness guard: bad year"));
    let m = ts[5..7]
        .parse::<u32>()
        .unwrap_or_else(|_| fail("ship readiness freshness guard: bad month"));
    let d = ts[8..10]
        .parse::<u32>()
        .unwrap_or_else(|_| fail("ship readiness freshness guard: bad day"));
    let hh = ts[11..13]
        .parse::<u32>()
        .unwrap_or_else(|_| fail("ship readiness freshness guard: bad hour"));
    let mm = ts[14..16]
        .parse::<u32>()
        .unwrap_or_else(|_| fail("ship readiness freshness guard: bad minute"));
    let ss = ts[17..19]
        .parse::<u32>()
        .unwrap_or_else(|_| fail("ship readiness freshness guard: bad second"));
    chrono_epoch(y, m, d, hh, mm, ss)
}

fn chrono_epoch(y: i32, m: u32, d: u32, hh: u32, mm: u32, ss: u32) -> i64 {
    let days = days_from_civil(y, m, d);
    days * 86_400 + (hh as i64) * 3600 + (mm as i64) * 60 + (ss as i64)
}

fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = y - (m <= 2) as i32;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = m as i32 + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era as i64) * 146097 + (doe as i64) - 719468
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
