use crate::{
    MeshMultipathRebuildAction, MeshMultipathRebuildPolicy, MeshPeerHealth, MeshPeerPerformance,
    MeshPeerTablePolicy, MultipathDemand, MultipathMode,
};

use super::{explain_has, record, request, runtime_with_peers};

fn policy() -> MeshMultipathRebuildPolicy {
    MeshMultipathRebuildPolicy::new(3, 4)
        .unwrap_or_else(|e| unreachable!("policy should be accepted: {e}"))
}

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
    let debug = format!("{runtime:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("198.51.100.32"));
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
}

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
