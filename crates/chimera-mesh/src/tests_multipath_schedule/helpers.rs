use crate::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshMultipathLaneRole, MeshPathPlan, MeshRuntime,
};

pub(super) fn request() -> MeshJoinRequest {
    MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    }
}

pub(super) fn runtime_with_peers(records: Vec<MeshDiscoveryRecord>) -> MeshRuntime {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    runtime
        .merge_discovery("seed-b", &records)
        .unwrap_or_else(|e| unreachable!("discovery merge should succeed: {e}"));
    runtime
}

pub(super) fn record(
    node_id: &str,
    endpoint: &str,
    region: &str,
    load: u8,
    reliability: u8,
) -> MeshDiscoveryRecord {
    MeshDiscoveryRecord {
        node_id: node_id.to_string(),
        endpoint: endpoint.to_string(),
        region: region.to_string(),
        load_score: load,
        reliability_score: reliability,
    }
}

pub(super) fn explain_has(plan_explain: &[String], expected: &str) -> bool {
    plan_explain.iter().any(|line| line.contains(expected))
}

pub(super) fn assert_active_weight_contract(plan: &MeshPathPlan) {
    let active_weight_sum: u16 = plan
        .multipath_schedule
        .lanes
        .iter()
        .filter(|lane| lane.role == MeshMultipathLaneRole::Active)
        .map(|lane| lane.weight_pct as u16)
        .sum();
    assert_eq!(
        plan.multipath_schedule.active_weight_sum_pct,
        active_weight_sum
    );
    assert_eq!(active_weight_sum, 100);
    let active_capacity_sum: u16 = plan
        .multipath_schedule
        .lanes
        .iter()
        .filter(|lane| lane.role == MeshMultipathLaneRole::Active)
        .map(|lane| lane.capacity_weight_pct as u16)
        .sum();
    assert_eq!(
        plan.multipath_schedule.active_capacity_sum_pct,
        active_capacity_sum
    );
    assert_eq!(
        active_capacity_sum,
        plan.multipath_schedule.transit_capacity_budget_pct as u16
    );
    assert!(
        plan.multipath_schedule
            .lanes
            .iter()
            .filter(|lane| lane.role == MeshMultipathLaneRole::Active)
            .all(|lane| lane.weight_pct > 0)
    );
    assert!(
        plan.multipath_schedule
            .lanes
            .iter()
            .filter(|lane| lane.role == MeshMultipathLaneRole::Active)
            .all(|lane| lane.capacity_weight_pct > 0)
    );
    assert!(
        plan.multipath_schedule
            .lanes
            .iter()
            .filter(|lane| lane.role == MeshMultipathLaneRole::Standby)
            .all(|lane| lane.capacity_weight_pct == 0)
    );
    assert!(plan.multipath_schedule.local_traffic_reserve_pct > 0);
    assert_eq!(
        plan.multipath_schedule.local_traffic_reserve_pct
            + plan.multipath_schedule.transit_capacity_budget_pct,
        100
    );
    assert!(
        plan.multipath_schedule.transit_capacity_budget_pct
            < plan.multipath_schedule.active_weight_sum_pct as u8
    );
    assert!(
        plan.multipath_schedule.local_traffic_reserve_pct >= 10,
        "local traffic reserve must not be silently weakened"
    );
    assert!(
        plan.multipath_schedule.transit_capacity_budget_pct <= 90,
        "transit budget must not silently consume local reserve"
    );
}

pub(super) fn assert_carrier_binding_contract(plan: &MeshPathPlan) {
    let route_binding_id = plan
        .multipath_schedule
        .route_binding_id
        .as_ref()
        .unwrap_or_else(|| unreachable!("route binding id should be configured"));
    assert!(route_binding_id.get() > 0);
    assert_eq!(
        plan.multipath_schedule.carrier_lane_bindings.len(),
        plan.multipath_schedule.lanes.len()
    );
    let mut route_lane_pairs = std::collections::BTreeSet::new();
    for binding in &plan.multipath_schedule.carrier_lane_bindings {
        assert_eq!(&binding.route_binding_id, route_binding_id);
        assert!(route_lane_pairs.insert((binding.route_binding_id.get(), binding.lane_id)));
    }
}

pub(super) fn assert_binding_matches_lane(
    plan: &MeshPathPlan,
    index: usize,
    peer_id: &str,
    endpoint: &str,
) {
    let lane = &plan.multipath_schedule.lanes[index];
    let binding = &plan.multipath_schedule.carrier_lane_bindings[index];
    assert_eq!(
        Some(&binding.route_binding_id),
        plan.multipath_schedule.route_binding_id.as_ref()
    );
    assert_eq!(binding.lane_id, lane.lane_id);
    assert_eq!(binding.peer_node_id, peer_id);
    assert_eq!(binding.carrier_endpoint, endpoint);
    assert_eq!(binding.role, lane.role);
    assert_eq!(binding.weight_pct, lane.weight_pct);
    assert_eq!(binding.capacity_weight_pct, lane.capacity_weight_pct);
}
