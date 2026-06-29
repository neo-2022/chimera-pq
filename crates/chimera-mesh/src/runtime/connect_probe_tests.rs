use super::{build_connect_attempt_plan, visit_connect_attempt_targets};
use crate::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPeerState,
    MeshPublishedEndpointUpdate, MeshRuntime,
};

fn record(node_id: &str, endpoint: &str) -> MeshDiscoveryRecord {
    MeshDiscoveryRecord {
        node_id: node_id.to_string(),
        endpoint: endpoint.to_string(),
        region: "eu".to_string(),
        load_score: 10,
        reliability_score: 95,
    }
}

fn endpoint_update(endpoint: &str, endpoint_generation: u64) -> MeshPublishedEndpointUpdate {
    MeshPublishedEndpointUpdate {
        node_id: "node-a".to_string(),
        endpoint: endpoint.to_string(),
        update_bootstrap_url: None,
        endpoint_generation,
    }
}

fn request() -> MeshJoinRequest {
    MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    }
}

fn policy() -> MeshPathPolicy {
    MeshPathPolicy::from_dps_payload(
        "allow=mesh;target_region=eu;mesh_max_peers=1;mesh_max_selected_per_region=1;mesh_connect_fallback_ports=443,8443",
    )
    .unwrap_or_else(|e| unreachable!("policy parse should succeed: {e}"))
}

fn planned_endpoints(
    runtime: &MeshRuntime,
    policy: &MeshPathPolicy,
) -> Result<Vec<String>, String> {
    let plan = runtime.plan_path(&request(), policy)?;
    Ok(
        build_connect_attempt_plan(&plan.selected_peers, &policy.connect_fallback_ports)?
            .into_iter()
            .map(|target| target.endpoint)
            .collect(),
    )
}

fn assert_no_endpoint_with_host(endpoints: &[String], host: &str) {
    assert!(
        endpoints.iter().all(|endpoint| !endpoint.starts_with(host)),
        "unexpected stale host in connect attempt plan"
    );
}

fn assert_endpoint_with_host(endpoints: &[String], host: &str) {
    assert!(
        endpoints.iter().any(|endpoint| endpoint.starts_with(host)),
        "expected host missing from connect attempt plan"
    );
}

fn assert_same_connect_attempt_plan(actual: &[String], expected: &[String]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "connect attempt plan length changed"
    );
    assert!(
        actual
            .iter()
            .zip(expected.iter())
            .all(|(actual, expected)| actual == expected),
        "connect attempt plan changed"
    );
}

fn peer_state(node_id: &str, endpoint: &str) -> MeshPeerState {
    MeshPeerState {
        node_id: node_id.to_string(),
        endpoint: endpoint.to_string(),
        region: "eu".to_string(),
        reliability_score: 90,
        load_score: 10,
        latency_ms: None,
        throughput_mbps: None,
        selection_score: 100,
    }
}

#[test]
fn connect_attempt_plan_uses_fresh_published_endpoint_generation() -> Result<(), String> {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
    runtime.merge_discovery("seed-b", &[record("node-a", "198.51.100.10:9443")])?;
    let _ = runtime.take_pending_multipath_rebuild_signal();
    let policy = policy();

    let before = planned_endpoints(&runtime, &policy)?;
    assert_endpoint_with_host(&before, "198.51.100.10:");

    runtime.merge_published_endpoint_updates(
        "state-publish",
        &[endpoint_update("198.51.100.20:9443", 2)],
    )?;

    let signal = runtime
        .pending_multipath_rebuild_signal()
        .ok_or_else(|| "fresh endpoint update should mark reconnect path dirty".to_string())?;
    assert_eq!(signal.reason(), "published_endpoint_changed");
    assert_eq!(signal.affected_peer_count(), 1);

    let after = planned_endpoints(&runtime, &policy)?;
    assert_endpoint_with_host(&after, "198.51.100.20:");
    assert_no_endpoint_with_host(&after, "198.51.100.10:");
    assert_eq!(after.len(), 3);
    Ok(())
}

