use crate::{MeshMultipathFlowAction, MeshMultipathFlowKey, plan_multipath_flow};

use super::{
    assert_active_weight_contract, assert_carrier_binding_contract, record, request,
    runtime_with_peers,
};

fn rebuild_schedule() -> crate::MeshMultipathSchedule {
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
                "mesh_route_binding_id=7301"
            ),
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    assert_active_weight_contract(&plan);
    assert_carrier_binding_contract(&plan);
    plan.multipath_schedule.demand_rebuild_recommended = false;
    plan.multipath_schedule.demand_unmet_lane_count = 0;
    plan.multipath_schedule
        .lane_admission_rejected_active_lane_count = 0;
    plan.multipath_schedule.demand_planned_active_lane_count =
        plan.multipath_schedule.active_lane_count;
    plan.multipath_schedule
        .lane_admission_admitted_active_lane_count = plan.multipath_schedule.active_lane_count;
    plan.multipath_schedule
}

fn flow_plan(schedule: &crate::MeshMultipathSchedule, flow: &str) -> crate::MeshMultipathFlowPlan {
    let key = MeshMultipathFlowKey::from_opaque_flow_id(flow)
        .unwrap_or_else(|e| unreachable!("flow key should be accepted: {e}"));
    plan_multipath_flow(schedule, key)
}

#[test]
fn active_lanes_below_plan_sets_rebuild_reason() {
    let mut schedule = rebuild_schedule();
    schedule.demand_planned_active_lane_count = schedule.active_lane_count + 1;

    let plan = flow_plan(&schedule, "rebuild-active-lanes-below-plan");

    assert_eq!(plan.action, MeshMultipathFlowAction::Assigned);
    assert!(plan.rebuild_recommended);
    assert_eq!(plan.rebuild_reason, "active_lanes_below_plan");
    assert!(
        plan.explain
            .iter()
            .any(|line| line == "multipath_flow_rebuild_reason=active_lanes_below_plan")
    );
}

#[test]
fn capacity_pressure_sets_rebuild_reason() {
    let mut schedule = rebuild_schedule();
    schedule.demand_unmet_lane_count = 1;

    let plan = flow_plan(&schedule, "rebuild-capacity-pressure");

    assert_eq!(plan.action, MeshMultipathFlowAction::Assigned);
    assert!(plan.rebuild_recommended);
    assert_eq!(plan.rebuild_reason, "capacity_pressure");
    assert!(
        plan.explain
            .iter()
            .any(|line| line == "multipath_flow_rebuild_reason=capacity_pressure")
    );
}
