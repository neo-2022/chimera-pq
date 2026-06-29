use super::build_connect_attempt_plan;
use crate::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPublishedEndpointUpdate, MeshRuntime,
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
