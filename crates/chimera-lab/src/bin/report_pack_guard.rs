#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let pack_json = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/REPORT_PACK.json");
    let pack_md = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("docs/REPORT_PACK.md");
    let reality_json = args
        .get(3)
        .map(String::as_str)
        .unwrap_or("docs/REALITY_AUDIT_LATEST.json");

    let pack = read_obj(pack_json);
    let md = read_text(pack_md);
    let reality = read_obj(reality_json);

    require_str(&pack, "status", "ok");
    require_str(&pack, "kind", "report_pack");
    for key in [
        "mvp_spec_report",
        "m5_artifacts_report",
        "m6_artifacts_report",
        "release_readiness_report",
        "cef_phase1_smoke",
        "mesh_route_explain",
    ] {
        require_bool(&pack, key, true);
    }
    let gate = get_obj(
        &pack,
        "release_gate",
        "report pack guard: missing release_gate",
    );
    require_bool(gate, "runtime_forced_stop_rollback_verified", true);
    require_str(&pack, "network_state", "not_modified");

    let expected_truth = serde_json::json!({
        "lab_scope_only": true,
        "real_world_datapath_closed": reality
            .get("real_world_datapath_closed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    if pack.get("truth_boundary") != Some(&expected_truth) {
        fail("report pack guard: truth_boundary mismatch");
    }

    require_md_line(&md, "# Report Pack");
    require_md_line(&md, "Status: **PASS**");
    require_md_line(&md, "Included reports:");
    require_md_line(&md, "Truth boundary:");
    require_md_line(
        &md,
        "Network safety: no OS route/DNS/firewall/proxy changes in this report path.",
    );

    let items = [
        "- MVP spec coverage: `true` (`docs/MVP_SPEC_COVERAGE.md`)",
        "- M5 artifacts: `true` (`docs/M5_ARTIFACTS_REPORT.md`)",
        "- M6 artifacts: `true` (`docs/M6_ARTIFACTS_REPORT.md`)",
        "- Release readiness: `true` (`docs/RELEASE_READINESS_REPORT.md`)",
        "- CEF phase1 smoke: `true` (`docs/CEF_PHASE1_SMOKE.json`)",
        "- Mesh route explain: `true` (`docs/MESH_ROUTE_EXPLAIN.json`)",
    ];
    require_strict_order(&md, &items);

    let line_included = line_no(&md, "Included reports:");
    let line_truth = line_no(&md, "Truth boundary:");
    let line_safety = line_no(
        &md,
        "Network safety: no OS route/DNS/firewall/proxy changes in this report path.",
    );
    if !(line_included < line_truth && line_truth < line_safety) {
        fail("report pack guard: section order mismatch");
    }

    println!("report pack guard: PASS");
}

fn read_text(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("report pack guard: missing file: {path}")))
}

fn read_obj(path: &str) -> serde_json::Map<String, Value> {
    let raw = read_text(path);
    let v: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail(&format!("report pack guard: invalid json: {path}")));
    v.as_object()
        .cloned()
        .unwrap_or_else(|| fail(&format!("report pack guard: root not object: {path}")))
}

fn require_str(obj: &serde_json::Map<String, Value>, key: &str, expected: &str) {
    if obj.get(key).and_then(Value::as_str) != Some(expected) {
        fail(&format!("report pack guard: {key} mismatch"));
    }
}

fn require_bool(obj: &serde_json::Map<String, Value>, key: &str, expected: bool) {
    if obj.get(key).and_then(Value::as_bool) != Some(expected) {
        fail(&format!("report pack guard: {key} mismatch"));
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

fn require_md_line(md: &str, expected: &str) {
    if md.lines().filter(|l| *l == expected).count() != 1 {
        fail(&format!(
            "report pack guard: markdown line mismatch: {expected}"
        ));
    }
}

fn line_no(md: &str, expected: &str) -> usize {
    md.lines()
        .enumerate()
        .find_map(|(i, l)| if l == expected { Some(i + 1) } else { None })
        .unwrap_or_else(|| {
            fail(&format!(
                "report pack guard: missing markdown line: {expected}"
            ))
        })
}

fn require_strict_order(md: &str, items: &[&str]) {
    let mut prev = 0usize;
    for item in items {
        let ln = line_no(md, item);
        if ln <= prev {
            fail("report pack guard: item order mismatch");
        }
        prev = ln;
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
