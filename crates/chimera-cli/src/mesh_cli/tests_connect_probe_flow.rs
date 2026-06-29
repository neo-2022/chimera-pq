use super::connect_probe_flow::run_mesh_connect_probe_flow;
use super::options::MeshRouteExplainOptions;
use chimera_carrier::peer_egress::lane_binding::load_transit_lane_document;
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

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
        transit_lane_bindings_out_path: None,
    }
}

fn temp_lane_document_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|error| unreachable!("system clock should be after unix epoch: {error}"))
        .as_nanos();
    std::env::temp_dir().join(format!(
        "chimera-connect-probe-{name}-{}-{nanos}.lanes",
        std::process::id()
    ))
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
    assert_eq!(report.connected_peer, "peer#1");
    assert_eq!(report.connected_endpoint, "endpoint#1:<redacted>");
    assert_eq!(report.selected_peers, vec!["peer#1".to_string()]);
    assert!(
        report
            .attempts
            .iter()
            .all(|attempt| attempt.peer_id == "peer#1"
                && attempt.endpoint == "endpoint#1:<redacted>")
    );
    assert!(report.attempts.iter().any(|attempt| attempt.success));

    let report_debug = format!("{report:?}");
    assert!(!report_debug.contains("n1"));
    assert!(!report_debug.contains("127.0.0.1"));
    assert!(!report_debug.contains(&addr.port().to_string()));
}

#[test]
fn connect_probe_flow_publishes_refreshed_transit_lane_document_after_success() {
    let listener_a = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|e| unreachable!("listener bind should work: {e}"));
    let addr_a = listener_a
        .local_addr()
        .unwrap_or_else(|e| unreachable!("listener local_addr should work: {e}"));
    let _accept_thread_a = thread::spawn(move || {
        let _ = listener_a.accept();
    });
    let listener_b = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|e| unreachable!("listener bind should work: {e}"));
    let addr_b = listener_b
        .local_addr()
        .unwrap_or_else(|e| unreachable!("listener local_addr should work: {e}"));
    let _accept_thread_b = thread::spawn(move || {
        let _ = listener_b.accept();
    });
    let out = temp_lane_document_path("success");
    let out_path = out.to_string_lossy().into_owned();

    let mut options = base_options();
    options.policy_payload = concat!(
        "allow=mesh;",
        "mesh_allowed_regions=eu;",
        "mesh_max_peers=2;",
        "mesh_max_selected_per_region=2;",
        "mesh_min_reliability=80;",
        "mesh_multipath_mode=flow_shard;",
        "mesh_route_binding_id=8123"
    )
    .to_string();
    options.peers = vec![
        format!("n1@127.0.0.1:{}@eu@20@90", addr_a.port()),
        format!("n2@127.0.0.1:{}@eu@21@91", addr_b.port()),
    ];
    options.connect_timeout_ms = Some(500);
    options.transit_lane_bindings_out_path = Some(out_path.clone());

    let (report, _timeout_ms) = run_mesh_connect_probe_flow(&options, "test-connect-probe-flow")
        .unwrap_or_else(|e| unreachable!("reachable peer should publish lanes: {e:?}"));
    let document = load_transit_lane_document(&out_path)
        .unwrap_or_else(|e| unreachable!("published lane document should parse: {e}"));
    let plan = document
        .require_mesh_path_plan()
        .unwrap_or_else(|e| unreachable!("published lane document should include plan: {e}"));

    assert!(report.success);
    assert_eq!(document.registrations().len(), 2);
    assert_eq!(plan.namespace, "cef-public");
    assert_eq!(plan.multipath_schedule.carrier_lane_bindings.len(), 2);
    assert!(
        fs::read_to_string(&out)
            .unwrap_or_else(|e| unreachable!("lane document should be readable: {e}"))
            .starts_with("# chimera_transit_lane_document=v1\n")
    );

    let _ = fs::remove_file(out);
}

#[test]
fn connect_probe_flow_does_not_overwrite_transit_lane_document_after_failed_probe() {
    let out = temp_lane_document_path("failed");
    let sentinel = "existing sealed lane document placeholder\n";
    fs::write(&out, sentinel).unwrap_or_else(|e| unreachable!("sentinel write should work: {e}"));

    let mut options = base_options();
    options.policy_payload = concat!(
        "allow=mesh;",
        "mesh_max_peers=1;",
        "mesh_min_reliability=80;",
        "mesh_multipath_mode=flow_shard;",
        "mesh_route_binding_id=8124"
    )
    .to_string();
    options.transit_lane_bindings_out_path = Some(out.to_string_lossy().to_string());

    let (report, _timeout_ms) = run_mesh_connect_probe_flow(&options, "test-connect-probe-flow")
        .unwrap_or_else(|e| unreachable!("failed probe should return a report: {e:?}"));
    let body = fs::read_to_string(&out)
        .unwrap_or_else(|e| unreachable!("sentinel lane file should remain readable: {e}"));

    assert!(!report.success);
    assert_eq!(body, sentinel);

    let _ = fs::remove_file(out);
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
