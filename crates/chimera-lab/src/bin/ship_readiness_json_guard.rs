#![forbid(unsafe_code)]

use serde_json::Value;
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
    let reality_json = args
        .get(3)
        .map(String::as_str)
        .unwrap_or("docs/REALITY_AUDIT_LATEST.json");

    let report_raw = read_file(report_json);
    let report_md_raw = read_file(report_md);
    let reality_raw = read_file(reality_json);

    let report = parse_json(
        &report_raw,
        "ship readiness json guard: invalid report json",
    );
    let reality = parse_json(
        &reality_raw,
        "ship readiness json guard: invalid reality json",
    );

    let report_obj = report
        .as_object()
        .unwrap_or_else(|| fail("ship readiness json guard: report root is not object"));
    let reality_obj = reality
        .as_object()
        .unwrap_or_else(|| fail("ship readiness json guard: reality root is not object"));
    require_str_eq(report_obj, "status", "ok");
    require_str_eq(report_obj, "kind", "ship_readiness_report");
    require_bool_eq(report_obj, "release_ok", true);
    require_bool_eq(report_obj, "release_ok_lab_only", true);
    require_bool_eq(report_obj, "cef_phase1_smoke_ok", true);
    require_bool_eq(report_obj, "mesh_route_explain_ok", true);
    require_bool_eq(report_obj, "mesh_auto_adaptive_ok", true);
    require_bool_eq(report_obj, "runtime_real_world_probe_smoke_ok", true);

    let generated_at = require_ts_z(report_obj, "generated_at");

    let truth = report_obj
        .get("truth_boundary")
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail("ship readiness json guard: missing truth_boundary"));
    let real_world_expected = reality_obj
        .get("real_world_datapath_closed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    require_bool_eq(truth, "lab_scope_only", true);
    require_bool_eq(truth, "real_world_datapath_closed", real_world_expected);

    for key in [
        "cef_track_report",
        "cef_track_guard",
        "cef_track_sync_guard",
        "cef_gap_map_guard",
        "cef_consistency_guard",
        "benchmark_regression_gate",
        "runtime_apply_dns_smoke",
        "runtime_apply_route_smoke_selfcheck",
        "runtime_apply_route_smoke",
        "mesh_auto_smoke",
        "mesh_auto_adaptive_trace_guard",
    ] {
        require_step_true(report_obj, key);
    }

    for key in [
        "runtime_real_world_datapath_targets_total",
        "runtime_real_world_datapath_targets_ok",
        "runtime_real_world_datapath_targets_failed",
    ] {
        if report_obj
            .get(key)
            .and_then(Value::as_i64)
            .is_none_or(|v| v < 0)
        {
            fail(&format!(
                "ship readiness json guard: invalid int field: {key}"
            ));
        }
    }
    if let Err(msg) = validate_runtime_datapath_logic(report_obj) {
        fail(&msg);
    }

    for key in [
        "runtime_real_world_datapath_probe_attempted",
        "runtime_real_world_datapath_probe_ok",
        "runtime_real_world_direct_probe_ok",
        "runtime_real_world_skipped_no_curl",
    ] {
        if report_obj.get(key).and_then(Value::as_bool).is_none() {
            fail(&format!(
                "ship readiness json guard: invalid bool field: {key}"
            ));
        }
    }
    if let Err(msg) = validate_direct_probe_visibility(report_obj) {
        fail(&msg);
    }

    let probe_error = report_obj
        .get("runtime_real_world_datapath_probe_error")
        .and_then(Value::as_str)
        .unwrap_or("");
    if ![
        "none",
        "curl_not_found",
        "datapath_target_failed",
        "unknown",
    ]
    .contains(&probe_error)
    {
        fail("ship readiness json guard: invalid runtime_real_world_datapath_probe_error");
    }

    require_md_contains(&report_md_raw, "CEF track sync guard:");
    require_md_contains(&report_md_raw, "Truth boundary:");
    require_md_contains(
        &report_md_raw,
        &format!(
            "Real OS-level datapath closure (strict M4/M5): `{}`",
            if real_world_expected { "true" } else { "false" }
        ),
    );
    require_md_contains(&report_md_raw, "Runtime real-world datapath targets total:");
    require_md_contains(&report_md_raw, "Runtime real-world datapath targets ok:");
    require_md_contains(
        &report_md_raw,
        "Runtime real-world datapath targets failed:",
    );
    require_md_contains(&report_md_raw, "Mesh route explain:");
    require_md_contains(&report_md_raw, "Mesh auto adaptive trace:");
    require_md_contains(
        &report_md_raw,
        "Runtime real-world datapath probe attempted:",
    );
    require_md_contains(&report_md_raw, "Runtime real-world datapath probe error:");

    let md_generated = find_generated_at(&report_md_raw);
    if md_generated != generated_at {
        fail("ship readiness json guard: generated_at mismatch between json and markdown");
    }

    require_ordered_lines(
        &report_md_raw,
        &[
            "- CEF track report: `true`",
            "- CEF track guard: `true`",
            "- CEF track sync guard: `true`",
            "- CEF gap map guard: `true`",
            "- CEF consistency guard: `true`",
        ],
    );

    println!("ship readiness json guard: PASS");
}

