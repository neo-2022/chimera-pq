use super::connect_probe_flow::run_mesh_connect_probe_flow;
use super::options::MeshRouteExplainOptions;
use std::net::TcpListener;
use std::thread;

fn base_options() -> MeshRouteExplainOptions {
    MeshRouteExplainOptions {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
        policy_payload: "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        failed_node_id: None,
        cooldown_node_id: None,
        table_max_entries: None,
        table_max_entries_per_region: None,
        table_stale_after_ticks: None,
        connect_timeout_ms: Some(50),
        peers: vec!["n1@127.0.0.1:1@eu@20@90".to_string()],
        json_output: true,
        out_path: None,
    }
}

#[test]
fn connect_probe_flow_maps_duplicate_peer_to_simulation_input_stage() {
    let mut options = base_options();
    options.peers.push("n1@127.0.0.1:2@eu@25@88".to_string());

    let err = run_mesh_connect_probe_flow(&options, "test-connect-probe-flow")
        .err()
        .unwrap_or_else(|| unreachable!("duplicate peers should fail"));

    assert_eq!(err.stage, "simulation_input");
    assert_eq!(err.message, "duplicate peer node_id 'n1' in --peer set");
}

#[test]
fn connect_probe_flow_maps_policy_parse_errors_to_policy_parse_stage() {
    let mut options = base_options();
    options.policy_payload = "mesh_max_peers=0".to_string();

    let err = run_mesh_connect_probe_flow(&options, "test-connect-probe-flow")
        .err()
        .unwrap_or_else(|| unreachable!("invalid policy payload should fail"));

    assert_eq!(err.stage, "policy_parse");
    assert!(err.message.contains("mesh policy max_peers must be > 0"));
}

#[test]
fn connect_probe_flow_maps_peer_spec_errors_to_peer_spec_stage() {
    let mut options = base_options();
    options.peers = vec!["bad-peer-format".to_string()];

    let err = run_mesh_connect_probe_flow(&options, "test-connect-probe-flow")
        .err()
        .unwrap_or_else(|| unreachable!("invalid peer format should fail"));

    assert_eq!(err.stage, "peer_spec");
    assert_eq!(
        err.message,
        "expected node@endpoint#region@load@reliability"
    );
}

#[test]
fn connect_probe_flow_maps_peer_table_policy_errors_to_peer_table_policy_stage() {
    let mut options = base_options();
    options.table_max_entries = Some(0);

    let err = run_mesh_connect_probe_flow(&options, "test-connect-probe-flow")
        .err()
        .unwrap_or_else(|| unreachable!("invalid peer table policy should fail"));

    assert_eq!(err.stage, "peer_table_policy");
    assert!(
        err.message
            .contains("mesh peer table max_entries must be > 0")
    );
}

#[test]
fn connect_probe_flow_maps_unselectable_path_to_plan_path_stage() {
    let mut options = base_options();
    options.policy_payload = "allow=mesh;mesh_max_peers=1;mesh_min_reliability=99".to_string();

    let err = run_mesh_connect_probe_flow(&options, "test-connect-probe-flow")
        .err()
        .unwrap_or_else(|| unreachable!("unselectable path should fail planning"));

    assert_eq!(err.stage, "plan_path");
    assert!(
        err.message
            .contains("mesh path plan has zero eligible peers")
    );
}

#[test]
fn connect_probe_flow_succeeds_on_reachable_peer() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|e| unreachable!("listener bind should work: {e}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|e| unreachable!("listener local_addr should work: {e}"));
    let _accept_thread = thread::spawn(move || {
        let _ = listener.accept();
    });

    let mut options = base_options();
    options.peers = vec![format!("n1@127.0.0.1:{}@eu@20@90", addr.port())];
    options.connect_timeout_ms = Some(500);

    let (report, timeout_ms) = run_mesh_connect_probe_flow(&options, "test-connect-probe-flow")
        .unwrap_or_else(|e| unreachable!("reachable peer should succeed: {e:?}"));

    assert_eq!(timeout_ms, 500);
    assert!(report.success);
    assert_eq!(report.connected_peer, "n1");
    assert_eq!(
        report.connected_endpoint,
        format!("127.0.0.1:{}", addr.port())
    );
    assert_eq!(report.selected_peers, vec!["n1".to_string()]);
    assert!(report.attempts.iter().any(|attempt| attempt.success));
}

#[test]
fn connect_probe_flow_clamps_timeout_to_minimum_of_one_ms() {
    let mut options = base_options();
    options.connect_timeout_ms = Some(0);

    let (_report, timeout_ms) = run_mesh_connect_probe_flow(&options, "test-connect-probe-flow")
        .unwrap_or_else(|e| unreachable!("flow should still run with zero timeout input: {e:?}"));

    assert_eq!(timeout_ms, 1);
}

#[test]
fn connect_probe_flow_uses_default_timeout_when_not_provided() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|e| unreachable!("listener bind should work: {e}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|e| unreachable!("listener local_addr should work: {e}"));
    let _accept_thread = thread::spawn(move || {
        let _ = listener.accept();
    });

    let mut options = base_options();
    options.peers = vec![format!("n1@127.0.0.1:{}@eu@20@90", addr.port())];
    options.connect_timeout_ms = None;

    let (_report, timeout_ms) = run_mesh_connect_probe_flow(&options, "test-connect-probe-flow")
        .unwrap_or_else(|e| unreachable!("default timeout flow should succeed: {e:?}"));

    assert_eq!(timeout_ms, 1200);
}

#[test]
fn connect_probe_flow_maps_invalid_bootstrap_source_to_runtime_bootstrap_stage() {
    let mut options = base_options();
    options.namespace = String::new();

    let err = run_mesh_connect_probe_flow(&options, "test-connect-probe-flow")
        .err()
        .unwrap_or_else(|| unreachable!("invalid namespace should fail bootstrap"));

    assert_eq!(err.stage, "runtime_bootstrap");
    assert!(err.message.contains("namespace"));
}
