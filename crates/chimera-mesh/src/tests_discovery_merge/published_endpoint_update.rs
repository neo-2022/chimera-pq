use crate::{
    MeshDiscoveryRecord, MeshMultipathRebuildDirtyScope, MeshPublishedEndpointUpdate, MeshRuntime,
};

fn record(node_id: &str, endpoint: &str) -> MeshDiscoveryRecord {
    MeshDiscoveryRecord {
        node_id: node_id.to_string(),
        endpoint: endpoint.to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 90,
    }
}

fn endpoint_update(
    node_id: &str,
    endpoint: &str,
    update_bootstrap_url: Option<&str>,
    endpoint_generation: u64,
) -> MeshPublishedEndpointUpdate {
    MeshPublishedEndpointUpdate {
        node_id: node_id.to_string(),
        endpoint: endpoint.to_string(),
        update_bootstrap_url: update_bootstrap_url.map(str::to_string),
        endpoint_generation,
    }
}

fn runtime_with_peers(records: &[MeshDiscoveryRecord]) -> MeshRuntime {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    runtime
        .merge_discovery("seed-b", records)
        .unwrap_or_else(|e| unreachable!("discovery merge should succeed: {e}"));
    let _ = runtime.take_pending_multipath_rebuild_signal();
    runtime
}

fn assert_peer_set_signal(runtime: &MeshRuntime, affected_peer_count: usize) {
    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("endpoint update should trigger rebuild"));
    assert_eq!(signal.reason(), "published_endpoint_changed");
    assert_eq!(
        signal.dirty_scope(),
        MeshMultipathRebuildDirtyScope::PeerSet
    );
    assert_eq!(signal.affected_peer_count(), affected_peer_count);
}

#[test]
fn published_endpoint_new_generation_updates_existing_peer_and_marks_dirty() {
    let mut runtime = runtime_with_peers(&[record("node-a", "198.51.100.10:443")]);

    runtime
        .merge_published_endpoint_updates(
            "state-publish",
            &[endpoint_update(
                "node-a",
                "198.51.100.20:9443",
                Some("https://node.example:9443/chimera.sh"),
                1,
            )],
        )
        .unwrap_or_else(|e| unreachable!("endpoint update should succeed: {e}"));

    assert_peer_set_signal(&runtime, 1);
    assert_eq!(runtime.peer_snapshot()[0].endpoint, "198.51.100.20:9443");
}

#[test]
fn published_endpoint_same_generation_same_state_is_noop() {
    let mut runtime = runtime_with_peers(&[record("node-a", "198.51.100.10:443")]);
    let update = endpoint_update(
        "node-a",
        "198.51.100.20:9443",
        Some("https://node.example:9443/chimera.sh"),
        3,
    );
    runtime
        .merge_published_endpoint_updates("state-publish", std::slice::from_ref(&update))
        .unwrap_or_else(|e| unreachable!("initial endpoint update should succeed: {e}"));
    let _ = runtime.take_pending_multipath_rebuild_signal();
    let before_snapshot = runtime.peer_snapshot();

    runtime
        .merge_published_endpoint_updates("state-publish", &[update])
        .unwrap_or_else(|e| unreachable!("same endpoint update should succeed: {e}"));

    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    assert_eq!(runtime.peer_snapshot(), before_snapshot);
}

#[test]
fn published_endpoint_stale_generation_is_ignored_without_dirty_signal() {
    let mut runtime = runtime_with_peers(&[record("node-a", "198.51.100.10:443")]);
    runtime
        .merge_published_endpoint_updates(
            "state-publish",
            &[endpoint_update("node-a", "198.51.100.20:9443", None, 8)],
        )
        .unwrap_or_else(|e| unreachable!("initial endpoint update should succeed: {e}"));
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_published_endpoint_updates(
            "state-publish",
            &[endpoint_update("node-a", "198.51.100.30:9443", None, 7)],
        )
        .unwrap_or_else(|e| unreachable!("stale endpoint update should be ignored: {e}"));

    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    assert_eq!(runtime.peer_snapshot()[0].endpoint, "198.51.100.20:9443");
}

