use crate::{
    MeshMultipathRebuildAction, MeshMultipathRebuildDirtyMetadata, MeshMultipathRebuildDirtyScope,
    MeshMultipathRebuildPolicy, MeshMultipathRebuildSignal, MeshRuntime, MultipathDemand,
    MultipathMode,
};

use super::{explain_has, record, request, runtime_with_peers, seeded_runtime};

fn policy() -> MeshMultipathRebuildPolicy {
    MeshMultipathRebuildPolicy::new(3, 4)
        .unwrap_or_else(|e| unreachable!("policy should be accepted: {e}"))
}

fn soft_signal(
    reason: &str,
    generation: u64,
    fingerprint: u64,
    epoch: u64,
    observed_tick: u64,
) -> MeshMultipathRebuildSignal {
    MeshMultipathRebuildSignal::soft(reason, generation, fingerprint, epoch, observed_tick)
        .unwrap_or_else(|e| unreachable!("signal should be accepted: {e}"))
}

fn soft_peer_set_signal(
    reason: &str,
    generation: u64,
    fingerprint: u64,
    epoch: u64,
    observed_tick: u64,
    affected_peer_count: usize,
) -> MeshMultipathRebuildSignal {
    MeshMultipathRebuildSignal::soft_with_dirty_scope(
        reason,
        generation,
        fingerprint,
        epoch,
        observed_tick,
        MeshMultipathRebuildDirtyMetadata::peer_set(affected_peer_count)
            .unwrap_or_else(|e| unreachable!("dirty metadata should be accepted: {e}")),
    )
    .unwrap_or_else(|e| unreachable!("signal should be accepted: {e}"))
}

fn urgent_signal(observed_tick: u64) -> MeshMultipathRebuildSignal {
    MeshMultipathRebuildSignal::urgent_failover(
        "urgent_failover",
        10,
        0xfeed_face,
        3,
        observed_tick,
    )
    .unwrap_or_else(|e| unreachable!("signal should be accepted: {e}"))
}

fn hard_signal(observed_tick: u64) -> MeshMultipathRebuildSignal {
    MeshMultipathRebuildSignal::hard_safety(
        "route_binding_mismatch",
        10,
        0xfeed_face,
        3,
        observed_tick,
    )
    .unwrap_or_else(|e| unreachable!("signal should be accepted: {e}"))
}

fn runtime_for_bridge() -> MeshRuntime {
    runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
        record("node-c", "198.51.100.33:443", "eu", 24, 92),
        record("node-d", "198.51.100.34:443", "eu", 26, 93),
    ])
}

fn advance_tick(runtime: &mut MeshRuntime, source: &str) {
    runtime
        .merge_discovery(source, &[])
        .unwrap_or_else(|e| unreachable!("empty discovery tick should succeed: {e}"));
}