fn read_file(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("ship readiness json guard: missing file: {path}")))
}

fn parse_json(raw: &str, msg: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| fail(msg))
}

fn require_str_eq(obj: &serde_json::Map<String, Value>, key: &str, expected: &str) {
    if obj.get(key).and_then(Value::as_str) != Some(expected) {
        fail(&format!("ship readiness json guard: {key} mismatch"));
    }
}

fn require_bool_eq(obj: &serde_json::Map<String, Value>, key: &str, expected: bool) {
    if obj.get(key).and_then(Value::as_bool) != Some(expected) {
        fail(&format!("ship readiness json guard: {key} mismatch"));
    }
}

fn require_step_true(root: &serde_json::Map<String, Value>, step: &str) {
    let steps = root
        .get("steps")
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail("ship readiness json guard: missing steps object"));
    if steps.get(step).and_then(Value::as_bool) != Some(true) {
        fail(&format!("ship readiness json guard: step not true: {step}"));
    }
}

fn require_ts_z(obj: &serde_json::Map<String, Value>, key: &str) -> String {
    let ts = obj.get(key).and_then(Value::as_str).unwrap_or("");
    if !(ts.len() == 20
        && ts.as_bytes()[4] == b'-'
        && ts.as_bytes()[7] == b'-'
        && ts.as_bytes()[10] == b'T'
        && ts.as_bytes()[13] == b':'
        && ts.as_bytes()[16] == b':'
        && ts.ends_with('Z'))
    {
        fail(&format!(
            "ship readiness json guard: invalid timestamp: {key}"
        ));
    }
    ts.to_owned()
}

fn require_md_contains(md: &str, needle: &str) {
    if !md.contains(needle) {
        fail(&format!(
            "ship readiness json guard: markdown missing: {needle}"
        ));
    }
}

fn find_generated_at(md: &str) -> String {
    let mut found: Option<String> = None;
    for line in md.lines() {
        if let Some(rest) = line.strip_prefix("Generated at (UTC): `")
            && let Some(ts) = rest.strip_suffix('`')
        {
            if found.is_some() {
                fail("ship readiness json guard: duplicate generated-at lines");
            }
            found = Some(ts.to_owned());
        }
    }
    found.unwrap_or_else(|| fail("ship readiness json guard: missing generated-at line"))
}

fn require_ordered_lines(md: &str, expected: &[&str]) {
    let mut prev = 0usize;
    for needle in expected {
        let line = md
            .lines()
            .enumerate()
            .find_map(|(idx, l)| if l == *needle { Some(idx + 1) } else { None })
            .unwrap_or_else(|| {
                fail(&format!(
                    "ship readiness json guard: missing line: {needle}"
                ))
            });
        if line <= prev {
            fail("ship readiness json guard: invalid CEF line order");
        }
        prev = line;
    }
}

