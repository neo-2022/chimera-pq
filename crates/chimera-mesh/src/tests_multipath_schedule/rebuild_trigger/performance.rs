use crate::{
    MeshMultipathRebuildAction, MeshMultipathRebuildDirtyScope, MeshPeerPerformance,
    MultipathDemand, MultipathMode,
};

use super::policy;
use crate::tests_multipath_schedule::{explain_has, record, request, runtime_with_peers};

#[test]
fn performance_update_marks_pending_rebuild_and_refreshes_multipath_plan() {
    let mut runtime = runtime_with_peers(vec![
        record("node-a-slow", "198.51.100.31:443", "eu", 20, 90),
        record("node-z-fast", "198.51.100.32:443", "eu", 20, 90),
    ]);
    let _ = runtime.take_pending_multipath_rebuild_signal();
    runtime
        .update_peer_performance(&[
            MeshPeerPerformance {
                node_id: "node-a-slow".to_string(),
                latency_ms: Some(250),
                throughput_mbps: Some(40),
            },
            MeshPeerPerformance {
                node_id: "node-z-fast".to_string(),
                latency_ms: Some(30),
                throughput_mbps: Some(400),
            },
        ])
        .unwrap_or_else(|e| unreachable!("performance update should succeed: {e}"));
    assert_eq!(
        runtime
            .pending_multipath_rebuild_signal()
            .unwrap_or_else(|| unreachable!("performance should set pending rebuild"))
            .reason(),
        "peer_performance_changed"
    );

    let mut path_policy = crate::MeshPathPolicy::default_auto();
    path_policy.allowed_regions = vec!["eu".to_string()];
    path_policy.max_peers = 2;
    path_policy.max_selected_per_region = 2;
    path_policy.multipath_mode = Some(MultipathMode::AggregateBuffered);
    path_policy.multipath_demand = Some(MultipathDemand::Normal);
    let (plan, decision) = runtime
        .plan_path_with_pending_multipath_rebuild(&request(), &path_policy, &policy())
        .unwrap_or_else(|e| unreachable!("pending rebuild should apply: {e}"));

    let decision = decision.unwrap_or_else(|| unreachable!("pending decision should be present"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::AllowRebuild);
    assert!(plan.multipath_schedule.active_lane_count >= 1);
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    assert!(explain_has(
        &plan.explain,
        "multipath_rebuild_signal_reason=peer_performance_changed"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_rebuild_dirty_scope=peer_set"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_rebuild_affected_peer_count=2"
    ));
}

#[test]
fn performance_update_mixed_batch_counts_only_changed_records() {
    let mut runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 18, 95),
    ]);
    let _ = runtime.take_pending_multipath_rebuild_signal();
    runtime
        .update_peer_performance(&[
            MeshPeerPerformance {
                node_id: "node-a".to_string(),
                latency_ms: Some(40),
                throughput_mbps: Some(500),
            },
            MeshPeerPerformance {
                node_id: "node-b".to_string(),
                latency_ms: Some(20),
                throughput_mbps: Some(800),
            },
        ])
        .unwrap_or_else(|e| unreachable!("performance update should succeed: {e}"));
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .update_peer_performance(&[
            MeshPeerPerformance {
                node_id: "node-a".to_string(),
                latency_ms: Some(40),
                throughput_mbps: Some(500),
            },
            MeshPeerPerformance {
                node_id: "node-b".to_string(),
                latency_ms: Some(15),
                throughput_mbps: Some(850),
            },
        ])
        .unwrap_or_else(|e| unreachable!("performance update should succeed: {e}"));

    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("changed performance should set pending rebuild signal"));
    assert_eq!(signal.reason(), "peer_performance_changed");
    assert_eq!(
        signal.dirty_scope(),
        MeshMultipathRebuildDirtyScope::PeerSet
    );
    assert_eq!(signal.affected_peer_count(), 1);
}
