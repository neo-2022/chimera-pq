#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json");

    let raw = fs::read_to_string(path).unwrap_or_else(|_| {
        fail(&format!(
            "mesh launch preflight verify guard: missing file: {path}"
        ))
    });
    let root: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail("mesh launch preflight verify guard: invalid json"));
    let obj = root.as_object().unwrap_or_else(|| {
        fail("mesh launch preflight verify guard: root not object");
    });

    validate_obj(obj).unwrap_or_else(|msg| fail(&msg));
    println!("mesh launch preflight verify guard: PASS");
}

fn validate_obj(obj: &serde_json::Map<String, Value>) -> Result<(), String> {
    let status = require_str(obj, "status")?;
    if status != "ready" && status != "blocked" {
        return Err("mesh launch preflight verify guard: status must be ready|blocked".to_string());
    }
    let network_state = require_str(obj, "network_state")?;
    if network_state != "not_modified" {
        return Err(
            "mesh launch preflight verify guard: network_state must be not_modified".to_string(),
        );
    }
    let namespace = require_str(obj, "namespace")?;
    if namespace.trim().is_empty() {
        return Err("mesh launch preflight verify guard: namespace is blank".to_string());
    }
    let all_ready = require_bool(obj, "all_ready")?;
    let vps_ready = require_bool(obj, "vps_ready")?;
    let laptop_ready = require_bool(obj, "laptop_ready")?;
    let blockers = obj
        .get("blockers")
        .and_then(Value::as_array)
        .ok_or_else(|| "mesh launch preflight verify guard: blockers missing".to_string())?;
    for blocker in blockers {
        if blocker.as_str().unwrap_or("").trim().is_empty() {
            return Err(
                "mesh launch preflight verify guard: blockers contains blank entry".to_string(),
            );
        }
    }

    if all_ready {
        if status != "ready" {
            return Err(
                "mesh launch preflight verify guard: all_ready=true requires status=ready"
                    .to_string(),
            );
        }
        if !vps_ready || !laptop_ready {
            return Err(
                "mesh launch preflight verify guard: all_ready=true requires both peer flags true"
                    .to_string(),
            );
        }
        if !blockers.is_empty() {
            return Err(
                "mesh launch preflight verify guard: all_ready=true requires empty blockers"
                    .to_string(),
            );
        }
    } else {
        if status != "blocked" {
            return Err(
                "mesh launch preflight verify guard: all_ready=false requires status=blocked"
                    .to_string(),
            );
        }
        if blockers.is_empty() {
            return Err(
                "mesh launch preflight verify guard: blocked status requires blockers".to_string(),
            );
        }
    }
    Ok(())
}

fn require_str<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> Result<&'a str, String> {
    obj.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("mesh launch preflight verify guard: missing field: {key}"))
}

fn require_bool(obj: &serde_json::Map<String, Value>, key: &str) -> Result<bool, String> {
    obj.get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("mesh launch preflight verify guard: missing field: {key}"))
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::validate_obj;
    use serde_json::{Map, Value};

    fn base_ready() -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("status".into(), Value::String("ready".into()));
        m.insert("all_ready".into(), Value::Bool(true));
        m.insert("vps_ready".into(), Value::Bool(true));
        m.insert("laptop_ready".into(), Value::Bool(true));
        m.insert("namespace".into(), Value::String("cef-public".into()));
        m.insert("network_state".into(), Value::String("not_modified".into()));
        m.insert("blockers".into(), Value::Array(vec![]));
        m
    }

    #[test]
    fn validate_accepts_ready_contract() {
        let m = base_ready();
        assert!(validate_obj(&m).is_ok());
    }

    #[test]
    fn validate_rejects_ready_with_blockers() {
        let mut m = base_ready();
        m.insert(
            "blockers".into(),
            Value::Array(vec![Value::String("namespace_mismatch".into())]),
        );
        let err = match validate_obj(&m) {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("all_ready=true requires empty blockers"));
    }

    #[test]
    fn validate_rejects_blocked_without_blockers() {
        let mut m = base_ready();
        m.insert("status".into(), Value::String("blocked".into()));
        m.insert("all_ready".into(), Value::Bool(false));
        m.insert("vps_ready".into(), Value::Bool(false));
        m.insert("laptop_ready".into(), Value::Bool(true));
        let err = match validate_obj(&m) {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("blocked status requires blockers"));
    }
}
