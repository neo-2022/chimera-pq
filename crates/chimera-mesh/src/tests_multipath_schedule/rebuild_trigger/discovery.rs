use crate::{MeshMultipathRebuildDirtyScope, MeshPeerTablePolicy};

use super::assert_peer_table_changed_signal;
use crate::tests_multipath_schedule::{explain_has, record, request, runtime_with_peers};

#[test]
fn discovery_update_marks_pending_peer_table_rebuild() {
    let mut runtime = runtime_with_peers(vec![record("node-a", "198.51.100.31:443", "eu", 10, 95)]);
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery(
            "seed-c",
            &[record("node-b", "198.51.100.32:443", "eu", 20, 99)],
        )
        .unwrap_or_else(|e| unreachable!("discovery should succeed: {e}"));

    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("discovery should set pending rebuild signal"));
    assert_eq!(signal.reason(), "peer_table_changed");
    assert_eq!(
        signal.dirty_scope(),
        MeshMultipathRebuildDirtyScope::PeerSet
    );
    assert_eq!(signal.affected_peer_count(), 1);
    let debug = format!("{runtime:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("198.51.100.32"));
}

#[test]
fn discovery_update_batch_marks_aggregate_peer_set_dirty_scope() {
    let mut runtime = runtime_with_peers(vec![record("node-a", "198.51.100.31:443", "eu", 10, 95)]);
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery(
            "seed-c",
            &[
                record("node-b", "198.51.100.32:443", "eu", 20, 99),
                record("node-c", "198.51.100.33:443", "us", 25, 96),
            ],
        )
        .unwrap_or_else(|e| unreachable!("discovery should succeed: {e}"));

    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("discovery should set pending rebuild signal"));
    assert_eq!(signal.reason(), "peer_table_changed");
    assert_eq!(
        signal.dirty_scope(),
        MeshMultipathRebuildDirtyScope::PeerSet
    );
    assert_eq!(signal.affected_peer_count(), 2);
}

#[test]
fn discovery_update_mixed_batch_counts_only_changed_records() {
    let unchanged = record("node-a", "198.51.100.31:443", "eu", 10, 95);
    let mut runtime = runtime_with_peers(vec![unchanged.clone()]);
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery(
            "seed-c",
            &[
                unchanged,
                record("node-b", "198.51.100.32:443", "eu", 20, 99),
            ],
        )
        .unwrap_or_else(|e| unreachable!("mixed discovery should succeed: {e}"));

    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("changed discovery should set pending rebuild signal"));
    assert_eq!(signal.reason(), "peer_table_changed");
    assert_eq!(
        signal.dirty_scope(),
        MeshMultipathRebuildDirtyScope::PeerSet
    );
    assert_eq!(signal.affected_peer_count(), 1);
}

#[test]
fn discovery_update_mixed_existing_batch_counts_only_changed_records() {
    let unchanged = record("node-a", "198.51.100.31:443", "eu", 10, 95);
    let mut runtime = runtime_with_peers(vec![
        unchanged.clone(),
        record("node-b", "198.51.100.32:443", "eu", 20, 90),
    ]);
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery(
            "seed-c",
            &[
                unchanged,
                record("node-b", "198.51.100.42:443", "eu", 10, 99),
            ],
        )
        .unwrap_or_else(|e| unreachable!("mixed discovery should succeed: {e}"));

    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("changed discovery should set pending rebuild signal"));
    assert_eq!(signal.reason(), "peer_table_changed");
    assert_eq!(
        signal.dirty_scope(),
        MeshMultipathRebuildDirtyScope::PeerSet
    );
    assert_eq!(signal.affected_peer_count(), 1);
}

#[test]
fn peer_table_enforcement_drop_falls_back_to_unknown_dirty_scope() {
    let mut runtime = crate::MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    runtime
        .set_peer_table_policy(MeshPeerTablePolicy {
            max_entries: 1,
            max_entries_per_region: 1,
            ..MeshPeerTablePolicy::default()
        })
        .unwrap_or_else(|e| unreachable!("table policy should be accepted: {e}"));

    runtime
        .merge_discovery(
            "seed-b",
            &[
                record("node-b", "198.51.100.32:443", "eu", 20, 90),
                record("node-c", "198.51.100.33:443", "eu", 20, 90),
            ],
        )
        .unwrap_or_else(|e| unreachable!("discovery should succeed: {e}"));

    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("enforced discovery should trigger rebuild"));
    assert_eq!(signal.reason(), "peer_table_changed");
    assert_eq!(
        signal.dirty_scope(),
        MeshMultipathRebuildDirtyScope::Unknown
    );
    assert_eq!(signal.affected_peer_count(), 0);
}