#[test]
fn published_endpoint_noop_batch_skips_dirty_rebuild_for_liveness_only_updates() {
    let mut runtime = runtime_with_peers(&[
        record("node-a", "198.51.100.10:443"),
        record("node-b", "198.51.100.11:443"),
    ]);
    runtime
        .merge_published_endpoint_updates(
            "state-publish",
            &[
                endpoint_update("node-a", "198.51.100.20:9443", None, 4),
                endpoint_update("node-b", "198.51.100.21:9443", None, 3),
            ],
        )
        .unwrap_or_else(|e| unreachable!("initial endpoint update should succeed: {e}"));
    let _ = runtime.take_pending_multipath_rebuild_signal();
    let before_snapshot = runtime.peer_snapshot();

    runtime
        .merge_published_endpoint_updates(
            "state-publish",
            &[
                endpoint_update("node-a", "198.51.100.20:9443", None, 4),
                endpoint_update("node-b", "198.51.100.31:9443", None, 2),
                endpoint_update("missing-node", "198.51.100.99:9443", None, 1),
            ],
        )
        .unwrap_or_else(|e| unreachable!("noop endpoint batch should succeed: {e}"));

    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    assert_eq!(runtime.peer_snapshot(), before_snapshot);
}

#[test]
fn published_endpoint_empty_batch_is_liveness_noop_without_dirty_rebuild() {
    let mut runtime = runtime_with_peers(&[record("node-a", "198.51.100.10:443")]);
    let before_snapshot = runtime.peer_snapshot();

    runtime
        .merge_published_endpoint_updates("state-publish", &[])
        .unwrap_or_else(|e| unreachable!("empty endpoint batch should succeed: {e}"));

    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    assert_eq!(runtime.peer_snapshot(), before_snapshot);
}

#[test]
fn published_endpoint_unknown_only_batch_is_noop_without_dirty_rebuild() {
    let mut runtime = runtime_with_peers(&[record("node-a", "198.51.100.10:443")]);
    let before_snapshot = runtime.peer_snapshot();

    runtime
        .merge_published_endpoint_updates(
            "state-publish",
            &[endpoint_update(
                "missing-node",
                "198.51.100.99:9443",
                Some("https://missing.example:9443/chimera.sh"),
                1,
            )],
        )
        .unwrap_or_else(|e| unreachable!("unknown-only endpoint batch should succeed: {e}"));

    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    assert_eq!(runtime.peer_snapshot(), before_snapshot);
}

