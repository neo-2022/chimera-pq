use std::net::TcpListener;
use std::thread;

use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshRuntime};

#[test]
fn connect_probe_uses_current_endpoint_then_fallback_ports() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|e| unreachable!("listener bind should succeed: {e}"));
    let fallback_port = listener
        .local_addr()
        .unwrap_or_else(|e| unreachable!("listener local_addr should succeed: {e}"))
        .port();
    let _accept_thread = thread::spawn(move || {
        let _ = listener.accept();
    });

    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "127.0.0.1:1".to_string(),
        region: "eu".to_string(),
        load_score: 10,
        reliability_score: 90,
    }];
    runtime
        .merge_discovery("seed-b", &records)
        .unwrap_or_else(|e| unreachable!("merge discovery should succeed: {e}"));

    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let payload = format!(
        "allow=mesh;target_region=eu;mesh_max_peers=1;mesh_max_selected_per_region=1;mesh_connect_fallback_ports={fallback_port}"
    );
    let policy = MeshPathPolicy::from_dps_payload(&payload)
        .unwrap_or_else(|e| unreachable!("policy parse should succeed: {e}"));

    let report = runtime
        .connect_probe(&req, &policy, 500)
        .unwrap_or_else(|e| unreachable!("connect probe should succeed: {e}"));

    assert!(report.success);
    assert_eq!(report.connected_peer, "node-a");
    assert_eq!(
        report.connected_endpoint,
        format!("127.0.0.1:{fallback_port}")
    );
    assert_eq!(report.attempts.len(), 2);
    assert_eq!(report.attempts[0].peer_id, "node-a");
    assert_eq!(report.attempts[0].endpoint, "127.0.0.1:1");
    assert!(!report.attempts[0].success);
    assert_eq!(report.attempts[1].peer_id, "node-a");
    assert_eq!(
        report.attempts[1].endpoint,
        format!("127.0.0.1:{fallback_port}")
    );
    assert!(report.attempts[1].success);
}

#[test]
fn connect_probe_reports_failed_attempts_with_errors() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    runtime
        .merge_discovery(
            "seed-b",
            &[MeshDiscoveryRecord {
                node_id: "node-a".to_string(),
                endpoint: "127.0.0.1:1".to_string(),
                region: "eu".to_string(),
                load_score: 10,
                reliability_score: 90,
            }],
        )
        .unwrap_or_else(|e| unreachable!("merge discovery should succeed: {e}"));
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let policy = MeshPathPolicy::from_dps_payload(
        "allow=mesh;target_region=eu;mesh_max_peers=1;mesh_max_selected_per_region=1;mesh_connect_fallback_ports=2",
    )
    .unwrap_or_else(|e| unreachable!("policy parse should succeed: {e}"));

    let report = runtime
        .connect_probe(&req, &policy, 25)
        .unwrap_or_else(|e| unreachable!("connect probe should succeed with report: {e}"));

    assert!(!report.success);
    assert!(report.connected_peer.is_empty());
    assert!(report.connected_endpoint.is_empty());
    assert_eq!(report.attempts.len(), 2);
    assert!(report.attempts.iter().all(|attempt| !attempt.success));
    assert!(
        report
            .attempts
            .iter()
            .all(|attempt| !attempt.error.trim().is_empty())
    );
    assert!(
        report
            .explain
            .iter()
            .any(|line| line == "connect_probe_result=failed")
    );
}
