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
    let track_md = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("docs/CEF_TRACK_REPORT.md");
    let gap_md = args
        .get(3)
        .map(String::as_str)
        .unwrap_or("docs/CEF_GAP_MAP_2026-05-18.md");

    let data = read_obj(json);
    let track = read_text(track_md);
    let gap = read_text(gap_md);

    require_str(&track, "Status: **PARTIAL / NOT CLOSED**");
    require_str(
        &gap,
        "- Full CEF contour from `CHIMERA.pdf`: PARTIAL / NOT CLOSED.",
    );
    if data
        .get("truth_boundary")
        .and_then(Value::as_object)
        .and_then(|o| o.get("full_cef_closed").and_then(Value::as_bool))
        != Some(false)
    {
        fail("cef consistency guard: full_cef_closed mismatch");
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
        .unwrap_or_else(|| fail("cef consistency guard: missing blocks"));

    let mut prev = 0usize;
    let mut runtime_all_true = true;
    for b in blocks {
        let ln = find_line(&track, &format!("- {b}: "))
            .unwrap_or_else(|| fail("cef consistency guard: missing block line"));
        if ln <= prev {
            fail("cef consistency guard: block order mismatch");
        }
        prev = ln;
        let v = blocks_obj
            .get(b)
            .and_then(Value::as_object)
            .unwrap_or_else(|| fail("cef consistency guard: missing block json"));
        for k in ["implemented", "code", "api", "tests"] {
            if v.get(k).and_then(Value::as_bool) != Some(true) {
                fail("cef consistency guard: block bool mismatch");
            }
        }
        let wired = v
            .get("runtime_wired")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| fail("cef consistency guard: runtime_wired missing"));
        runtime_all_true &= wired;
        let expected = if wired { "ready" } else { "partial" };
        if v.get("gate_status").and_then(Value::as_str) != Some(expected) {
            fail("cef consistency guard: gate_status mismatch");
        }
    }
    if data.get("phase1_closed").and_then(Value::as_bool) != Some(runtime_all_true) {
        fail("cef consistency guard: phase1_closed mismatch");
    }

    let status_count = gap
        .lines()
        .filter(|l| {
            *l == "- Status: implemented for Phase-1 track (runtime wired), Full CEF not closed."
        })
        .count();
    if status_count != 7 {
        fail("cef consistency guard: gap status count mismatch");
    }
    for title in [
        "1. Full cooperative mesh runtime",
        "2. DHT discovery (public/private) and provider records",
        "3. Distributed Policy Store (DPS)",
        "4. Cooperative relay participation/consent model",
        "5. Emergency/OOB carriers",
        "6. Roaming cache / distributed bootstrap continuation",
        "7. Reputation / complaint / relay credit subsystems",
    ] {
        if gap.lines().filter(|l| *l == title).count() != 1 {
            fail("cef consistency guard: gap section title mismatch");
        }
    }
    require_str(&gap, "## Phase-1 Closure Snapshot");
    require_str(
        &gap,
        "- `phase1_closed` in `docs/CEF_TRACK_REPORT.json`: `true`.",
    );
    require_str(
        &gap,
        "- This does **not** mean Full CEF closure; `full_cef_closed` remains `false`.",
    );

    println!("cef consistency guard: PASS");
}

fn read_text(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("cef consistency guard: missing file: {path}")))
}
fn read_obj(path: &str) -> serde_json::Map<String, Value> {
    let raw = read_text(path);
    let v: Value =
        serde_json::from_str(&raw).unwrap_or_else(|_| fail("cef consistency guard: invalid json"));
    v.as_object()
        .cloned()
        .unwrap_or_else(|| fail("cef consistency guard: root not object"))
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
fn require_str(md: &str, line: &str) {
    if !md.lines().any(|l| l == line) {
        fail("cef consistency guard: missing expected line");
    }
}
fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