#[test]
fn first_rebuild_signal_is_allowed() {
    let mut runtime = seeded_runtime();
    let signal = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);

    let decision = runtime
        .evaluate_multipath_rebuild(&signal, &policy())
        .unwrap_or_else(|e| unreachable!("rebuild gate should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::AllowRebuild);
    assert!(decision.rebuild_allowed);
    assert_eq!(decision.reason, "initial_observation");
    assert!(!decision.debounced);
    assert!(!decision.stale);
    assert_eq!(decision.pending_count, 0);
    assert_eq!(
        decision.dirty_scope,
        MeshMultipathRebuildDirtyScope::Unknown
    );
    assert_eq!(decision.affected_peer_count, 0);
    assert!(
        decision
            .explain
            .iter()
            .any(|line| line == "multipath_rebuild_action=allow_rebuild")
    );
    assert!(explain_has(
        &decision.explain,
        "multipath_rebuild_dirty_scope=unknown"
    ));
    assert!(explain_has(
        &decision.explain,
        "multipath_rebuild_affected_peer_count=0"
    ));
}

#[test]
fn peer_set_dirty_scope_is_redacted_and_explained() {
    let mut runtime = seeded_runtime();
    let signal = soft_peer_set_signal("peer_table_changed", 1, 0x1001, 1, 1, 2);

    let decision = runtime
        .evaluate_multipath_rebuild(&signal, &policy())
        .unwrap_or_else(|e| unreachable!("rebuild gate should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::AllowRebuild);
    assert_eq!(
        decision.dirty_scope,
        MeshMultipathRebuildDirtyScope::PeerSet
    );
    assert_eq!(decision.affected_peer_count, 2);
    assert!(explain_has(
        &decision.explain,
        "multipath_rebuild_dirty_scope=peer_set"
    ));
    assert!(explain_has(
        &decision.explain,
        "multipath_rebuild_affected_peer_count=2"
    ));
    let debug = format!("{decision:?}");
    assert!(debug.contains("affected_peer_count"));
    assert!(!debug.contains("node-a"));
    assert!(!debug.contains("198.51."));
    assert!(!decision.explain.iter().any(|line| line.contains("0x1001")));
}

#[test]
fn peer_set_dirty_scope_rejects_zero_affected_peers() {
    assert!(MeshMultipathRebuildDirtyMetadata::peer_set(0).is_err());
}

#[test]
fn unknown_dirty_scope_rejects_nonzero_affected_peers() {
    let signal = MeshMultipathRebuildSignal::soft_with_dirty_scope(
        "peer_table_changed",
        1,
        0x1001,
        1,
        1,
        MeshMultipathRebuildDirtyMetadata::unknown(),
    )
    .unwrap_or_else(|e| unreachable!("unknown dirty metadata should be accepted: {e}"));
    assert_eq!(
        signal.dirty_scope(),
        MeshMultipathRebuildDirtyScope::Unknown
    );
    assert_eq!(signal.affected_peer_count(), 0);
}

#[test]
fn allowed_rebuild_bridge_refreshes_schedule_from_current_plan() {
    let mut runtime = runtime_for_bridge();
    let mut path_policy = crate::MeshPathPolicy::default_auto();
    path_policy.allowed_regions = vec!["eu".to_string()];
    path_policy.max_peers = 4;
    path_policy.max_selected_per_region = 4;
    path_policy.multipath_mode = Some(MultipathMode::AggregateBuffered);
    path_policy.multipath_demand = Some(MultipathDemand::Low);
    let mut plan = runtime
        .plan_path(&request(), &path_policy)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    plan.multipath_schedule.demand_rebuild_recommended = false;
    plan.multipath_schedule.planner_rebuild_reason = "stale_test_value".to_string();
    let signal = soft_signal("capacity_pressure", 1, 0x1001, 1, 1);

    let decision = runtime
        .apply_multipath_rebuild_to_plan(&mut plan, &signal, &policy())
        .unwrap_or_else(|e| unreachable!("bridge should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::AllowRebuild);
    assert_eq!(plan.multipath_schedule.demand_policy, "low");
    assert_eq!(
        plan.multipath_schedule.demand_policy_source,
        "control_policy"
    );
    assert_eq!(plan.multipath_schedule.active_lane_count, 1);
    assert!(plan.multipath_schedule.demand_rebuild_recommended);
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_planner_rebuild_reason=multipath_hint_replan"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_rebuild_action=allow_rebuild"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_rebuild_privacy=aggregate_only"
    ));
    assert!(!plan.explain.iter().any(|line| line.contains("0x1001")));
}

#[test]
fn allowed_rebuild_with_policy_refreshes_selected_peers_before_schedule_rebuild() {
    let mut runtime = runtime_for_bridge();
    let mut path_policy = crate::MeshPathPolicy::default_auto();
    path_policy.allowed_regions = vec!["eu".to_string()];
    path_policy.max_peers = 4;
    path_policy.max_selected_per_region = 4;
    path_policy.multipath_mode = Some(MultipathMode::FlowShard);
    path_policy.multipath_demand = Some(MultipathDemand::Normal);
    let mut plan = runtime
        .plan_path(&request(), &path_policy)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    let original_peer_ids: Vec<String> = plan
        .selected_peers
        .iter()
        .map(|peer| peer.node_id.clone())
        .collect();
    runtime
        .merge_discovery(
            "seed-c",
            &[
                record("node-a", "198.51.100.31:443", "eu", 95, 40),
                record("node-b", "198.51.100.32:443", "eu", 12, 99),
                record("node-c", "198.51.100.33:443", "eu", 13, 98),
                record("node-d", "198.51.100.34:443", "eu", 14, 97),
            ],
        )
        .unwrap_or_else(|e| unreachable!("runtime refresh should succeed: {e}"));
    let signal = soft_signal("capacity_pressure", 2, 0x2002, 1, 2);

    let decision = runtime
        .apply_multipath_rebuild_with_policy_to_plan(
            &request(),
            &path_policy,
            &mut plan,
            &signal,
            &policy(),
        )
        .unwrap_or_else(|e| unreachable!("policy-aware bridge should evaluate: {e}"));

    let rebuilt_peer_ids: Vec<String> = plan
        .selected_peers
        .iter()
        .map(|peer| peer.node_id.clone())
        .collect();

    assert_eq!(decision.action, MeshMultipathRebuildAction::AllowRebuild);
    assert_ne!(rebuilt_peer_ids, original_peer_ids);
    assert_eq!(
        plan.multipath_schedule.transit_payload_policy,
        "sealed_opaque_only"
    );
    assert_eq!(plan.multipath_schedule.local_traffic_reserve_pct, 10);
    assert_eq!(plan.multipath_schedule.transit_capacity_budget_pct, 90);
    assert_eq!(
        plan.multipath_schedule.local_traffic_reserve_pct
            + plan.multipath_schedule.transit_capacity_budget_pct,
        100
    );
    assert!(plan.explain.iter().all(|line| !line.contains("0x2002")));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_planner_rebuild_reason=multipath_hint_replan"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_rebuild_privacy=aggregate_only"
    ));
}

#[test]
fn suppressed_rebuild_bridge_preserves_existing_schedule() {
    let mut runtime = runtime_for_bridge();
    let mut path_policy = crate::MeshPathPolicy::default_auto();
    path_policy.allowed_regions = vec!["eu".to_string()];
    path_policy.max_peers = 4;
    path_policy.max_selected_per_region = 4;
    path_policy.multipath_mode = Some(MultipathMode::FlowShard);
    path_policy.multipath_demand = Some(MultipathDemand::High);
    let mut plan = runtime
        .plan_path(&request(), &path_policy)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    let before = plan.multipath_schedule.clone();
    let first = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);
    let duplicate = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);

    runtime
        .apply_multipath_rebuild_to_plan(&mut plan, &first, &policy())
        .unwrap_or_else(|e| unreachable!("first bridge signal should evaluate: {e}"));
    let refreshed = plan.multipath_schedule.clone();
    let decision = runtime
        .apply_multipath_rebuild_to_plan(&mut plan, &duplicate, &policy())
        .unwrap_or_else(|e| unreachable!("duplicate bridge signal should evaluate: {e}"));

    assert_eq!(before.mode, refreshed.mode);
    assert_eq!(decision.action, MeshMultipathRebuildAction::SuppressRebuild);
    assert_eq!(plan.multipath_schedule, refreshed);
    assert!(explain_has(
        &plan.explain,
        "multipath_rebuild_action=suppress_rebuild"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_rebuild_debounced=true"
    ));
}