#[test]
fn published_endpoint_zero_generation_is_rejected_atomically() -> Result<(), String> {
    let mut runtime = runtime_with_peers(&[record("node-a", "198.51.100.10:443")]);
    let before_snapshot = runtime.peer_snapshot();

    let error = match runtime.merge_published_endpoint_updates(
        "state-publish",
        &[endpoint_update("node-a", "198.51.100.20:9443", None, 0)],
    ) {
        Ok(_) => return Err("zero generation endpoint update must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("endpoint_generation"));
    assert_eq!(runtime.peer_snapshot(), before_snapshot);
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    Ok(())
}

#[test]
fn published_endpoint_invalid_endpoint_is_rejected_atomically() -> Result<(), String> {
    let mut runtime = runtime_with_peers(&[
        record("node-a", "198.51.100.10:443"),
        record("node-b", "198.51.100.11:443"),
    ]);
    let before_snapshot = runtime.peer_snapshot();

    let error = match runtime.merge_published_endpoint_updates(
        "state-publish",
        &[
            endpoint_update("node-a", "198.51.100.20:9443", None, 1),
            endpoint_update("node-b", "198.51.100.21", None, 1),
        ],
    ) {
        Ok(_) => return Err("invalid endpoint update batch must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("endpoint"));
    assert_eq!(runtime.peer_snapshot(), before_snapshot);
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    Ok(())
}

#[test]
fn published_endpoint_generation_conflict_is_rejected_atomically() -> Result<(), String> {
    let mut runtime = runtime_with_peers(&[record("node-a", "198.51.100.10:443")]);
    runtime
        .merge_published_endpoint_updates(
            "state-publish",
            &[endpoint_update("node-a", "198.51.100.20:9443", None, 4)],
        )
        .unwrap_or_else(|e| unreachable!("initial endpoint update should succeed: {e}"));
    let _ = runtime.take_pending_multipath_rebuild_signal();
    let before_snapshot = runtime.peer_snapshot();

    let error = match runtime.merge_published_endpoint_updates(
        "state-publish",
        &[endpoint_update("node-a", "198.51.100.30:9443", None, 4)],
    ) {
        Ok(_) => return Err("generation conflict endpoint update must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("generation conflict"));
    assert_eq!(runtime.peer_snapshot(), before_snapshot);
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    Ok(())
}

#[test]
fn published_endpoint_mixed_batch_counts_only_changed_existing_peers() {
    let unchanged = record("node-a", "198.51.100.10:443");
    let mut runtime = runtime_with_peers(&[
        unchanged.clone(),
        record("node-b", "198.51.100.11:443"),
        record("node-c", "198.51.100.12:443"),
    ]);
    runtime
        .merge_published_endpoint_updates(
            "state-publish",
            &[endpoint_update("node-a", "198.51.100.10:443", None, 5)],
        )
        .unwrap_or_else(|e| unreachable!("initial endpoint generation should succeed: {e}"));
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_published_endpoint_updates(
            "state-publish",
            &[
                endpoint_update("node-a", "198.51.100.10:443", None, 5),
                endpoint_update("node-b", "198.51.100.21:9443", None, 1),
                endpoint_update("missing-node", "198.51.100.99:443", None, 1),
            ],
        )
        .unwrap_or_else(|e| unreachable!("mixed endpoint update should succeed: {e}"));

    assert_peer_set_signal(&runtime, 1);
    let endpoints = runtime
        .peer_snapshot()
        .into_iter()
        .map(|peer| (peer.node_id, peer.endpoint))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        endpoints.get("node-a").map(String::as_str),
        Some("198.51.100.10:443")
    );
    assert_eq!(
        endpoints.get("node-b").map(String::as_str),
        Some("198.51.100.21:9443")
    );
    assert_eq!(
        endpoints.get("node-c").map(String::as_str),
        Some("198.51.100.12:443")
    );
}

#[test]
fn discovery_without_generation_does_not_downgrade_published_endpoint() {
    let mut runtime = runtime_with_peers(&[record("node-a", "198.51.100.10:443")]);
    runtime
        .merge_published_endpoint_updates(
            "state-publish",
            &[endpoint_update("node-a", "198.51.100.20:9443", None, 4)],
        )
        .unwrap_or_else(|e| unreachable!("published endpoint should be accepted: {e}"));
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery(
            "seed-c",
            &[MeshDiscoveryRecord {
                node_id: "node-a".to_string(),
                endpoint: "198.51.100.10:443".to_string(),
                region: "eu".to_string(),
                load_score: 10,
                reliability_score: 95,
            }],
        )
        .unwrap_or_else(|e| unreachable!("discovery metadata update should succeed: {e}"));

    let snapshot = runtime.peer_snapshot();
    assert_eq!(snapshot[0].endpoint, "198.51.100.20:9443");
    assert_eq!(snapshot[0].load_score, 10);
    assert_eq!(snapshot[0].reliability_score, 95);
}

#[test]
fn published_endpoint_two_newer_records_count_two() {
    let mut runtime = runtime_with_peers(&[
        record("node-a", "198.51.100.10:443"),
        record("node-b", "198.51.100.11:443"),
    ]);

    runtime
        .merge_published_endpoint_updates(
            "state-publish",
            &[
                endpoint_update("node-a", "198.51.100.20:9443", None, 1),
                endpoint_update("node-b", "198.51.100.21:9443", None, 1),
            ],
        )
        .unwrap_or_else(|e| unreachable!("endpoint update should succeed: {e}"));

    assert_peer_set_signal(&runtime, 2);
}

#[test]
fn published_endpoint_debug_and_rebuild_signal_redact_endpoint_and_url() {
    let update = endpoint_update(
        "node-sensitive",
        "198.51.100.20:9443",
        Some("https://node.example:9443/chimera.sh"),
        9,
    );
    let debug = format!("{update:?}");
    assert!(debug.contains("<redacted>"));
    assert!(debug.contains("endpoint_generation"));
    assert!(!debug.contains("node-sensitive"));
    assert!(!debug.contains("198.51.100.20"));
    assert!(!debug.contains("node.example"));

    let mut runtime = runtime_with_peers(&[record("node-sensitive", "198.51.100.10:443")]);
    runtime
        .merge_published_endpoint_updates("state-publish", &[update])
        .unwrap_or_else(|e| unreachable!("endpoint update should succeed: {e}"));
    let signal_debug = format!(
        "{:?}",
        runtime
            .pending_multipath_rebuild_signal()
            .unwrap_or_else(|| unreachable!("endpoint update should trigger rebuild"))
    );
    assert!(signal_debug.contains("published_endpoint_changed"));
    assert!(signal_debug.contains("affected_peer_count"));
    assert!(!signal_debug.contains("node-sensitive"));
    assert!(!signal_debug.contains("198.51.100.20"));
    assert!(!signal_debug.contains("node.example"));
}
