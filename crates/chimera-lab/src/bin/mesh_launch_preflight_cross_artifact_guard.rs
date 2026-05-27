#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        fail(
            "usage: mesh_launch_preflight_cross_artifact_guard <vps_json> <laptop_json> <verify_json>",
        );
    }
    let vps_path = args[1].as_str();
    let laptop_path = args[2].as_str();
    let verify_path = args[3].as_str();

    let vps = read_json(vps_path, "vps");
    let laptop = read_json(laptop_path, "laptop");
    let verify = read_json(verify_path, "verify");

    validate_cross(&vps, &laptop, &verify).unwrap_or_else(|msg| fail(&msg));
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

fn validate_cross(vps: &Value, laptop: &Value, verify: &Value) -> Result<(), String> {
    let vps_ns = vps["namespace"].as_str().unwrap_or("").trim();
    let laptop_ns = laptop["namespace"].as_str().unwrap_or("").trim();
    let verify_ns = verify["namespace"].as_str().unwrap_or("").trim();
    if vps_ns.is_empty() || laptop_ns.is_empty() || verify_ns.is_empty() {
        return Err("mesh launch preflight cross artifact guard: namespace missing".to_string());
    }
    if vps_ns != laptop_ns {
        return Err(
            "mesh launch preflight cross artifact guard: peer namespace mismatch".to_string(),
        );
    }
    if verify_ns != vps_ns {
        return Err(
            "mesh launch preflight cross artifact guard: verify namespace mismatch".to_string(),
        );
    }

    let vps_ready = is_peer_ready(vps);
    let laptop_ready = is_peer_ready(laptop);
    let verify_vps_ready = verify["vps_ready"].as_bool().ok_or_else(|| {
        "mesh launch preflight cross artifact guard: verify vps_ready missing".to_string()
    })?;
    let verify_laptop_ready = verify["laptop_ready"].as_bool().ok_or_else(|| {
        "mesh launch preflight cross artifact guard: verify laptop_ready missing".to_string()
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

    if verify_vps_ready != vps_ready {
        return Err(
            "mesh launch preflight cross artifact guard: verify vps_ready mismatch".to_string(),
        );
    }
    if verify_laptop_ready != laptop_ready {
        return Err(
            "mesh launch preflight cross artifact guard: verify laptop_ready mismatch".to_string(),
        );
    }
    let expected_all_ready = vps_ready && laptop_ready;
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
            "selected_peers":["n1"],
            "connected_peer":"n1",
            "connected_endpoint":"127.0.0.1:443",
            "connect_probe_success":true,
            "attempts":[{"peer_id":"n1","endpoint":"127.0.0.1:443","success":true,"error":""}],
            "explain":["ok"]
        })
    }

    #[test]
    fn accepts_consistent_ready_triplet() {
        let vps = ready_peer("node-a");
        let laptop = ready_peer("node-b");
        let verify = json!({
            "status":"ready",
            "all_ready":true,
            "vps_ready":true,
            "laptop_ready":true,
            "namespace":"cef-public",
            "network_state":"not_modified",
            "blockers":[]
        });
        assert!(validate_cross(&vps, &laptop, &verify).is_ok());
    }

    #[test]
    fn rejects_verify_namespace_mismatch() {
        let vps = ready_peer("node-a");
        let laptop = ready_peer("node-b");
        let verify = json!({
            "status":"ready",
            "all_ready":true,
            "vps_ready":true,
            "laptop_ready":true,
            "namespace":"cef-private",
            "network_state":"not_modified",
            "blockers":[]
        });
        let err = match validate_cross(&vps, &laptop, &verify) {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("verify namespace mismatch"));
    }

    #[test]
    fn rejects_verify_flag_mismatch() {
        let vps = ready_peer("node-a");
        let laptop = ready_peer("node-b");
        let verify = json!({
            "status":"ready",
            "all_ready":true,
            "vps_ready":false,
            "laptop_ready":true,
            "namespace":"cef-public",
            "network_state":"not_modified",
            "blockers":[]
        });
        let err = match validate_cross(&vps, &laptop, &verify) {
            Ok(()) => unreachable!("must fail"),
            Err(err) => err,
        };
        assert!(err.contains("verify vps_ready mismatch"));
    }
}
