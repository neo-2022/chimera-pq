use crate::{
    MeshMultipathRebuildAction, MeshPeerPerformance, MeshRouteBindingId, MultipathDemand,
    MultipathMode,
};

use super::{explain_has, policy, record, request, runtime_with_peers};

fn runtime_for_pending() -> crate::MeshRuntime {
    runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
        record("node-c", "198.51.100.33:443", "eu", 24, 92),
        record("node-d", "198.51.100.34:443", "eu", 26, 93),
    ])
}

fn path_policy() -> crate::MeshPathPolicy {
    let mut path_policy = crate::MeshPathPolicy::default_auto();
    path_policy.allowed_regions = vec!["eu".to_string()];
    path_policy.max_peers = 4;
    path_policy.max_selected_per_region = 4;
    path_policy.multipath_mode = Some(MultipathMode::FlowShard);
    path_policy.multipath_demand = Some(MultipathDemand::Normal);
    path_policy
}

fn mark_pending_rebuild(runtime: &mut crate::MeshRuntime) {
    let _ = runtime.take_pending_multipath_rebuild_signal();
    runtime
        .update_peer_performance(&[
            MeshPeerPerformance {
                node_id: "node-a".to_string(),
                latency_ms: Some(250),
                throughput_mbps: Some(40),
            },
            MeshPeerPerformance {
                node_id: "node-b".to_string(),
                latency_ms: Some(20),
                throughput_mbps: Some(900),
            },
        ])
        .unwrap_or_else(|e| unreachable!("performance update should succeed: {e}"));
    assert!(runtime.pending_multipath_rebuild_signal().is_some());
}

#[test]
fn pending_full_and_core_plans_match_and_clear_signal() {
    let mut full_runtime = runtime_for_pending();
    mark_pending_rebuild(&mut full_runtime);
    let mut core_runtime = full_runtime.clone();
    let path_policy = path_policy();

    let (full_plan, full_decision) = full_runtime
        .plan_path_with_pending_multipath_rebuild(&request(), &path_policy, &policy())
        .unwrap_or_else(|e| unreachable!("full pending rebuild should apply: {e}"));
    let (core_plan, core_decision) = core_runtime
        .plan_path_core_with_pending_multipath_rebuild(&request(), &path_policy, &policy())
        .unwrap_or_else(|e| unreachable!("core pending rebuild should apply: {e}"));
    let full_decision =
        full_decision.unwrap_or_else(|| unreachable!("full pending decision should be present"));
    let core_decision =
        core_decision.unwrap_or_else(|| unreachable!("core pending decision should be present"));

    assert_eq!(
        full_decision.action,
        MeshMultipathRebuildAction::AllowRebuild
    );
    assert_eq!(core_decision.action, full_decision.action);
    assert_eq!(core_decision.reason, full_decision.reason);
    assert_eq!(core_plan.selected_peers, full_plan.selected_peers);
    assert_eq!(core_plan.multipath_schedule, full_plan.multipath_schedule);
    assert!(full_runtime.pending_multipath_rebuild_signal().is_none());
    assert!(core_runtime.pending_multipath_rebuild_signal().is_none());
    assert!(explain_has(
        &full_plan.explain,
        "multipath_rebuild_signal_reason=peer_performance_changed"
    ));
    assert!(
        !full_plan
            .explain
            .iter()
            .any(|line| line.contains("198.51."))
    );
    assert!(
        core_decision
            .explain()
            .iter()
            .all(|line| !line.contains("198.51."))
    );
}

