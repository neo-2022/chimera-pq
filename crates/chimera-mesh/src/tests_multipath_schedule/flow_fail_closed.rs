use crate::{
    MeshCarrierLaneBinding, MeshMultipathFlowAction, MeshMultipathFlowKey, MeshMultipathLaneRole,
    plan_multipath_flow,
};

use super::{
    assert_active_weight_contract, assert_carrier_binding_contract, record, request,
    runtime_with_peers,
};

fn bound_schedule() -> crate::MeshMultipathSchedule {
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
                "mesh_route_binding_id=7201"
            ),
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    assert_active_weight_contract(&plan);
    assert_carrier_binding_contract(&plan);
    plan.multipath_schedule
}

fn assert_fail_reason(schedule: &crate::MeshMultipathSchedule, expected_reason: &str) {
    let key = MeshMultipathFlowKey::from_opaque_flow_id("negative-flow")
        .unwrap_or_else(|e| unreachable!("flow key should be accepted: {e}"));
    let plan = plan_multipath_flow(schedule, key);

    assert_eq!(plan.action, MeshMultipathFlowAction::FailClosed);
    assert_eq!(plan.reason, expected_reason);
    assert_eq!(plan.selected_lane_id, None);
    assert!(
        plan.explain
            .iter()
            .any(|line| line == "multipath_flow_selected_lane=none")
    );
}

#[test]
fn non_opaque_transit_policy_fails_closed() {
    let mut schedule = bound_schedule();
    schedule.transit_payload_policy = "plaintext_forbidden".to_string();

    assert_fail_reason(&schedule, "transit_payload_policy_not_opaque");
}

#[test]
fn invalid_local_reserve_fails_closed() {
    let mut schedule = bound_schedule();
    schedule.local_traffic_reserve_pct = 0;

    assert_fail_reason(&schedule, "local_reserve_invalid");
}

#[test]
fn active_binding_missing_fails_closed() {
    let mut schedule = bound_schedule();
    schedule.carrier_lane_bindings.clear();

    assert_fail_reason(&schedule, "active_binding_missing");
}

#[test]
fn duplicate_active_lane_fails_closed() {
    let mut schedule = bound_schedule();
    let duplicate = schedule.carrier_lane_bindings[0].lane_id;
    schedule.carrier_lane_bindings[1].lane_id = duplicate;

    assert_fail_reason(&schedule, "duplicate_active_lane");
}

#[test]
fn active_binding_capacity_missing_fails_closed() {
    let mut schedule = bound_schedule();
    for binding in &mut schedule.carrier_lane_bindings {
        binding.capacity_weight_pct = 0;
    }

    assert_fail_reason(&schedule, "active_binding_capacity_missing");
}

#[test]
fn active_binding_capacity_overflow_fails_closed() {
    let mut schedule = bound_schedule();
    let route_binding_id = schedule
        .route_binding_id
        .clone()
        .unwrap_or_else(|| unreachable!("route binding should be configured"));
    for lane_id in 2..300 {
        schedule.carrier_lane_bindings.push(MeshCarrierLaneBinding {
            route_binding_id: route_binding_id.clone(),
            lane_id,
            peer_node_id: format!("node-overflow-{lane_id}"),
            carrier_endpoint: format!("198.51.100.{lane_id}:443"),
            role: MeshMultipathLaneRole::Active,
            weight_pct: 1,
            capacity_weight_pct: 255,
        });
    }

    assert_fail_reason(&schedule, "capacity_overflow");
}