fn validate_runtime_datapath_logic(
    report_obj: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    let total = report_obj
        .get("runtime_real_world_datapath_targets_total")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let ok = report_obj
        .get("runtime_real_world_datapath_targets_ok")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let failed = report_obj
        .get("runtime_real_world_datapath_targets_failed")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if ok + failed != total {
        return Err("ship readiness json guard: runtime real-world totals mismatch".to_string());
    }
    let attempted = report_obj
        .get("runtime_real_world_datapath_probe_attempted")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let ok_flag = report_obj
        .get("runtime_real_world_datapath_probe_ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let skipped_no_curl = report_obj
        .get("runtime_real_world_skipped_no_curl")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let probe_error = report_obj
        .get("runtime_real_world_datapath_probe_error")
        .and_then(Value::as_str)
        .unwrap_or("");

    if attempted && total <= 0 {
        return Err(
            "ship readiness json guard: datapath probe attempted with empty totals".to_string(),
        );
    }
    if !attempted && total != 0 {
        return Err(
            "ship readiness json guard: datapath probe not attempted with non-zero totals"
                .to_string(),
        );
    }
    if ok_flag && failed != 0 {
        return Err("ship readiness json guard: datapath probe ok with failed targets".to_string());
    }
    if skipped_no_curl && attempted {
        return Err("ship readiness json guard: no curl but datapath attempted".to_string());
    }
    if !skipped_no_curl && !attempted {
        return Err(
            "ship readiness json guard: datapath must be attempted when curl is available"
                .to_string(),
        );
    }
    if attempted && probe_error == "curl_not_found" {
        return Err(
            "ship readiness json guard: datapath attempted with curl_not_found".to_string(),
        );
    }
    if probe_error == "none" && !ok_flag {
        return Err("ship readiness json guard: datapath failed without error marker".to_string());
    }
    Ok(())
}

fn validate_direct_probe_visibility(
    report_obj: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    let direct_ok = report_obj
        .get("runtime_real_world_direct_probe_ok")
        .and_then(Value::as_bool)
        .ok_or_else(|| "ship readiness json guard: invalid direct probe field".to_string())?;
    let snapshot_ok = report_obj
        .get("runtime_real_world_probe_smoke_ok")
        .and_then(Value::as_bool)
        .ok_or_else(|| "ship readiness json guard: invalid snapshot smoke field".to_string())?;
    if !direct_ok && !snapshot_ok {
        return Err(
            "ship readiness json guard: direct probe failure must remain visible without failing snapshot integrity gate"
                .to_string(),
        );
    }
    Ok(())
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::{validate_direct_probe_visibility, validate_runtime_datapath_logic};
    use serde_json::{Map, Value, json};

    fn base_report_obj() -> serde_json::Map<String, Value> {
        let mut m = Map::new();
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
            "runtime_real_world_datapath_probe_attempted".to_string(),
            json!(true),
        );
        m.insert(
            "runtime_real_world_datapath_probe_ok".to_string(),
            json!(true),
        );
        m.insert(
            "runtime_real_world_skipped_no_curl".to_string(),
            json!(false),
        );
        m.insert(
            "runtime_real_world_datapath_probe_error".to_string(),
            json!("none"),
        );
        m
    }

    #[test]
    fn runtime_datapath_logic_accepts_valid_payload() {
        let payload = base_report_obj();
        assert!(validate_runtime_datapath_logic(&payload).is_ok());
    }

    #[test]
    fn runtime_datapath_logic_accepts_visible_direct_probe_failure() {
        let mut payload = base_report_obj();
        payload.insert(
            "runtime_real_world_direct_probe_ok".to_string(),
            json!(false),
        );
        assert!(validate_runtime_datapath_logic(&payload).is_ok());
    }

    #[test]
    fn runtime_datapath_logic_rejects_totals_mismatch() {
        let mut payload = base_report_obj();
        payload.insert(
            "runtime_real_world_datapath_targets_failed".to_string(),
            json!(1),
        );
        let res = validate_runtime_datapath_logic(&payload);
        assert!(res.is_err());
        assert!(res.err().is_some_and(|e| e.contains("totals mismatch")));
    }

    #[test]
    fn runtime_datapath_logic_rejects_not_attempted_without_skip_flag() {
        let mut payload = base_report_obj();
        payload.insert(
            "runtime_real_world_datapath_probe_attempted".to_string(),
            json!(false),
        );
        payload.insert(
            "runtime_real_world_datapath_targets_total".to_string(),
            json!(0),
        );
        payload.insert(
            "runtime_real_world_datapath_targets_ok".to_string(),
            json!(0),
        );
        let res = validate_runtime_datapath_logic(&payload);
        assert!(res.is_err());
        assert!(res.err().is_some_and(|e| e.contains("must be attempted")));
    }

    #[test]
    fn direct_probe_failure_is_allowed_when_snapshot_gate_is_still_visible() {
        let mut payload = Map::new();
        payload.insert(
            "runtime_real_world_direct_probe_ok".to_string(),
            json!(false),
        );
        payload.insert("runtime_real_world_probe_smoke_ok".to_string(), json!(true));

        assert!(validate_direct_probe_visibility(&payload).is_ok());
    }

    #[test]
    fn direct_probe_failure_rejects_hidden_snapshot_failure() {
        let mut payload = Map::new();
        payload.insert(
            "runtime_real_world_direct_probe_ok".to_string(),
            json!(false),
        );
        payload.insert(
            "runtime_real_world_probe_smoke_ok".to_string(),
            json!(false),
        );

        let res = validate_direct_probe_visibility(&payload);
        assert!(res.is_err());
        assert!(res.err().is_some_and(|e| e.contains("must remain visible")));
    }
}
