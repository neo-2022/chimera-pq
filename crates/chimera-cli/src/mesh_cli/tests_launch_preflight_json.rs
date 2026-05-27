#![forbid(unsafe_code)]

use std::net::TcpListener;
use std::thread;

use serde_json::json;

use super::tests_json_runner_utils::run_mesh_subcommand_json;

fn normalized_explain_lines(payload: &serde_json::Value) -> Vec<String> {
    payload["explain"]
        .as_array()
        .unwrap_or_else(|| unreachable!("explain should be an array"))
        .iter()
        .filter_map(|line| line.as_str())
        .filter(|line| !line.starts_with("discovery_source_names="))
        .map(ToOwned::to_owned)
        .collect()
}

fn expected_launch_payload_from_connect(
    connect: &serde_json::Value,
    node: &str,
    timeout_ms: u64,
) -> serde_json::Value {
    let success = connect["success"]
        .as_bool()
        .unwrap_or_else(|| unreachable!("connect success should be bool"));
    let blockers = if success {
        vec![]
    } else {
        vec!["connectivity_probe_failed".to_string()]
    };
    json!({
        "status": if success { "ready" } else { "blocked" },
        "network_state": "not_modified",
        "namespace": connect["namespace"].clone(),
        "node": node,
        "timeout_ms": timeout_ms,
        "ready_for_real_launch": success,
        "blockers": blockers,
        "selected_peers": connect["selected_peers"].clone(),
        "connected_peer": connect["connected_peer"].clone(),
        "connected_endpoint": connect["connected_endpoint"].clone(),
        "connect_probe_success": success,
        "attempts": connect["attempts"].clone(),
        "explain": normalized_explain_lines(connect),
    })
}

fn canonicalize_launch_payload(payload: &serde_json::Value) -> serde_json::Value {
    let mut value = payload.clone();
    value["explain"] = json!(normalized_explain_lines(payload));
    value
}

#[test]
fn launch_preflight_json_blocked_shape() {
    let parsed = run_mesh_subcommand_json(
        "launch-preflight",
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
        "launch_preflight_blocked",
    );
    assert_eq!(parsed["status"].as_str().unwrap_or(""), "blocked");
    assert_eq!(
        parsed["network_state"].as_str().unwrap_or(""),
        "not_modified"
    );
    assert_eq!(parsed["ready_for_real_launch"].as_bool(), Some(false));
    assert_eq!(parsed["connect_probe_success"].as_bool(), Some(false));
    assert!(parsed["selected_peers"].is_array());
    assert!(parsed["attempts"].is_array());
    assert!(parsed["explain"].is_array());
    let blockers = parsed["blockers"]
        .as_array()
        .unwrap_or_else(|| unreachable!("blockers should be array"));
    assert_eq!(blockers.len(), 1);
    assert_eq!(
        blockers[0].as_str().unwrap_or(""),
        "connectivity_probe_failed"
    );
}

#[test]
fn launch_preflight_json_ready_shape() {
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
        "launch-preflight",
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
        "launch_preflight_ready",
    );
    assert_eq!(parsed["status"].as_str().unwrap_or(""), "ready");
    assert_eq!(
        parsed["network_state"].as_str().unwrap_or(""),
        "not_modified"
    );
    assert_eq!(parsed["ready_for_real_launch"].as_bool(), Some(true));
    assert_eq!(parsed["connect_probe_success"].as_bool(), Some(true));
    let blockers = parsed["blockers"]
        .as_array()
        .unwrap_or_else(|| unreachable!("blockers should be array"));
    assert!(blockers.is_empty());
    assert_eq!(parsed["connected_peer"].as_str().unwrap_or(""), "n1");
    assert_eq!(
        parsed["connected_endpoint"].as_str().unwrap_or(""),
        format!("127.0.0.1:{}", addr.port())
    );
}

