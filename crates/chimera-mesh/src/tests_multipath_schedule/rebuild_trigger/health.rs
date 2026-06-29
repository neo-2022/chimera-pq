use crate::{MeshMultipathRebuildDirtyScope, MeshPeerHealth, MeshPeerPerformance, MultipathMode};

use super::policy;
use crate::tests_multipath_schedule::{explain_has, record, request, runtime_with_peers};

#[test]
fn unhealthy_update_marks_urgent_pending_rebuild_and_reselects_peer() {
    let mut runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 10, 99),
        record("node-b", "198.51.100.32:443", "eu", 20, 90),
    ]);
    let _ = runtime.take_pending_multipath_rebuild_signal();
    runtime
        .update_health_state(&[MeshPeerHealth {
            node_id: "node-a".to_string(),
            healthy: false,
            cooldown_active: true,
        }])
        .unwrap_or_else(|e| unreachable!("health update should succeed: {e}"));

    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("health should set pending rebuild"));
    assert_eq!(signal.reason(), "urgent_failover");
    assert_eq!(
        signal.urgency(),
        crate::MeshMultipathRebuildUrgency::UrgentFailover
    );

    let mut path_policy = crate::MeshPathPolicy::default_auto();
    path_policy.allowed_regions = vec!["eu".to_string()];
    path_policy.max_peers = 2;
    path_policy.max_selected_per_region = 2;
    path_policy.multipath_mode = Some(MultipathMode::FlowShard);
    let (plan, decision) = runtime
        .plan_path_with_pending_multipath_rebuild(&request(), &path_policy, &policy())
        .unwrap_or_else(|e| unreachable!("urgent pending rebuild should apply: {e}"));
    let decision = decision.unwrap_or_else(|| unreachable!("pending decision should be present"));

    assert_eq!(decision.reason, "urgent_failover");
    assert!(plan.multipath_schedule.active_lane_count >= 1);
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    assert!(explain_has(
        &plan.explain,
        "multipath_rebuild_signal_reason=urgent_failover"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_rebuild_dirty_scope=peer_set"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_rebuild_affected_peer_count=1"
    ));
}

#[test]
fn health_update_mixed_batch_counts_only_changed_records() {
    let mut runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 18, 95),
    ]);
    let _ = runtime.take_pending_multipath_rebuild_signal();
    runtime
        .update_health_state(&[
            MeshPeerHealth {
                node_id: "node-a".to_string(),
                healthy: true,
                cooldown_active: false,
            },
            MeshPeerHealth {
                node_id: "node-b".to_string(),
                healthy: true,
                cooldown_active: false,
            },
        ])
        .unwrap_or_else(|e| unreachable!("health update should succeed: {e}"));
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .update_health_state(&[
            MeshPeerHealth {
                node_id: "node-a".to_string(),
                healthy: true,
                cooldown_active: false,
            },
            MeshPeerHealth {
                node_id: "node-b".to_string(),
                healthy: false,
                cooldown_active: true,
            },
        ])
        .unwrap_or_else(|e| unreachable!("health update should succeed: {e}"));

    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("changed health should set pending rebuild signal"));
    assert_eq!(signal.reason(), "urgent_failover");
    assert_eq!(
        signal.dirty_scope(),
        MeshMultipathRebuildDirtyScope::PeerSet
    );
    assert_eq!(signal.affected_peer_count(), 1);
}

