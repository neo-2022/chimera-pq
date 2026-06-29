use crate::{MeshMultipathRebuildAction, MeshRuntime, MultipathDemand, MultipathMode};

use super::{explain_has, policy, record, request, runtime_with_peers, soft_signal};

fn runtime_for_bridge() -> MeshRuntime {
    runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
        record("node-c", "198.51.100.33:443", "eu", 24, 92),
        record("node-d", "198.51.100.34:443", "eu", 26, 93),
    ])
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
fn core_rebuild_with_policy_matches_full_plan_schedule_without_explain_churn() {
    let mut full_runtime = runtime_for_bridge();
    let mut core_runtime = runtime_for_bridge();
    let mut path_policy = crate::MeshPathPolicy::default_auto();
    path_policy.allowed_regions = vec!["eu".to_string()];
    path_policy.max_peers = 4;
    path_policy.max_selected_per_region = 4;
    path_policy.multipath_mode = Some(MultipathMode::FlowShard);
    path_policy.multipath_demand = Some(MultipathDemand::Normal);
    let refreshed_records = [
        record("node-a", "198.51.100.31:443", "eu", 95, 40),
        record("node-b", "198.51.100.32:443", "eu", 12, 99),
        record("node-c", "198.51.100.33:443", "eu", 13, 98),
        record("node-d", "198.51.100.34:443", "eu", 14, 97),
    ];
    full_runtime
        .merge_discovery("seed-c", &refreshed_records)
        .unwrap_or_else(|e| unreachable!("runtime refresh should succeed: {e}"));
    core_runtime
        .merge_discovery("seed-c", &refreshed_records)
        .unwrap_or_else(|e| unreachable!("runtime refresh should succeed: {e}"));
    let signal = soft_signal("capacity_pressure", 2, 0x2002, 1, 2);
    let mut full_plan = full_runtime
        .plan_path(&request(), &path_policy)
        .unwrap_or_else(|e| unreachable!("full planning should succeed: {e}"));
    let mut core_plan = core_runtime
        .plan_path_core(&request(), &path_policy)
        .unwrap_or_else(|e| unreachable!("core planning should succeed: {e}"));

    let full_decision = full_runtime
        .apply_multipath_rebuild_with_policy_to_plan(
            &request(),
            &path_policy,
            &mut full_plan,
            &signal,
            &policy(),
        )
        .unwrap_or_else(|e| unreachable!("full rebuild should succeed: {e}"));
    let core_decision = core_runtime
        .apply_multipath_rebuild_with_policy_to_plan_core(
            &request(),
            &path_policy,
            &mut core_plan,
            &signal,
            &policy(),
        )
        .unwrap_or_else(|e| unreachable!("core rebuild should succeed: {e}"));

    assert_eq!(
        full_decision.action,
        MeshMultipathRebuildAction::AllowRebuild
    );
    assert_eq!(
        core_decision.action,
        MeshMultipathRebuildAction::AllowRebuild
    );
    assert_eq!(core_decision.reason, full_decision.reason);
    assert_eq!(core_plan.selected_peers, full_plan.selected_peers);
    assert_eq!(core_plan.multipath_schedule, full_plan.multipath_schedule);
    assert!(explain_has(
        &full_plan.explain,
        "multipath_rebuild_privacy=aggregate_only"
    ));
}