#[test]
fn duplicate_soft_signal_inside_debounce_window_is_suppressed() {
    let mut runtime = seeded_runtime();
    let first = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);
    let duplicate = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);

    let first_decision = runtime
        .evaluate_multipath_rebuild(&first, &policy())
        .unwrap_or_else(|e| unreachable!("first signal should evaluate: {e}"));
    let second_decision = runtime
        .evaluate_multipath_rebuild(&duplicate, &policy())
        .unwrap_or_else(|e| unreachable!("duplicate signal should evaluate: {e}"));

    assert_eq!(
        first_decision.action,
        MeshMultipathRebuildAction::AllowRebuild
    );
    assert_eq!(
        second_decision.action,
        MeshMultipathRebuildAction::SuppressRebuild
    );
    assert_eq!(second_decision.reason, "debounced_same_fingerprint");
    assert!(second_decision.debounced);
    assert_eq!(second_decision.pending_count, 1);
}

#[test]
fn duplicate_soft_signal_after_debounce_window_is_allowed() {
    let mut runtime = seeded_runtime();
    let signal = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);

    runtime
        .evaluate_multipath_rebuild(&signal, &policy())
        .unwrap_or_else(|e| unreachable!("first signal should evaluate: {e}"));
    advance_tick(&mut runtime, "seed-c");
    advance_tick(&mut runtime, "seed-d");
    advance_tick(&mut runtime, "seed-e");
    let later = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 4);
    let decision = runtime
        .evaluate_multipath_rebuild(&later, &policy())
        .unwrap_or_else(|e| unreachable!("later signal should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::AllowRebuild);
    assert_eq!(decision.reason, "debounce_window_elapsed");
    assert!(!decision.debounced);
    assert_eq!(decision.pending_count, 0);
}

#[test]
fn changed_reason_generation_or_fingerprint_bypasses_duplicate_suppression() {
    let mut runtime = seeded_runtime();
    let first = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);
    let reason_changed = soft_signal("capacity_pressure", 1, 0x1001, 1, 1);
    let generation_changed = soft_signal("capacity_pressure", 2, 0x1001, 1, 1);
    let fingerprint_changed = soft_signal("capacity_pressure", 2, 0x2002, 1, 1);

    runtime
        .evaluate_multipath_rebuild(&first, &policy())
        .unwrap_or_else(|e| unreachable!("first signal should evaluate: {e}"));
    let by_reason = runtime
        .evaluate_multipath_rebuild(&reason_changed, &policy())
        .unwrap_or_else(|e| unreachable!("reason change should evaluate: {e}"));
    let by_generation = runtime
        .evaluate_multipath_rebuild(&generation_changed, &policy())
        .unwrap_or_else(|e| unreachable!("generation change should evaluate: {e}"));
    let by_fingerprint = runtime
        .evaluate_multipath_rebuild(&fingerprint_changed, &policy())
        .unwrap_or_else(|e| unreachable!("fingerprint change should evaluate: {e}"));

    assert_eq!(by_reason.action, MeshMultipathRebuildAction::AllowRebuild);
    assert_eq!(by_reason.reason, "reason_changed");
    assert_eq!(
        by_generation.action,
        MeshMultipathRebuildAction::AllowRebuild
    );
    assert_eq!(by_generation.reason, "generation_changed");
    assert_eq!(
        by_fingerprint.action,
        MeshMultipathRebuildAction::AllowRebuild
    );
    assert_eq!(by_fingerprint.reason, "fingerprint_changed");
    assert!(by_fingerprint.fingerprint_changed);
}