#[test]
fn stale_peer_eviction_falls_back_to_unknown_dirty_scope() {
    let stale = record("node-a", "198.51.100.31:443", "eu", 10, 95);
    let mut runtime = crate::MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    runtime
        .set_peer_table_policy(MeshPeerTablePolicy {
            max_entries: 16,
            max_entries_per_region: 16,
            stale_after_ticks: 1,
            stability_window_ticks: 8,
            ..MeshPeerTablePolicy::default()
        })
        .unwrap_or_else(|e| unreachable!("table policy should be accepted: {e}"));
    runtime
        .merge_discovery("seed-b", std::slice::from_ref(&stale))
        .unwrap_or_else(|e| unreachable!("initial discovery should succeed: {e}"));
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery("seed-c", &[])
        .unwrap_or_else(|e| unreachable!("empty discovery should succeed: {e}"));
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery("seed-d", &[])
        .unwrap_or_else(|e| unreachable!("empty discovery should succeed: {e}"));

    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("stale eviction should trigger rebuild"));
    assert_eq!(signal.reason(), "peer_table_changed");
    assert_eq!(
        signal.dirty_scope(),
        MeshMultipathRebuildDirtyScope::Unknown
    );
    assert_eq!(signal.affected_peer_count(), 0);
}

#[test]
fn invalid_discovery_batch_does_not_mark_pending_rebuild() -> Result<(), String> {
    let mut runtime = runtime_with_peers(vec![record("node-a", "198.51.100.31:443", "eu", 10, 95)]);
    let _ = runtime.take_pending_multipath_rebuild_signal();
    let before_count = runtime.peer_count();

    let error = match runtime.merge_discovery(
        "seed-c",
        &[
            record("node-b", "198.51.100.32:443", "eu", 20, 99),
            record("node-b", "198.51.100.33:443", "eu", 21, 98),
        ],
    ) {
        Ok(_) => return Err("duplicate discovery batch must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("duplicate node_id"));
    assert_eq!(runtime.peer_count(), before_count);
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    Ok(())
}

#[test]
fn empty_discovery_batch_keeps_pending_rebuild_clear_on_stable_state() {
    let mut runtime = runtime_with_peers(vec![record("node-a", "198.51.100.31:443", "eu", 10, 95)]);
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery("seed-c", &[])
        .unwrap_or_else(|e| unreachable!("empty discovery should succeed: {e}"));

    assert!(runtime.pending_multipath_rebuild_signal().is_none());
}

#[test]
fn identical_existing_discovery_update_keeps_pending_rebuild_clear() {
    let unchanged = record("node-a", "198.51.100.31:443", "eu", 10, 95);
    let mut runtime = runtime_with_peers(vec![unchanged.clone()]);
    let _ = runtime.take_pending_multipath_rebuild_signal();
    let before_snapshot = runtime.peer_snapshot();

    runtime
        .merge_discovery("seed-c", &[unchanged])
        .unwrap_or_else(|e| unreachable!("identical discovery should succeed: {e}"));

    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    assert_eq!(runtime.peer_snapshot(), before_snapshot);

    let plan = runtime
        .plan_path(&request(), &crate::MeshPathPolicy::default_auto())
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    assert!(explain_has(
        &plan.explain,
        "selected_peer_stability=peer#1:u1:r0:h0:d0"
    ));
}

#[test]
fn identical_existing_discovery_update_refreshes_liveness_without_rebuild() {
    let unchanged = record("node-a", "198.51.100.31:443", "eu", 10, 95);
    let mut runtime = crate::MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    runtime
        .set_peer_table_policy(MeshPeerTablePolicy {
            max_entries: 16,
            max_entries_per_region: 16,
            stale_after_ticks: 1,
            stability_window_ticks: 8,
            ..MeshPeerTablePolicy::default()
        })
        .unwrap_or_else(|e| unreachable!("table policy should be accepted: {e}"));
    runtime
        .merge_discovery("seed-b", std::slice::from_ref(&unchanged))
        .unwrap_or_else(|e| unreachable!("initial discovery should succeed: {e}"));
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery("seed-c", std::slice::from_ref(&unchanged))
        .unwrap_or_else(|e| unreachable!("identical discovery should succeed: {e}"));
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    assert_eq!(runtime.peer_count(), 1);

    runtime
        .merge_discovery("seed-d", &[])
        .unwrap_or_else(|e| unreachable!("empty discovery should succeed: {e}"));
    assert_eq!(runtime.peer_count(), 1);
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
}

#[test]
fn changed_existing_discovery_endpoint_marks_pending_rebuild() {
    let mut runtime = runtime_with_peers(vec![record("node-a", "198.51.100.31:443", "eu", 10, 95)]);
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery(
            "seed-c",
            &[record("node-a", "198.51.100.41:443", "eu", 9, 98)],
        )
        .unwrap_or_else(|e| unreachable!("changed discovery should succeed: {e}"));

    assert_peer_table_changed_signal(&runtime);
}

#[test]
fn changed_existing_discovery_region_marks_pending_rebuild() {
    let mut runtime = runtime_with_peers(vec![record("node-a", "198.51.100.31:443", "eu", 10, 95)]);
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery(
            "seed-c",
            &[record("node-a", "198.51.100.31:443", "us", 9, 98)],
        )
        .unwrap_or_else(|e| unreachable!("changed discovery should succeed: {e}"));

    assert_peer_table_changed_signal(&runtime);
}

#[test]
fn changed_existing_discovery_load_marks_pending_rebuild() {
    let mut runtime = runtime_with_peers(vec![record("node-a", "198.51.100.31:443", "eu", 10, 95)]);
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery(
            "seed-c",
            &[record("node-a", "198.51.100.31:443", "eu", 4, 95)],
        )
        .unwrap_or_else(|e| unreachable!("changed discovery should succeed: {e}"));

    assert_peer_table_changed_signal(&runtime);
}

#[test]
fn changed_existing_discovery_reliability_marks_pending_rebuild() {
    let mut runtime = runtime_with_peers(vec![record("node-a", "198.51.100.31:443", "eu", 10, 95)]);
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery(
            "seed-c",
            &[record("node-a", "198.51.100.31:443", "eu", 9, 99)],
        )
        .unwrap_or_else(|e| unreachable!("changed discovery should succeed: {e}"));

    assert_peer_table_changed_signal(&runtime);
}

#[test]
fn same_metric_peer_replacement_marks_pending_rebuild_without_exposing_identity() {
    let mut runtime = crate::MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    runtime
        .set_peer_table_policy(MeshPeerTablePolicy {
            max_entries: 1,
            max_entries_per_region: 1,
            ..MeshPeerTablePolicy::default()
        })
        .unwrap_or_else(|e| unreachable!("table policy should be accepted: {e}"));
    runtime
        .merge_discovery(
            "seed-b",
            &[
                record("node-b", "198.51.100.32:443", "eu", 20, 90),
                record("node-c", "198.51.100.33:443", "eu", 20, 90),
            ],
        )
        .unwrap_or_else(|e| unreachable!("initial discovery should succeed: {e}"));
    assert_eq!(runtime.peer_count(), 1);
    assert_eq!(runtime.peer_snapshot()[0].node_id, "node-b");
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .merge_discovery(
            "seed-c",
            &[record("node-a", "198.51.100.31:443", "eu", 20, 90)],
        )
        .unwrap_or_else(|e| unreachable!("replacement discovery should succeed: {e}"));

    assert_eq!(runtime.peer_count(), 1);
    assert_eq!(runtime.peer_snapshot()[0].node_id, "node-a");
    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("identity-only replacement should trigger rebuild"));
    assert_eq!(signal.reason(), "peer_table_changed");
    let debug = format!("{runtime:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("node-a"));
    assert!(!debug.contains("198.51.100.31"));
}
