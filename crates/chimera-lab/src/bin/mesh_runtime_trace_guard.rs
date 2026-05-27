#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/MESH_RUNTIME_TRACE.json");

    let raw = fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("mesh runtime trace guard: missing file: {path}")));
    let root: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail("mesh runtime trace guard: invalid json"));
    let obj = root
        .as_object()
        .unwrap_or_else(|| fail("mesh runtime trace guard: root not object"));

    require_str(obj, "status", "ok");
    require_str(obj, "kind", "mesh_runtime_trace");
    require_str(obj, "namespace", "cef-public");
    require_str(obj, "join_mode", "InvitationOnly");

    let phases = obj
        .get("phases")
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail("mesh runtime trace guard: missing phases"));

    require_phase_peer(phases, "initial", "node-eu-1");
    require_phase_peer(phases, "failover", "node-eu-2");
    require_phase_peer(phases, "reselection", "node-eu-2");
    require_phase_peer(phases, "persisted_state_reselection", "node-eu-2");

    require_phase_explain_contains(phases, "failover", "failover_triggered");
    require_phase_explain_contains(phases, "reselection", "health_reselection_applied=");
    require_phase_explain_contains(
        phases,
        "persisted_state_reselection",
        "health_reselection_applied=",
    );

    println!("mesh runtime trace guard: PASS");
}

fn require_phase_peer(phases: &serde_json::Map<String, Value>, phase: &str, expected_peer: &str) {
    let phase_obj = phases
        .get(phase)
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail("mesh runtime trace guard: missing phase object"));
    let selected = phase_obj
        .get("selected_peer")
        .and_then(Value::as_str)
        .unwrap_or_else(|| fail("mesh runtime trace guard: missing selected_peer"));
    if selected != expected_peer {
        fail("mesh runtime trace guard: selected_peer mismatch");
    }
}

fn require_phase_explain_contains(
    phases: &serde_json::Map<String, Value>,
    phase: &str,
    needle: &str,
) {
    let phase_obj = phases
        .get(phase)
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail("mesh runtime trace guard: missing phase object"));
    let explain = phase_obj
        .get("explain")
        .and_then(Value::as_array)
        .unwrap_or_else(|| fail("mesh runtime trace guard: missing explain array"));
    let found = explain
        .iter()
        .filter_map(Value::as_str)
        .any(|line| line.contains(needle));
    if !found {
        fail("mesh runtime trace guard: explain marker missing");
    }
}

fn require_str(obj: &serde_json::Map<String, Value>, key: &str, expected: &str) {
    if obj.get(key).and_then(Value::as_str) != Some(expected) {
        fail("mesh runtime trace guard: field mismatch");
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
