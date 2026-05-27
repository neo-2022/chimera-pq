#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/MESH_LAUNCH_PREFLIGHT_VPS.json");
    let expected_role = args.get(2).map(String::as_str).unwrap_or("side_a");

    if expected_role != "side_a" && expected_role != "side_b" {
        fail("mesh launch preflight report guard: expected role must be side_a|side_b");
    }

    let raw = fs::read_to_string(path).unwrap_or_else(|_| {
        fail(&format!(
            "mesh launch preflight report guard: missing file: {path}"
        ))
    });
    let root: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail("mesh launch preflight report guard: invalid json"));
    let obj = root
        .as_object()
        .unwrap_or_else(|| fail("mesh launch preflight report guard: root not object"));

    validate_obj(obj, expected_role).unwrap_or_else(|msg| fail(&msg));
    println!("mesh launch preflight report guard: PASS");
}

fn validate_obj(obj: &serde_json::Map<String, Value>, expected_role: &str) -> Result<(), String> {
    let status = require_str(obj, "status")?;
    if status != "ready" && status != "blocked" {
        return Err("mesh launch preflight report guard: status must be ready|blocked".to_string());
    }
    let network_state = require_str(obj, "network_state")?;
    if network_state != "not_modified" {
        return Err(
            "mesh launch preflight report guard: network_state must be not_modified".to_string(),
        );
    }
    let namespace = require_str(obj, "namespace")?;
    if namespace.trim().is_empty() {
        return Err("mesh launch preflight report guard: namespace is blank".to_string());
    }
    let node = require_str(obj, "node")?;
    if node.trim().is_empty() {
        return Err("mesh launch preflight report guard: node is blank".to_string());
    }
    if expected_role != "side_a" && expected_role != "side_b" {
        return Err(
            "mesh launch preflight report guard: expected role must be side_a|side_b".to_string(),
        );
    }
    let timeout_ms = require_u64(obj, "timeout_ms")?;
    if timeout_ms == 0 {
        return Err("mesh launch preflight report guard: timeout_ms must be > 0".to_string());
    }
    let ready = require_bool(obj, "ready_for_real_launch")?;
    let success = require_bool(obj, "connect_probe_success")?;
    if ready != success {
        return Err(
            "mesh launch preflight report guard: ready_for_real_launch must match connect_probe_success"
                .to_string(),
        );
    }
    let blockers = require_array(obj, "blockers")?;
    for blocker in blockers {
        if blocker.as_str().unwrap_or("").trim().is_empty() {
            return Err(
                "mesh launch preflight report guard: blockers contains blank entry".to_string(),
            );
        }
    }
    if ready {
        if !blockers.is_empty() {
            return Err(
                "mesh launch preflight report guard: ready report requires empty blockers"
                    .to_string(),
            );
        }
    } else {
        if blockers.is_empty() {
            return Err(
                "mesh launch preflight report guard: blocked report requires blockers".to_string(),
            );
        }
        if !blockers
            .iter()
            .any(|v| v.as_str().unwrap_or("") == "connectivity_probe_failed")
        {
            return Err("mesh launch preflight report guard: blocked report must include connectivity_probe_failed".to_string());
        }
    }
    let _ = require_array(obj, "selected_peers")?;
    let _ = require_array(obj, "attempts")?;
    let _ = require_array(obj, "explain")?;
    let _ = require_str(obj, "connected_peer")?;
    let _ = require_str(obj, "connected_endpoint")?;
    Ok(())
}

fn require_str<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> Result<&'a str, String> {
    obj.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("mesh launch preflight report guard: missing field: {key}"))
}

fn require_bool(obj: &serde_json::Map<String, Value>, key: &str) -> Result<bool, String> {
    obj.get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("mesh launch preflight report guard: missing field: {key}"))
}

fn require_u64(obj: &serde_json::Map<String, Value>, key: &str) -> Result<u64, String> {
    obj.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("mesh launch preflight report guard: missing field: {key}"))
}

fn require_array<'a>(
    obj: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a [Value], String> {
    obj.get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("mesh launch preflight report guard: missing field: {key}"))
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::validate_obj;
    use serde_json::{Map, Value};

    fn base_ready(node: &str) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("status".into(), Value::String("ready".into()));
        m.insert("network_state".into(), Value::String("not_modified".into()));
        m.insert("namespace".into(), Value::String("cef-public".into()));
        m.insert("node".into(), Value::String(node.to_string()));
        m.insert("timeout_ms".into(), Value::from(1200_u64));
        m.insert("ready_for_real_launch".into(), Value::Bool(true));
        m.insert("connect_probe_success".into(), Value::Bool(true));
        m.insert("blockers".into(), Value::Array(vec![]));
        m.insert(
            "selected_peers".into(),
            Value::Array(vec![Value::String("n1".into())]),
        );
        m.insert(
            "attempts".into(),
            Value::Array(vec![serde_json::json!({
                "peer_id":"n1",
                "endpoint":"127.0.0.1:443",
                "success":true,
                "error":""
            })]),
        );
        m.insert(
            "explain".into(),
            Value::Array(vec![Value::String("ok".into())]),
        );
        m.insert("connected_peer".into(), Value::String("n1".into()));
        m.insert(
            "connected_endpoint".into(),
            Value::String("127.0.0.1:443".into()),
        );
        m
    }

    #[test]
    fn validate_accepts_ready_vps_report() {
        let m = base_ready("node-a");
        assert!(validate_obj(&m, "side_a").is_ok());
    }

    #[test]
    fn validate_accepts_any_non_empty_node_for_role() {
        let m = base_ready("custom-node-alpha");
        assert!(validate_obj(&m, "side_a").is_ok());
    }

    #[test]
    fn validate_rejects_unknown_role() {
        let m = base_ready("custom-node-alpha");
        let err = match validate_obj(&m, "desktop") {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("expected role must be side_a|side_b"));
    }

    #[test]
    fn validate_rejects_blocked_without_connectivity_blocker() {
        let mut m = base_ready("node-b");
        m.insert("status".into(), Value::String("blocked".into()));
        m.insert("ready_for_real_launch".into(), Value::Bool(false));
        m.insert("connect_probe_success".into(), Value::Bool(false));
        m.insert(
            "blockers".into(),
            Value::Array(vec![Value::String("namespace_mismatch".into())]),
        );
        let err = match validate_obj(&m, "side_b") {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("must include connectivity_probe_failed"));
    }
}
