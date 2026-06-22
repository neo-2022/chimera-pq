use crate::{
    MeshCarrierLaneBinding, MeshMultipathFlowAction, MeshMultipathFlowKey, MeshMultipathLaneRole,
    MeshMultipathMode, MeshRouteBindingId, plan_multipath_flow,
};

use super::{
    assert_active_weight_contract, assert_carrier_binding_contract, explain_has, record, request,
    runtime_with_peers,
};

fn flow_plan(payload: &str, flow: &str) -> crate::MeshMultipathFlowPlan {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 10, 95),
        record("node-b", "198.51.100.32:443", "eu", 12, 93),
        record("node-c", "198.51.100.33:443", "eu", 14, 91),
    ]);
    let plan = runtime
        .plan_path_from_dps_payload(&request(), payload)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    assert_active_weight_contract(&plan);
    if plan.multipath_schedule.route_binding_id.is_some() {
        assert_carrier_binding_contract(&plan);
    }
    let key = MeshMultipathFlowKey::from_opaque_flow_id(flow)
        .unwrap_or_else(|e| unreachable!("flow key should be accepted: {e}"));
    plan_multipath_flow(&plan.multipath_schedule, key)
}

#[test]
fn same_flow_maps_to_same_active_lane() {
    let payload = concat!(
        "mesh_allowed_regions=eu;",
        "mesh_max_peers=3;",
        "mesh_max_selected_per_region=3;",
        "mesh_multipath_mode=aggregate_buffered;",
        "mesh_multipath_demand=bulk;",
        "mesh_route_binding_id=7101"
    );

    let first = flow_plan(payload, "local-ingress#stable-flow");
    let second = flow_plan(payload, "local-ingress#stable-flow");

    assert_eq!(first.action, MeshMultipathFlowAction::Assigned);
    assert_eq!(first.selected_lane_id, second.selected_lane_id);
    assert_eq!(first.transit_payload_policy, "sealed_opaque_only");
    assert_eq!(first.fairness_policy, "weighted_round_robin_v1");
    assert!(first.route_binding_configured);
    assert!(
        first
            .explain
            .iter()
            .any(|line| line == "multipath_flow_privacy=sealed_opaque_only")
    );
}

#[test]
fn different_flows_spread_across_active_lanes() {
    let payload = concat!(
        "mesh_allowed_regions=eu;",
        "mesh_max_peers=3;",
        "mesh_max_selected_per_region=3;",
        "mesh_multipath_mode=aggregate_buffered;",
        "mesh_multipath_demand=bulk;",
        "mesh_route_binding_id=7102"
    );
    let mut lanes = std::collections::BTreeSet::new();

    for index in 0..64 {
        let plan = flow_plan(payload, &format!("opaque-flow-{index}"));
        assert_eq!(plan.action, MeshMultipathFlowAction::Assigned);
        lanes.insert(plan.selected_lane_id);
    }

    assert!(
        lanes.len() >= 2,
        "weighted stable assignment should use more than one active lane"
    );
}

#[test]
fn standby_lane_is_not_selected_for_flow_assignment() {
    let payload = concat!(
        "mesh_allowed_regions=eu;",
        "mesh_max_peers=2;",
        "mesh_max_selected_per_region=2;",
        "mesh_multipath_mode=standby_only;",
        "mesh_route_binding_id=7103"
    );

    for index in 0..16 {
        let plan = flow_plan(payload, &format!("standby-check-{index}"));
        assert_eq!(plan.action, MeshMultipathFlowAction::Assigned);
        assert_eq!(plan.selected_lane_id, Some(0));
        assert_eq!(plan.active_binding_count, 1);
    }
}

#[test]
fn missing_route_binding_fails_closed_without_lane_selection() {
    let payload = concat!(
        "mesh_allowed_regions=eu;",
        "mesh_max_peers=3;",
        "mesh_max_selected_per_region=3;",
        "mesh_multipath_mode=aggregate_buffered;",
        "mesh_multipath_demand=bulk"
    );

    let plan = flow_plan(payload, "unbound-flow");

    assert_eq!(plan.action, MeshMultipathFlowAction::FailClosed);
    assert_eq!(plan.reason, "route_binding_missing");
    assert_eq!(plan.selected_lane_id, None);
    assert!(!plan.route_binding_configured);
    assert_eq!(plan.active_binding_count, 0);
}