#[test]
fn core_rebuild_with_policy_matches_full_plan_schedule_without_explain_plan() {
    let runtime = runtime_for_bridge();
    let mut full_runtime = runtime.clone();
    let mut core_runtime = runtime;
    let mut path_policy = crate::MeshPathPolicy::default_auto();
    path_policy.allowed_regions = vec!["eu".to_string()];
    path_policy.max_peers = 4;
    path_policy.max_selected_per_region = 4;
    path_policy.multipath_mode = Some(MultipathMode::AggregateBuffered);
    path_policy.multipath_demand = Some(MultipathDemand::High);
    let mut full_plan = full_runtime
        .plan_path(&request(), &path_policy)
        .unwrap_or_else(|e| unreachable!("full planning should succeed: {e}"));
    let mut core_plan = core_runtime
        .plan_path_core(&request(), &path_policy)
        .unwrap_or_else(|e| unreachable!("core planning should succeed: {e}"));
    let route_binding_id = crate::MeshRouteBindingId::new(0x7001)
        .unwrap_or_else(|e| unreachable!("route binding id should parse: {e}"));
    full_plan.multipath_schedule.route_binding_id = Some(route_binding_id);
    core_plan.multipath_schedule.route_binding_id = Some(route_binding_id);

    for runtime in [&mut full_runtime, &mut core_runtime] {
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
    }
    let signal = soft_signal("capacity_pressure", 3, 0x3003, 1, 2);

    let full_decision = full_runtime
        .apply_multipath_rebuild_with_policy_to_plan(
            &request(),
            &path_policy,
            &mut full_plan,
            &signal,
            &policy(),
        )
        .unwrap_or_else(|e| unreachable!("full bridge should evaluate: {e}"));
    let core_decision = core_runtime
        .apply_multipath_rebuild_with_policy_to_plan_core(
            &request(),
            &path_policy,
            &mut core_plan,
            &signal,
            &policy(),
        )
        .unwrap_or_else(|e| unreachable!("core bridge should evaluate: {e}"));

    let full_peer_ids: Vec<&str> = full_plan
        .selected_peers
        .iter()
        .map(|peer| peer.node_id.as_str())
        .collect();
    let core_peer_ids: Vec<&str> = core_plan
        .selected_peers
        .iter()
        .map(|peer| peer.node_id.as_str())
        .collect();

    assert_eq!(
        full_decision.action,
        MeshMultipathRebuildAction::AllowRebuild
    );
    assert_eq!(core_decision.action, full_decision.action);
    assert_eq!(core_decision.reason, full_decision.reason);
    assert_eq!(core_peer_ids, full_peer_ids);
    assert_eq!(core_plan.multipath_schedule, full_plan.multipath_schedule);
    assert!(explain_has(
        &full_plan.explain,
        "multipath_rebuild_action=allow_rebuild"
    ));
    assert!(
        core_decision
            .explain
            .iter()
            .all(|line| !line.contains("0x3003"))
    );
}

#[test]
fn core_pending_rebuild_reselects_peer_and_clears_pending_signal() {
    let mut runtime = runtime_for_bridge();
    let _ = runtime.take_pending_multipath_rebuild_signal();
    runtime
        .update_peer_performance(&[
            crate::MeshPeerPerformance {
                node_id: "node-a".to_string(),
                latency_ms: Some(250),
                throughput_mbps: Some(40),
            },
            crate::MeshPeerPerformance {
                node_id: "node-b".to_string(),
                latency_ms: Some(20),
                throughput_mbps: Some(900),
            },
        ])
        .unwrap_or_else(|e| unreachable!("performance update should succeed: {e}"));

    let mut path_policy = crate::MeshPathPolicy::default_auto();
    path_policy.allowed_regions = vec!["eu".to_string()];
    path_policy.max_peers = 4;
    path_policy.max_selected_per_region = 4;
    path_policy.multipath_mode = Some(MultipathMode::FlowShard);
    path_policy.multipath_demand = Some(MultipathDemand::Normal);
    let (plan, decision) = runtime
        .plan_path_core_with_pending_multipath_rebuild(&request(), &path_policy, &policy())
        .unwrap_or_else(|e| unreachable!("core pending rebuild should apply: {e}"));
    let decision = decision.unwrap_or_else(|| unreachable!("pending decision should be present"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::AllowRebuild);
    assert!(plan.multipath_schedule.active_lane_count >= 1);
    assert_eq!(
        plan.multipath_schedule.transit_payload_policy,
        "sealed_opaque_only"
    );
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
    assert!(
        decision
            .explain
            .iter()
            .any(|line| line.contains("multipath_rebuild_signal_reason=peer_performance_changed"))
    );
    assert!(
        decision
            .explain
            .iter()
            .all(|line| !line.contains("198.51."))
    );
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
fn suppressed_core_rebuild_preserves_existing_schedule() {
    let mut runtime = runtime_for_bridge();
    let mut path_policy = crate::MeshPathPolicy::default_auto();
    path_policy.allowed_regions = vec!["eu".to_string()];
    path_policy.max_peers = 4;
    path_policy.max_selected_per_region = 4;
    path_policy.multipath_mode = Some(MultipathMode::FlowShard);
    path_policy.multipath_demand = Some(MultipathDemand::High);
    let mut plan = runtime
        .plan_path_core(&request(), &path_policy)
        .unwrap_or_else(|e| unreachable!("core planning should succeed: {e}"));
    let first = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);
    let duplicate = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);

    runtime
        .apply_multipath_rebuild_to_plan_core(&mut plan, &first, &policy())
        .unwrap_or_else(|e| unreachable!("first core bridge signal should evaluate: {e}"));
    let refreshed = plan.multipath_schedule.clone();
    let decision = runtime
        .apply_multipath_rebuild_to_plan_core(&mut plan, &duplicate, &policy())
        .unwrap_or_else(|e| unreachable!("duplicate core bridge signal should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::SuppressRebuild);
    assert_eq!(plan.multipath_schedule, refreshed);
    assert!(
        decision
            .explain
            .iter()
            .any(|line| line.contains("multipath_rebuild_debounced=true"))
    );
}
