#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/MESH_ROUTE_EXPLAIN.json");

    let raw = fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("mesh route explain guard: missing file: {path}")));
    let root: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail("mesh route explain guard: invalid json"));
    let obj = root
        .as_object()
        .unwrap_or_else(|| fail("mesh route explain guard: root not object"));

    require_str(obj, "status", "ok");
    require_str(obj, "kind", "mesh_route_explain");
    require_str(obj, "namespace", "cef-public");
    require_str(obj, "node", "node-client");
    require_str(obj, "join_mode", "InvitationOnly");
    require_str(obj, "initial_selected_peer", "node-eu-1");
    require_str(obj, "failover_selected_peer", "node-eu-2");
    require_str(obj, "cooldown_selected_peer", "node-eu-1");
    require_str(obj, "network_state", "not_modified");

    let explain = obj
        .get("explain")
        .and_then(Value::as_str)
        .unwrap_or_else(|| fail("mesh route explain guard: explain missing"));
    for needle in ["join_mode=InvitationOnly", "selected_peers=1"] {
        if !explain.contains(needle) {
            fail("mesh route explain guard: explain content mismatch");
        }
    }

    println!("mesh route explain guard: PASS");
}

fn require_str(obj: &serde_json::Map<String, Value>, key: &str, expected: &str) {
    if obj.get(key).and_then(Value::as_str) != Some(expected) {
        fail(&format!("mesh route explain guard: field mismatch: {key}"));
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
