#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let json = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/CEF_TRACK_REPORT.json");
    let md = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("docs/CEF_TRACK_REPORT.md");

    let data = read_obj(json);
    let md_text = read_text(md);

    require_str(&data, "kind", "cef_track_report");
    let truth = data
        .get("truth_boundary")
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail("cef track guard: missing truth_boundary"));
    if truth.get("mvp_lab_ready").and_then(Value::as_bool) != Some(true)
        || truth.get("full_cef_closed").and_then(Value::as_bool) != Some(false)
    {
        fail("cef track guard: truth boundary mismatch");
    }
    if data.get("phase1_closed").and_then(Value::as_bool).is_none() {
        fail("cef track guard: phase1_closed missing");
    }

    let blocks = [
        "mesh_runtime",
        "dht_discovery",
        "distributed_policy_store",
        "cooperative_relay_model",
        "emergency_oob_carriers",
        "roaming_cache",
        "reputation_complaint_credit",
    ];
    let blocks_obj = data
        .get("blocks")
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail("cef track guard: missing blocks object"));

    let mut prev_line = 0usize;
    for block in blocks {
        let v = blocks_obj
            .get(block)
            .and_then(Value::as_object)
            .unwrap_or_else(|| fail("cef track guard: missing block"));
        for k in ["implemented", "code", "api", "tests"] {
            if v.get(k).and_then(Value::as_bool) != Some(true) {
                fail("cef track guard: block bool mismatch");
            }
        }
        let runtime_wired = v
            .get("runtime_wired")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| fail("cef track guard: runtime_wired missing"));
        let gate_status = v
            .get("gate_status")
            .and_then(Value::as_str)
            .unwrap_or_else(|| fail("cef track guard: gate_status missing"));
        let expected = if runtime_wired { "ready" } else { "partial" };
        if gate_status != expected {
            fail("cef track guard: gate_status mismatch");
        }

        let line = find_line(&md_text, &format!("- {block}: "))
            .unwrap_or_else(|| fail("cef track guard: markdown block line missing"));
        if line <= prev_line {
            fail("cef track guard: markdown block order mismatch");
        }
        prev_line = line;

        let checks_line = get_line(&md_text, line + 1);
        let runtime_line = get_line(&md_text, line + 2);
        if checks_line != "  checks: code=`true`, api=`true`, tests=`true`" {
            fail("cef track guard: checks line mismatch");
        }
        if !runtime_line.starts_with("  runtime_wired: `")
            || !runtime_line.contains("`, gate_status: `")
        {
            fail("cef track guard: runtime line shape mismatch");
        }
    }

    require_md_contains(&md_text, "Status: **PARTIAL / NOT CLOSED**");
    require_md_contains(&md_text, "Full CEF closed: `false`");
    require_md_contains(&md_text, "Phase-1 closed: `");
    require_md_contains(&md_text, "runtime_wired: `");

    println!("cef track guard: PASS");
}

fn read_text(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| fail(&format!("missing artifact: {path}")))
}
fn read_obj(path: &str) -> serde_json::Map<String, Value> {
    let raw = read_text(path);
    let v: Value =
        serde_json::from_str(&raw).unwrap_or_else(|_| fail("cef track guard: invalid json"));
    v.as_object()
        .cloned()
        .unwrap_or_else(|| fail("cef track guard: root not object"))
}
fn require_str(obj: &serde_json::Map<String, Value>, key: &str, expected: &str) {
    if obj.get(key).and_then(Value::as_str) != Some(expected) {
        fail("cef track guard: field mismatch");
    }
}
fn find_line(md: &str, prefix: &str) -> Option<usize> {
    md.lines().enumerate().find_map(|(i, l)| {
        if l.starts_with(prefix) {
            Some(i + 1)
        } else {
            None
        }
    })
}
fn get_line(md: &str, line_no: usize) -> &str {
    md.lines().nth(line_no.saturating_sub(1)).unwrap_or("")
}
fn require_md_contains(md: &str, needle: &str) {
    if !md.contains(needle) {
        fail("cef track guard: markdown missing expected fragment");
    }
}
fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
