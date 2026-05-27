#![forbid(unsafe_code)]

use std::net::TcpListener;
use std::thread;

use super::tests_json_runner_utils::run_mesh_subcommand_json;

#[test]
fn connect_probe_json_contract_failure_shape() {
    let parsed = run_mesh_subcommand_json(
        "connect-probe",
        vec![
            "--namespace".to_string(),
            "cef-public".to_string(),
            "--node".to_string(),
            "node-client".to_string(),
            "--policy-payload".to_string(),
            "target_region=eu;max_peers=1;mesh_connect_fallback_ports=65080,65081".to_string(),
            "--peer".to_string(),
            "n1@127.0.0.1:1@eu@20@90".to_string(),
            "--timeout-ms".to_string(),
            "25".to_string(),
        ],
        1,
        "connect_probe_fail_shape",
    );

    assert_eq!(parsed["namespace"].as_str().unwrap_or(""), "cef-public");
    assert_eq!(parsed["success"].as_bool(), Some(false));
    assert!(parsed["connected_peer"].as_str().unwrap_or("").is_empty());
    assert!(
        parsed["connected_endpoint"]
            .as_str()
            .unwrap_or("")
            .is_empty()
    );
    assert!(parsed["selected_peers"].is_array());
    assert!(parsed["attempts"].is_array());
    assert!(parsed["explain"].is_array());
    let attempts = parsed["attempts"]
        .as_array()
        .unwrap_or_else(|| unreachable!("attempts must be array"));
    assert!(!attempts.is_empty());
    for attempt in attempts {
        assert!(attempt["peer_id"].as_str().is_some());
        assert!(attempt["endpoint"].as_str().is_some());
        assert_eq!(attempt["success"].as_bool(), Some(false));
        assert!(!attempt["error"].as_str().unwrap_or("").is_empty());
    }
}

#[test]
fn connect_probe_json_contract_success_shape() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|e| unreachable!("listener bind should work: {e}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|e| unreachable!("listener local_addr should work: {e}"));
    let _accept_thread = thread::spawn(move || {
        let _ = listener.accept();
    });
    let peer = format!("n1@127.0.0.1:{}@eu@20@90", addr.port());
    let parsed = run_mesh_subcommand_json(
        "connect-probe",
        vec![
            "--namespace".to_string(),
            "cef-public".to_string(),
            "--node".to_string(),
            "node-client".to_string(),
            "--policy-payload".to_string(),
            "target_region=eu;max_peers=1;mesh_connect_fallback_ports=65080,65081".to_string(),
            "--peer".to_string(),
            peer,
            "--timeout-ms".to_string(),
            "500".to_string(),
        ],
        0,
        "connect_probe_success_shape",
    );

    assert_eq!(parsed["namespace"].as_str().unwrap_or(""), "cef-public");
    assert_eq!(parsed["success"].as_bool(), Some(true));
    assert_eq!(parsed["connected_peer"].as_str().unwrap_or(""), "n1");
    assert_eq!(
        parsed["connected_endpoint"].as_str().unwrap_or(""),
        format!("127.0.0.1:{}", addr.port())
    );
    assert!(parsed["selected_peers"].is_array());
    assert!(parsed["attempts"].is_array());
    assert!(parsed["explain"].is_array());
    let attempts = parsed["attempts"]
        .as_array()
        .unwrap_or_else(|| unreachable!("attempts must be array"));
    assert!(!attempts.is_empty());
    assert!(
        attempts
            .iter()
            .any(|attempt| attempt["success"].as_bool() == Some(true))
    );
}

#[test]
fn connect_probe_json_rejects_duplicate_peer_node_ids() {
    let parsed = run_mesh_subcommand_json(
        "connect-probe",
        vec![
            "--namespace".to_string(),
            "cef-public".to_string(),
            "--node".to_string(),
            "node-client".to_string(),
            "--policy-payload".to_string(),
            "target_region=eu;max_peers=1;mesh_connect_fallback_ports=65080,65081".to_string(),
            "--peer".to_string(),
            "n1@127.0.0.1:4443@eu@20@90".to_string(),
            "--peer".to_string(),
            "n1@127.0.0.1:5443@eu@25@88".to_string(),
            "--timeout-ms".to_string(),
            "25".to_string(),
        ],
        2,
        "connect_probe_duplicate_peer_node_id",
    );

    assert_eq!(parsed["status"].as_str().unwrap_or(""), "error");
    assert_eq!(
        parsed["kind"].as_str().unwrap_or(""),
        "mesh_route_explain_error"
    );
    assert_eq!(
        parsed["error_stage"].as_str().unwrap_or(""),
        "simulation_input"
    );
    assert_eq!(
        parsed["route_explain_error_action"].as_str().unwrap_or(""),
        "inspect_discovery_input"
    );
    assert_eq!(
        parsed["error"].as_str().unwrap_or(""),
        "duplicate peer node_id 'n1' in --peer set"
    );
}