#[test]
fn stale_telemetry_fails_closed_instead_of_using_debounce() {
    let mut runtime = seeded_runtime();
    for source in ["seed-c", "seed-d", "seed-e", "seed-f", "seed-g"] {
        advance_tick(&mut runtime, source);
    }
    let stale = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);

    let decision = runtime
        .evaluate_multipath_rebuild(&stale, &policy())
        .unwrap_or_else(|e| unreachable!("stale signal should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::FailClosed);
    assert_eq!(decision.reason, "stale_telemetry");
    assert!(decision.stale);
    assert!(!decision.rebuild_allowed);
}

#[test]
fn telemetry_from_future_fails_closed() {
    let mut runtime = seeded_runtime();
    let future = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 100);

    let decision = runtime
        .evaluate_multipath_rebuild(&future, &policy())
        .unwrap_or_else(|e| unreachable!("future signal should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::FailClosed);
    assert_eq!(decision.reason, "telemetry_from_future");
    assert!(decision.stale);
    assert!(!decision.rebuild_allowed);
}

#[test]
fn urgent_failover_bypasses_debounce() {
    let mut runtime = seeded_runtime();
    let first = soft_signal("demand_rebuild_recommended", 10, 0xfeed_face, 3, 1);
    let urgent = urgent_signal(1);

    runtime
        .evaluate_multipath_rebuild(&first, &policy())
        .unwrap_or_else(|e| unreachable!("first signal should evaluate: {e}"));
    let decision = runtime
        .evaluate_multipath_rebuild(&urgent, &policy())
        .unwrap_or_else(|e| unreachable!("urgent signal should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::AllowRebuild);
    assert_eq!(decision.reason, "urgent_failover");
    assert!(decision.rebuild_allowed);
    assert!(!decision.debounced);
}

#[test]
fn hard_safety_signal_fails_closed_without_debounce_delay() {
    let mut runtime = seeded_runtime();
    let first = soft_signal("demand_rebuild_recommended", 10, 0xfeed_face, 3, 1);
    let hard = hard_signal(1);

    runtime
        .evaluate_multipath_rebuild(&first, &policy())
        .unwrap_or_else(|e| unreachable!("first signal should evaluate: {e}"));
    let decision = runtime
        .evaluate_multipath_rebuild(&hard, &policy())
        .unwrap_or_else(|e| unreachable!("hard signal should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::FailClosed);
    assert_eq!(decision.reason, "hard_safety_fail_closed");
    assert!(!decision.rebuild_allowed);
    assert!(!decision.debounced);
}

#[test]
fn rebuild_control_diagnostics_are_aggregate_and_redacted() {
    let mut runtime = seeded_runtime();
    let signal = soft_signal("demand_rebuild_recommended", 7009, 0xdead_beef, 5, 1);

    let decision = runtime
        .evaluate_multipath_rebuild(&signal, &policy())
        .unwrap_or_else(|e| unreachable!("signal should evaluate: {e}"));
    let explain = decision.explain.join("|");
    let debug_signal = format!("{signal:?}");
    let debug_decision = format!("{decision:?}");

    assert!(explain.contains("multipath_rebuild_privacy=aggregate_only"));
    assert!(explain.contains("multipath_rebuild_generation_changed=true"));
    assert!(!explain.contains("dead_beef"));
    assert!(!explain.contains("0xdead"));
    assert!(!explain.contains("198.51.100.31"));
    assert!(!explain.contains("node-a"));
    assert!(!explain.contains("7009"));
    assert!(!debug_signal.contains("dead_beef"));
    assert!(!debug_signal.contains("0xdead"));
    assert!(debug_signal.contains("<redacted>"));
    assert!(!debug_decision.contains("dead_beef"));
    assert!(!debug_decision.contains("198.51.100.31"));
}

#[test]
fn runtime_debug_redacts_rebuild_state_fingerprint() {
    let mut runtime = seeded_runtime();
    let signal = soft_signal("demand_rebuild_recommended", 7009, 0xdead_beef, 5, 1);

    runtime
        .evaluate_multipath_rebuild(&signal, &policy())
        .unwrap_or_else(|e| unreachable!("signal should evaluate: {e}"));
    let debug = format!("{runtime:?}");

    assert!(debug.contains("schedule_fingerprint"));
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("dead_beef"));
    assert!(!debug.contains("3735928559"));
    assert!(!debug.contains("198.51.100.31"));
    assert!(!debug.contains("node-a"));
}

#[test]
fn invalid_rebuild_policy_and_reason_are_rejected() {
    assert!(MeshMultipathRebuildPolicy::new(0, 4).is_err());
    assert!(MeshMultipathRebuildPolicy::new(3, 0).is_err());
    assert!(
        MeshMultipathRebuildSignal::soft("Raw Endpoint 198.51.100.31:443", 1, 1, 1, 1).is_err()
    );
    assert!(MeshMultipathRebuildSignal::soft(" capacity_pressure", 1, 1, 1, 1).is_err());
    assert!(MeshMultipathRebuildSignal::soft("capacity_pressure\n", 1, 1, 1, 1).is_err());
    assert!(MeshMultipathRebuildSignal::soft("route_7009", 1, 1, 1, 1).is_err());
    assert!(MeshMultipathRebuildSignal::soft("peer_123", 1, 1, 1, 1).is_err());
    assert!(MeshMultipathRebuildSignal::soft("dead_beef", 1, 1, 1, 1).is_err());
    assert!(MeshMultipathRebuildSignal::soft("payload_secret", 1, 1, 1, 1).is_err());
    assert!(MeshMultipathRebuildSignal::soft("capacity_pressure", 1, 1, 1, 2).is_ok());
}
