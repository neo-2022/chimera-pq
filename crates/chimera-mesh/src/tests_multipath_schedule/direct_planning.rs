use super::helpers::{
    assert_active_weight_contract, explain_has, record, request, runtime_with_peers,
};
use crate::{MeshMultipathMode, MeshPathPolicy, MultipathMode};

#[test]
fn direct_plan_path_uses_policy_multipath_mode_without_carrier_binding() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
    ]);
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 0,
        max_load_score: 100,
        max_peers: 2,
        prefer_region_diversity: true,
        max_selected_per_region: 2,
        min_distinct_regions: 1,
        path_profile_override: None,
        multipath_mode: Some(MultipathMode::FlowShard),
        connect_fallback_ports: vec![443, 8443],
    };

    let plan = runtime
        .plan_path(&request(), &policy)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.selected_peers.len(), 2);
    assert_eq!(plan.multipath_schedule.mode, MeshMultipathMode::FlowShard);
    assert_eq!(plan.multipath_schedule.active_lane_count, 2);
    assert_eq!(
        plan.multipath_schedule
            .lane_admission_requested_active_lane_count,
        2
    );
    assert_eq!(
        plan.multipath_schedule
            .lane_admission_admitted_active_lane_count,
        2
    );
    assert_eq!(
        plan.multipath_schedule
            .lane_admission_rejected_active_lane_count,
        0
    );
    assert_eq!(
        plan.multipath_schedule.lane_admission_capacity_status,
        "within_budget"
    );
    assert_eq!(plan.multipath_schedule.standby_lane_count, 0);
    assert_eq!(plan.multipath_schedule.carrier_lane_bindings.len(), 0);
    assert!(plan.multipath_schedule.route_binding_id.is_none());
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_execution_status=planner_only_not_carrier_bound"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_planner_rebuild_reason=initial_plan"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_lane_admission_capacity_status=within_budget"
    ));
    assert_active_weight_contract(&plan);
}

#[test]
fn policy_multipath_mode_does_not_disable_auto_health_filtering() {
    let mut runtime = runtime_with_peers(vec![
        record("node-cooling", "198.51.100.33:443", "eu", 10, 95),
        record("node-a", "198.51.100.34:443", "eu", 20, 94),
        record("node-b", "198.51.100.35:443", "eu", 22, 93),
    ]);
    runtime
        .update_health_state(&[crate::MeshPeerHealth {
            node_id: "node-cooling".to_string(),
            healthy: true,
            cooldown_active: true,
        }])
        .unwrap_or_else(|e| unreachable!("health update should succeed: {e}"));
    let mut policy = MeshPathPolicy::default_auto();
    policy.multipath_mode = Some(MultipathMode::FlowShard);

    let plan = runtime
        .plan_path(&request(), &policy)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.selected_peers.len(), 2);
    assert_eq!(plan.multipath_schedule.mode, MeshMultipathMode::FlowShard);
    assert!(
        plan.multipath_schedule
            .lanes
            .iter()
            .all(|lane| lane.peer_node_id != "node-cooling")
    );
    assert!(explain_has(&plan.explain, "decision_control_mode=auto"));
    assert!(explain_has(&plan.explain, "manual_override_fields=none"));
    assert!(explain_has(
        &plan.explain,
        "effective_health_filter_source=auto"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_execution_status=planner_only_not_carrier_bound"
    ));
}