#[test]
fn pending_existing_plan_full_and_core_preserve_route_binding_and_clear_signal() {
    let mut full_runtime = runtime_for_pending();
    let mut core_runtime = runtime_for_pending();
    let path_policy = path_policy();
    let route_binding_id = MeshRouteBindingId::new(0x7001)
        .unwrap_or_else(|e| unreachable!("route binding id should parse: {e}"));
    let mut full_plan = full_runtime
        .plan_path(&request(), &path_policy)
        .unwrap_or_else(|e| unreachable!("full plan should build: {e}"));
    let mut core_plan = core_runtime
        .plan_path_core(&request(), &path_policy)
        .unwrap_or_else(|e| unreachable!("core plan should build: {e}"));
    full_plan.multipath_schedule.route_binding_id = Some(route_binding_id);
    core_plan.multipath_schedule.route_binding_id = Some(route_binding_id);
    mark_pending_rebuild(&mut full_runtime);
    mark_pending_rebuild(&mut core_runtime);

    let full_decision = full_runtime
        .apply_pending_multipath_rebuild_with_policy_to_plan(
            &request(),
            &path_policy,
            &mut full_plan,
            &policy(),
        )
        .unwrap_or_else(|e| unreachable!("full existing pending rebuild should apply: {e}"))
        .unwrap_or_else(|| unreachable!("full pending decision should be present"));
    let core_decision = core_runtime
        .apply_pending_multipath_rebuild_with_policy_to_plan_core(
            &request(),
            &path_policy,
            &mut core_plan,
            &policy(),
        )
        .unwrap_or_else(|e| unreachable!("core existing pending rebuild should apply: {e}"))
        .unwrap_or_else(|| unreachable!("core pending decision should be present"));

    assert_eq!(
        full_decision.action,
        MeshMultipathRebuildAction::AllowRebuild
    );
    assert_eq!(core_decision.action, full_decision.action);
    assert_eq!(core_decision.reason, full_decision.reason);
    assert_eq!(core_plan.selected_peers, full_plan.selected_peers);
    assert_eq!(core_plan.multipath_schedule, full_plan.multipath_schedule);
    assert_eq!(
        full_plan.multipath_schedule.route_binding_id,
        Some(route_binding_id)
    );
    assert_eq!(
        core_plan.multipath_schedule.route_binding_id,
        Some(route_binding_id)
    );
    assert!(full_runtime.pending_multipath_rebuild_signal().is_none());
    assert!(core_runtime.pending_multipath_rebuild_signal().is_none());
}

#[test]
fn core_stale_pending_rebuild_fails_closed_and_clears_signal() {
    let mut runtime = runtime_for_pending();
    mark_pending_rebuild(&mut runtime);
    for source in ["seed-c", "seed-d", "seed-e", "seed-f", "seed-g"] {
        runtime
            .merge_discovery(source, &[])
            .unwrap_or_else(|e| unreachable!("empty discovery tick should succeed: {e}"));
    }
    let path_policy = path_policy();

    let error = match runtime.plan_path_core_with_pending_multipath_rebuild(
        &request(),
        &path_policy,
        &policy(),
    ) {
        Ok(_) => unreachable!("core stale pending rebuild must fail closed"),
        Err(error) => error,
    };

    assert!(error.contains("failed closed"));
    assert!(error.contains("stale_telemetry"));
    assert!(runtime.pending_multipath_rebuild_signal().is_none());
}

#[test]
fn full_rebuild_explain_replaces_previous_rebuild_lines() {
    let mut runtime = runtime_for_pending();
    let path_policy = path_policy();
    let mut plan = runtime
        .plan_path(&request(), &path_policy)
        .unwrap_or_else(|e| unreachable!("plan should build: {e}"));
    mark_pending_rebuild(&mut runtime);
    let duplicate = runtime
        .pending_multipath_rebuild_signal()
        .cloned()
        .unwrap_or_else(|| unreachable!("pending signal should be present"));
    runtime
        .apply_pending_multipath_rebuild_with_policy_to_plan(
            &request(),
            &path_policy,
            &mut plan,
            &policy(),
        )
        .unwrap_or_else(|e| unreachable!("first pending rebuild should apply: {e}"))
        .unwrap_or_else(|| unreachable!("first pending decision should be present"));
    runtime
        .apply_multipath_rebuild_to_plan(&mut plan, &duplicate, &policy())
        .unwrap_or_else(|e| unreachable!("duplicate rebuild should evaluate: {e}"));

    let action_lines: Vec<&str> = plan
        .explain
        .iter()
        .filter(|line| line.starts_with("multipath_rebuild_action="))
        .map(String::as_str)
        .collect();
    assert_eq!(
        action_lines,
        vec!["multipath_rebuild_action=suppress_rebuild"]
    );
    assert!(explain_has(
        &plan.explain,
        "multipath_rebuild_debounced=true"
    ));
}
