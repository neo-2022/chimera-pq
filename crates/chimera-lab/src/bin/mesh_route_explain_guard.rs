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
    require_str(obj, "node", "<redacted>");
    require_str(obj, "join_mode", "InvitationOnly");
    require_str(obj, "initial_selected_peer", "peer#1");
    require_str(obj, "failover_selected_peer", "peer#1");
    require_str(obj, "cooldown_selected_peer", "peer#1");
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
    for forbidden in [
        "selected_peer_ids=node-",
        "selected_peer_endpoints=198.51.",
        "selected_peer_connect_priority=1:node-",
        "selected_peer_connect_retry_plan=node-",
        "effective_health_blocked_node_ids=node-",
    ] {
        if explain.contains(forbidden) {
            fail("mesh route explain guard: raw peer identity leaked");
        }
    }
    for (key, value) in obj {
        require_no_raw_public_value(key, value);
    }

    println!("mesh route explain guard: PASS");
}

fn require_no_raw_public_value(key: &str, value: &Value) {
    match value {
        Value::String(text) => {
            if key == "namespace" {
                return;
            }
            require_no_raw_public_text(text);
        }
        Value::Array(values) => {
            for item in values {
                require_no_raw_public_value(key, item);
            }
        }
        Value::Object(map) => {
            for (child_key, child_value) in map {
                require_no_raw_public_value(child_key, child_value);
            }
        }
        _ => {}
    }
}

fn require_no_raw_public_text(text: &str) {
    if text.contains("node-")
        || text.contains("node_id=")
        || text.contains("peer_id=")
        || text.contains("ports=443")
        || text.contains("ports=8443")
        || text.contains("127.0.0.1")
        || text.contains("192.168.")
        || text.contains("91.124.")
        || text.contains("198.51.")
        || text.contains("203.0.113.")
        || text_contains_socket_hint(text)
    {
        fail("mesh route explain guard: raw public diagnostic leak");
    }
}

fn text_contains_socket_hint(text: &str) -> bool {
    text.split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '"' | '\'' | '[' | ']'))
        .any(|token| {
            token.rsplit_once(':').is_some_and(|(host, port)| {
                port.parse::<u16>().is_ok()
                    && (host.contains('.')
                        || host.contains('[')
                        || host.contains(']')
                        || host == "localhost")
            })
        })
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