#[test]
fn invalid_health_batch_is_atomic_and_does_not_mark_pending_rebuild() -> Result<(), String> {
    let mut runtime = runtime_with_peers(vec![record("node-a", "198.51.100.31:443", "eu", 10, 95)]);
    let _ = runtime.take_pending_multipath_rebuild_signal();

    let error = match runtime.update_health_state(&[
        MeshPeerHealth {
            node_id: "node-a".to_string(),
            healthy: false,
            cooldown_active: true,
        },
        MeshPeerHealth {
            node_id: "bad\nnode".to_string(),
            healthy: false,
            cooldown_active: true,
        },
    ]) {
        Ok(_) => return Err("invalid health batch must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("node_id"));
    assert_eq!(runtime.health_state_count(), 0);
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    Ok(())
}

#[test]
fn health_update_for_unknown_peer_is_rejected_without_pending_rebuild() -> Result<(), String> {
    let mut runtime = runtime_with_peers(vec![record("node-a", "198.51.100.31:443", "eu", 10, 95)]);
    let _ = runtime.take_pending_multipath_rebuild_signal();

    let error = match runtime.update_health_state(&[MeshPeerHealth {
        node_id: "missing-node".to_string(),
        healthy: false,
        cooldown_active: true,
    }]) {
        Ok(_) => return Err("unknown peer health update must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("unknown node"));
    assert_eq!(runtime.health_state_count(), 0);
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    Ok(())
}

#[test]
fn urgent_pending_rebuild_is_not_downgraded_by_soft_performance_update() {
    let mut runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 10, 99),
        record("node-b", "198.51.100.32:443", "eu", 20, 90),
    ]);
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .update_health_state(&[MeshPeerHealth {
            node_id: "node-a".to_string(),
            healthy: false,
            cooldown_active: true,
        }])
        .unwrap_or_else(|e| unreachable!("health update should succeed: {e}"));
    runtime
        .update_peer_performance(&[MeshPeerPerformance {
            node_id: "node-b".to_string(),
            latency_ms: Some(20),
            throughput_mbps: Some(300),
        }])
        .unwrap_or_else(|e| unreachable!("performance update should succeed: {e}"));

    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("pending rebuild should remain"));
    assert_eq!(signal.reason(), "urgent_failover");
    assert_eq!(
        signal.urgency(),
        crate::MeshMultipathRebuildUrgency::UrgentFailover
    );
}

#[test]
fn soft_pending_rebuild_is_promoted_by_urgent_health_update() {
    let mut runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 10, 99),
        record("node-b", "198.51.100.32:443", "eu", 20, 90),
    ]);
    let _ = runtime.take_pending_multipath_rebuild_signal();

    runtime
        .update_peer_performance(&[MeshPeerPerformance {
            node_id: "node-b".to_string(),
            latency_ms: Some(20),
            throughput_mbps: Some(300),
        }])
        .unwrap_or_else(|e| unreachable!("performance update should succeed: {e}"));
    runtime
        .update_health_state(&[MeshPeerHealth {
            node_id: "node-a".to_string(),
            healthy: false,
            cooldown_active: true,
        }])
        .unwrap_or_else(|e| unreachable!("health update should succeed: {e}"));

    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("pending rebuild should remain"));
    assert_eq!(signal.reason(), "urgent_failover");
    assert_eq!(
        signal.urgency(),
        crate::MeshMultipathRebuildUrgency::UrgentFailover
    );
}

#[test]
fn stale_pending_rebuild_fails_closed_without_returning_plan() {
    let mut runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 20, 90),
    ]);
    let _ = runtime.take_pending_multipath_rebuild_signal();
    runtime
        .update_peer_performance(&[MeshPeerPerformance {
            node_id: "node-b".to_string(),
            latency_ms: Some(30),
            throughput_mbps: Some(400),
        }])
        .unwrap_or_else(|e| unreachable!("performance update should succeed: {e}"));
    for source in ["seed-c", "seed-d", "seed-e", "seed-f", "seed-g"] {
        runtime
            .merge_discovery(source, &[])
            .unwrap_or_else(|e| unreachable!("empty discovery tick should succeed: {e}"));
    }

    let mut path_policy = crate::MeshPathPolicy::default_auto();
    path_policy.allowed_regions = vec!["eu".to_string()];
    path_policy.max_peers = 2;
    path_policy.max_selected_per_region = 2;
    path_policy.multipath_mode = Some(MultipathMode::AggregateBuffered);
    let error =
        match runtime.plan_path_with_pending_multipath_rebuild(&request(), &path_policy, &policy())
        {
            Ok(_) => unreachable!("stale pending rebuild must fail closed"),
            Err(error) => error,
        };

    assert!(error.contains("failed closed"));
    assert!(error.contains("stale_telemetry"));
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
}