#[test]
fn multipath_schedule_prefers_high_reliability_and_low_load_weight() {
    let runtime = runtime_with_peers(vec![
        record("node-strong", "198.51.100.31:443", "eu", 10, 95),
        record("node-weaker", "198.51.100.32:443", "eu", 60, 75),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard;mesh_route_binding_id=7005",
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.selected_peers[0].node_id, "node-strong");
    assert_eq!(plan.selected_peers[1].node_id, "node-weaker");
    assert_eq!(plan.multipath_schedule.mode, MeshMultipathMode::FlowShard);
    assert_eq!(plan.multipath_schedule.lanes[0].peer_node_id, "node-strong");
    assert_eq!(plan.multipath_schedule.lanes[1].peer_node_id, "node-weaker");
    assert!(
        plan.multipath_schedule.lanes[0].weight_pct > plan.multipath_schedule.lanes[1].weight_pct
    );
    assert!(
        plan.multipath_schedule.lanes[0].capacity_weight_pct
            > plan.multipath_schedule.lanes[1].capacity_weight_pct
    );
    assert_active_weight_contract(&plan);
}

#[test]
fn planner_pressure_marks_rebuild_recommended() {
    let payload = concat!(
        "mesh_allowed_regions=eu;",
        "mesh_max_peers=2;",
        "mesh_max_selected_per_region=2;",
        "mesh_multipath_mode=flow_shard;",
        "mesh_multipath_demand=bulk;",
        "mesh_route_binding_id=7104"
    );

    let plan = flow_plan(payload, "pressure-flow");

    assert_eq!(plan.action, MeshMultipathFlowAction::Assigned);
    assert!(plan.rebuild_recommended);
    assert_eq!(plan.rebuild_reason, "demand_rebuild_recommended");
    assert!(
        plan.explain
            .iter()
            .any(|line| line == "multipath_flow_rebuild_recommended=true")
    );
}

#[test]
fn flow_explain_is_aggregate_and_redacted() {
    let payload = concat!(
        "mesh_allowed_regions=eu;",
        "mesh_max_peers=3;",
        "mesh_max_selected_per_region=3;",
        "mesh_multipath_mode=aggregate_buffered;",
        "mesh_multipath_demand=bulk;",
        "mesh_route_binding_id=7105"
    );

    let plan = flow_plan(payload, "SECRET_PAYLOAD_SENTINEL");
    let explain = plan.explain.join("|");
    let debug = format!("{plan:?}");

    assert!(explain.contains("multipath_flow_action=assigned"));
    assert!(explain.contains("multipath_flow_selected_lane=active"));
    assert!(explain.contains("multipath_flow_privacy=sealed_opaque_only"));
    assert!(!explain.contains("SECRET_PAYLOAD_SENTINEL"));
    assert!(!explain.contains("node-a"));
    assert!(!explain.contains("198.51.100.31"));
    assert!(!explain.contains("7105"));
    assert!(!debug.contains("SECRET_PAYLOAD_SENTINEL"));
    assert!(!debug.contains("node-a"));
    assert!(!debug.contains("198.51.100.31"));
    assert!(!debug.contains("7105"));
}

#[test]
fn malformed_flow_id_is_rejected_before_assignment() {
    assert!(MeshMultipathFlowKey::from_opaque_flow_id("").is_err());
    assert!(MeshMultipathFlowKey::from_opaque_flow_id("flow\nbad").is_err());
    assert!(MeshMultipathFlowKey::from_opaque_flow_id(&"x".repeat(257)).is_err());
}

#[test]
fn route_binding_mismatch_fails_closed() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 10, 95),
        record("node-b", "198.51.100.32:443", "eu", 12, 93),
    ]);
    let mut plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            concat!(
                "mesh_allowed_regions=eu;",
                "mesh_max_peers=2;",
                "mesh_max_selected_per_region=2;",
                "mesh_multipath_mode=flow_shard;",
                "mesh_route_binding_id=7107"
            ),
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    plan.multipath_schedule.carrier_lane_bindings[0].route_binding_id =
        MeshRouteBindingId::new(9007).unwrap_or_else(|e| unreachable!("{e}"));

    let key = MeshMultipathFlowKey::from_opaque_flow_id("binding-mismatch-flow")
        .unwrap_or_else(|e| unreachable!("flow key should be accepted: {e}"));
    let flow = plan_multipath_flow(&plan.multipath_schedule, key);

    assert_eq!(flow.action, MeshMultipathFlowAction::FailClosed);
    assert_eq!(flow.reason, "route_binding_mismatch");
    assert_eq!(flow.selected_lane_id, None);
}

#[test]
fn active_capacity_over_transit_budget_fails_closed() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 10, 95),
        record("node-b", "198.51.100.32:443", "eu", 12, 93),
    ]);
    let mut plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            concat!(
                "mesh_allowed_regions=eu;",
                "mesh_max_peers=2;",
                "mesh_max_selected_per_region=2;",
                "mesh_multipath_mode=flow_shard;",
                "mesh_route_binding_id=7108"
            ),
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    let route_binding_id = plan
        .multipath_schedule
        .route_binding_id
        .clone()
        .unwrap_or_else(|| unreachable!("route binding should be configured"));
    plan.multipath_schedule
        .carrier_lane_bindings
        .push(MeshCarrierLaneBinding {
            route_binding_id,
            lane_id: 99,
            peer_node_id: "node-over-budget".to_string(),
            carrier_endpoint: "198.51.100.99:443".to_string(),
            role: MeshMultipathLaneRole::Active,
            weight_pct: 50,
            capacity_weight_pct: 90,
        });

    let key = MeshMultipathFlowKey::from_opaque_flow_id("over-budget-flow")
        .unwrap_or_else(|e| unreachable!("flow key should be accepted: {e}"));
    let flow = plan_multipath_flow(&plan.multipath_schedule, key);

    assert_eq!(flow.action, MeshMultipathFlowAction::FailClosed);
    assert_eq!(flow.reason, "active_binding_capacity_over_budget");
    assert_eq!(flow.selected_lane_id, None);
    assert!(
        flow.total_capacity_weight_pct
            > u16::from(plan.multipath_schedule.transit_capacity_budget_pct)
    );
}

#[test]
fn existing_schedule_explain_still_reports_bound_contract() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 10, 95),
        record("node-b", "198.51.100.32:443", "eu", 12, 93),
    ]);
    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            concat!(
                "mesh_allowed_regions=eu;",
                "mesh_max_peers=2;",
                "mesh_max_selected_per_region=2;",
                "mesh_multipath_mode=flow_shard;",
                "mesh_route_binding_id=7106"
            ),
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_carrier_binding_contract=carrier_lane_binding_contract_ready"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_transit_payload_policy=sealed_opaque_only"
    ));
}
