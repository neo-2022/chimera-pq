#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        fail(
            "usage: mesh_launch_preflight_cross_artifact_guard <side_a_json> <side_b_json> <verify_json>",
        );
    }
    let side_a_path = args[1].as_str();
    let side_b_path = args[2].as_str();
    let verify_path = args[3].as_str();

    let side_a = read_json(side_a_path, "side_a");
    let side_b = read_json(side_b_path, "side_b");
    let verify = read_json(verify_path, "verify");

    validate_cross(&side_a, &side_b, &verify).unwrap_or_else(|msg| fail(&msg));
    println!("mesh launch preflight cross artifact guard: PASS");
}

fn read_json(path: &str, kind: &str) -> Value {
    let raw = fs::read_to_string(path).unwrap_or_else(|_| {
        fail(&format!(
            "mesh launch preflight cross artifact guard: missing {kind} file: {path}"
        ))
    });
    serde_json::from_str(&raw).unwrap_or_else(|_| {
        fail(&format!(
            "mesh launch preflight cross artifact guard: invalid {kind} json: {path}"
        ))
    })
}

fn is_peer_ready(v: &Value) -> bool {
    v["status"].as_str() == Some("ready")
        && v["ready_for_real_launch"].as_bool() == Some(true)
        && v["connect_probe_success"].as_bool() == Some(true)
        && v["network_state"].as_str() == Some("not_modified")
        && v["blockers"].as_array().is_some_and(|arr| arr.is_empty())
}

fn validate_cross(side_a: &Value, side_b: &Value, verify: &Value) -> Result<(), String> {
    let side_a_ns = side_a["namespace"].as_str().unwrap_or("").trim();
    let side_b_ns = side_b["namespace"].as_str().unwrap_or("").trim();
    let verify_ns = verify["namespace"].as_str().unwrap_or("").trim();
    if side_a_ns.is_empty() || side_b_ns.is_empty() || verify_ns.is_empty() {
        return Err("mesh launch preflight cross artifact guard: namespace missing".to_string());
    }
    if side_a_ns != side_b_ns {
        return Err(
            "mesh launch preflight cross artifact guard: peer namespace mismatch".to_string(),
        );
    }
    if verify_ns != side_a_ns {
        return Err(
            "mesh launch preflight cross artifact guard: verify namespace mismatch".to_string(),
        );
    }

    let side_a_ready = is_peer_ready(side_a);
    let side_b_ready = is_peer_ready(side_b);
    let verify_side_a_ready = verify["side_a_ready"].as_bool().ok_or_else(|| {
        "mesh launch preflight cross artifact guard: verify side_a_ready missing".to_string()
    })?;
    let verify_side_b_ready = verify["side_b_ready"].as_bool().ok_or_else(|| {
        "mesh launch preflight cross artifact guard: verify side_b_ready missing".to_string()
    })?;
    let verify_all_ready = verify["all_ready"].as_bool().ok_or_else(|| {
        "mesh launch preflight cross artifact guard: verify all_ready missing".to_string()
    })?;
    let verify_status = verify["status"].as_str().ok_or_else(|| {
        "mesh launch preflight cross artifact guard: verify status missing".to_string()
    })?;
    if verify_status != "ready" && verify_status != "blocked" {
        return Err(
            "mesh launch preflight cross artifact guard: verify status must be ready|blocked"
                .to_string(),
        );
    }

    if verify_side_a_ready != side_a_ready {
        return Err(
            "mesh launch preflight cross artifact guard: verify side_a_ready mismatch".to_string(),
        );
    }
    if verify_side_b_ready != side_b_ready {
        return Err(
            "mesh launch preflight cross artifact guard: verify side_b_ready mismatch".to_string(),
        );
    }
    let expected_all_ready = side_a_ready && side_b_ready;
    if verify_all_ready != expected_all_ready {
        return Err(
            "mesh launch preflight cross artifact guard: verify all_ready mismatch".to_string(),
        );
    }
    if verify_status == "ready" && !expected_all_ready {
        return Err(
            "mesh launch preflight cross artifact guard: ready verify requires both peers ready"
                .to_string(),
        );
    }
    if verify_status == "blocked" && expected_all_ready {
        return Err(
            "mesh launch preflight cross artifact guard: blocked verify with both peers ready"
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
    use super::validate_cross;
    use serde_json::{Value, json};

    fn ready_peer(node: &str) -> Value {
        json!({
            "status":"ready",
            "network_state":"not_modified",
            "namespace":"cef-public",
            "node":node,
            "timeout_ms":1200,
            "ready_for_real_launch":true,
            "blockers":[],
            "selected_peers":["peer#1"],
            "connected_peer":"peer#1",
            "connected_endpoint":"endpoint#1:<redacted>",
            "connect_probe_success":true,
            "attempts":[{"peer_id":"peer#1","endpoint":"endpoint#1:<redacted>","success":true,"error":""}],
            "explain":["ok"]
        })
    }

    #[test]
    fn accepts_consistent_ready_triplet() {
        let side_a = ready_peer("node-a");
        let side_b = ready_peer("node-b");
        let verify = json!({
            "status":"ready",
            "all_ready":true,
            "side_a_ready":true,
            "side_b_ready":true,
            "namespace":"cef-public",
            "network_state":"not_modified",
            "blockers":[]
        });
        assert!(validate_cross(&side_a, &side_b, &verify).is_ok());
    }

    #[test]
    fn rejects_verify_namespace_mismatch() {
        let side_a = ready_peer("node-a");
        let side_b = ready_peer("node-b");
        let verify = json!({
            "status":"ready",
            "all_ready":true,
            "side_a_ready":true,
            "side_b_ready":true,
            "namespace":"cef-private",
            "network_state":"not_modified",
            "blockers":[]
        });
        let err = match validate_cross(&side_a, &side_b, &verify) {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("verify namespace mismatch"));
    }

    #[test]
    fn rejects_verify_flag_mismatch() {
        let side_a = ready_peer("node-a");
        let side_b = ready_peer("node-b");
        let verify = json!({
            "status":"ready",
            "all_ready":true,
            "side_a_ready":false,
            "side_b_ready":true,
            "namespace":"cef-public",
            "network_state":"not_modified",
            "blockers":[]
        });
        let err = match validate_cross(&side_a, &side_b, &verify) {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("verify side_a_ready mismatch"));
    }
}