#[test]
fn launch_preflight_json_rejects_duplicate_peer_node_ids() {
    let parsed = run_mesh_subcommand_json(
        "launch-preflight",
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
        "launch_preflight_duplicate_peer_node_id",
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

#[test]
fn launch_preflight_json_matches_connect_probe_payload_on_blocked_probe() {
    let args = vec![
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
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        1,
        "connect_probe_parity_blocked",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        1,
        "launch_preflight_parity_blocked",
    );

    assert_eq!(launch["status"].as_str().unwrap_or(""), "blocked");
    assert_eq!(
        launch["network_state"].as_str().unwrap_or(""),
        "not_modified"
    );
    assert_eq!(launch["ready_for_real_launch"].as_bool(), Some(false));
    assert_eq!(
        launch["connect_probe_success"].as_bool(),
        connect["success"].as_bool()
    );
    assert_eq!(launch["namespace"], connect["namespace"]);
    assert_eq!(launch["selected_peers"], connect["selected_peers"]);
    assert_eq!(launch["attempts"], connect["attempts"]);
    assert_eq!(launch["connected_peer"], connect["connected_peer"]);
    assert_eq!(launch["connected_endpoint"], connect["connected_endpoint"]);
    assert_eq!(
        normalized_explain_lines(&launch),
        normalized_explain_lines(&connect)
    );
    let expected = expected_launch_payload_from_connect(&connect, "node-client", 25);
    assert_eq!(canonicalize_launch_payload(&launch), expected);
}

#[test]
fn launch_preflight_json_matches_connect_probe_payload_on_ready_probe() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|e| unreachable!("listener bind should work: {e}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|e| unreachable!("listener local_addr should work: {e}"));
    let _accept_thread = thread::spawn(move || {
        let _ = listener.accept();
        let _ = listener.accept();
    });

    let peer = format!("n1@127.0.0.1:{}@eu@20@90", addr.port());
    let args = vec![
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
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        0,
        "connect_probe_parity_ready",
    );
    let launch =
        run_mesh_subcommand_json("launch-preflight", args, 0, "launch_preflight_parity_ready");

    assert_eq!(launch["status"].as_str().unwrap_or(""), "ready");
    assert_eq!(
        launch["network_state"].as_str().unwrap_or(""),
        "not_modified"
    );
    assert_eq!(launch["ready_for_real_launch"].as_bool(), Some(true));
    assert_eq!(
        launch["connect_probe_success"].as_bool(),
        connect["success"].as_bool()
    );
    assert_eq!(launch["namespace"], connect["namespace"]);
    assert_eq!(launch["selected_peers"], connect["selected_peers"]);
    assert_eq!(launch["attempts"], connect["attempts"]);
    assert_eq!(launch["connected_peer"], connect["connected_peer"]);
    assert_eq!(launch["connected_endpoint"], connect["connected_endpoint"]);
    assert_eq!(
        normalized_explain_lines(&launch),
        normalized_explain_lines(&connect)
    );
    let expected = expected_launch_payload_from_connect(&connect, "node-client", 500);
    assert_eq!(canonicalize_launch_payload(&launch), expected);
}

#[test]
fn launch_preflight_json_uses_clamped_timeout_projection_when_timeout_zero() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "target_region=eu;max_peers=1;mesh_connect_fallback_ports=65080,65081".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--timeout-ms".to_string(),
        "0".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        1,
        "connect_probe_timeout_zero",
    );
    let launch =
        run_mesh_subcommand_json("launch-preflight", args, 1, "launch_preflight_timeout_zero");

    let expected = expected_launch_payload_from_connect(&connect, "node-client", 1);
    assert_eq!(canonicalize_launch_payload(&launch), expected);
}

#[test]
fn launch_preflight_json_uses_default_timeout_projection_when_timeout_missing() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "target_region=eu;max_peers=1;mesh_connect_fallback_ports=65080,65081".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        1,
        "connect_probe_timeout_default",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        1,
        "launch_preflight_timeout_default",
    );

    let expected = expected_launch_payload_from_connect(&connect, "node-client", 1200);
    assert_eq!(canonicalize_launch_payload(&launch), expected);
}