#[test]
fn connect_attempt_plan_ignores_stale_and_noop_published_endpoint_updates() -> Result<(), String> {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
    runtime.merge_discovery("seed-b", &[record("node-a", "198.51.100.10:9443")])?;
    let _ = runtime.take_pending_multipath_rebuild_signal();
    runtime.merge_published_endpoint_updates(
        "state-publish",
        &[endpoint_update("198.51.100.20:9443", 7)],
    )?;
    let _ = runtime.take_pending_multipath_rebuild_signal();
    let policy = policy();
    let fresh = planned_endpoints(&runtime, &policy)?;

    runtime.merge_published_endpoint_updates(
        "state-publish",
        &[endpoint_update("198.51.100.30:9443", 6)],
    )?;
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    let after_stale = planned_endpoints(&runtime, &policy)?;
    assert_same_connect_attempt_plan(&after_stale, &fresh);
    assert_no_endpoint_with_host(&after_stale, "198.51.100.30:");

    runtime.merge_published_endpoint_updates(
        "state-publish",
        &[endpoint_update("198.51.100.20:9443", 7)],
    )?;
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    let after_noop = planned_endpoints(&runtime, &policy)?;
    assert_same_connect_attempt_plan(&after_noop, &fresh);
    Ok(())
}

#[test]
fn connect_attempt_plan_survives_invalid_published_endpoint_update_atomically() -> Result<(), String>
{
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
    runtime.merge_discovery("seed-b", &[record("node-a", "198.51.100.10:9443")])?;
    let _ = runtime.take_pending_multipath_rebuild_signal();
    runtime.merge_published_endpoint_updates(
        "state-publish",
        &[endpoint_update("198.51.100.20:9443", 3)],
    )?;
    let _ = runtime.take_pending_multipath_rebuild_signal();
    let policy = policy();
    let before = planned_endpoints(&runtime, &policy)?;

    let error = match runtime
        .merge_published_endpoint_updates("state-publish", &[endpoint_update("198.51.100.30", 4)])
    {
        Ok(_) => return Err("invalid endpoint update must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("endpoint"));
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    let after = planned_endpoints(&runtime, &policy)?;
    assert_same_connect_attempt_plan(&after, &before);
    assert_no_endpoint_with_host(&after, "198.51.100.30:");
    Ok(())
}

#[test]
fn connect_attempt_lazy_traversal_matches_snapshot_order() -> Result<(), String> {
    let selected_peers = vec![
        peer_state("node-a", "198.51.100.10:9443"),
        peer_state("node-b", "[2001:db8::10]:9443"),
    ];
    let fallback_ports = [9443, 443, 8443, 443, 0];
    let snapshot = build_connect_attempt_plan(&selected_peers, &fallback_ports)?;
    let mut lazy = Vec::new();

    let stopped = visit_connect_attempt_targets(&selected_peers, &fallback_ports, |target| {
        lazy.push((
            target.peer_index,
            target.peer_id.to_string(),
            target.endpoint.to_string(),
        ));
        false
    })?;

    assert!(!stopped);
    let snapshot: Vec<_> = snapshot
        .into_iter()
        .map(|target| (target.peer_index, target.peer_id, target.endpoint))
        .collect();
    assert_eq!(lazy, snapshot);
    assert_eq!(
        lazy,
        vec![
            (0, "node-a".to_string(), "198.51.100.10:9443".to_string()),
            (0, "node-a".to_string(), "198.51.100.10:443".to_string()),
            (0, "node-a".to_string(), "198.51.100.10:8443".to_string()),
            (1, "node-b".to_string(), "[2001:db8::10]:9443".to_string()),
            (1, "node-b".to_string(), "[2001:db8::10]:443".to_string()),
            (1, "node-b".to_string(), "[2001:db8::10]:8443".to_string()),
        ]
    );
    Ok(())
}

#[test]
fn connect_attempt_lazy_traversal_stops_before_unused_fallbacks() -> Result<(), String> {
    let selected_peers = vec![
        peer_state("node-a", "198.51.100.10:9443"),
        peer_state("node-b", "198.51.100.20:9443"),
    ];
    let fallback_ports = [443, 8443, 9443];
    let mut visited = Vec::new();

    let stopped = visit_connect_attempt_targets(&selected_peers, &fallback_ports, |target| {
        visited.push(target.endpoint.to_string());
        visited.len() == 2
    })?;

    assert!(stopped);
    assert_eq!(
        visited,
        vec![
            "198.51.100.10:9443".to_string(),
            "198.51.100.10:443".to_string(),
        ]
    );
    Ok(())
}
