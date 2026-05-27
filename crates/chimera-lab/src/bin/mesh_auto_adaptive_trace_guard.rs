#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/MESH_AUTO_ADAPTIVE_TRACE.json");

    let raw = fs::read_to_string(path).unwrap_or_else(|_| {
        fail(&format!(
            "mesh auto adaptive trace guard: missing file: {path}"
        ))
    });
    let root: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail("mesh auto adaptive trace guard: invalid json"));
    let obj = root
        .as_object()
        .unwrap_or_else(|| fail("mesh auto adaptive trace guard: root not object"));

    require_str(obj, "status", "ok");
    require_str(obj, "kind", "mesh_auto_adaptive_trace");
    require_str(obj, "namespace", "cef-public");
    require_str(obj, "network_state", "not_modified");

    let plans = obj
        .get("plans")
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail("mesh auto adaptive trace guard: missing plans"));

    require_phase_and_marker(
        plans,
        "auto_baseline",
        "path_profile_reason=auto:fast_signals",
    );
    require_phase_and_marker(
        plans,
        "auto_degraded",
        "path_profile_reason=auto:degraded_active",
    );
    require_phase_and_marker(
        plans,
        "manual_override",
        "effective_filter_source=manual_override",
    );

    println!("mesh auto adaptive trace guard: PASS");
}

fn require_phase_and_marker(plans: &serde_json::Map<String, Value>, phase: &str, marker: &str) {
    let phase_obj = plans
        .get(phase)
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail("mesh auto adaptive trace guard: missing phase"));
    let selected_peer = phase_obj
        .get("selected_peer")
        .and_then(Value::as_str)
        .unwrap_or("");
    if selected_peer.is_empty() {
        fail("mesh auto adaptive trace guard: selected_peer empty");
    }
    let explain = phase_obj
        .get("explain")
        .and_then(Value::as_array)
        .unwrap_or_else(|| fail("mesh auto adaptive trace guard: missing explain"));
    if !explain.iter().any(|entry| {
        entry
            .as_str()
            .map(|text| text.contains(marker))
            .unwrap_or(false)
    }) {
        fail("mesh auto adaptive trace guard: explain marker missing");
    }
}

fn require_str(obj: &serde_json::Map<String, Value>, key: &str, expected: &str) {
    if obj.get(key).and_then(Value::as_str) != Some(expected) {
        fail(&format!(
            "mesh auto adaptive trace guard: field mismatch: {key}"
        ));
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
