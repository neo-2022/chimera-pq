use super::helpers::{
    assert_active_weight_contract, assert_carrier_binding_contract, explain_has, record, request,
    runtime_with_peers,
};
use crate::MeshMultipathLaneRole;

#[test]
fn flow_shard_capacity_budget_is_distributed_across_active_lanes() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard;mesh_route_binding_id=7010",
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.multipath_schedule.local_traffic_reserve_pct, 10);
    assert_eq!(plan.multipath_schedule.transit_capacity_budget_pct, 90);
    assert_eq!(
        plan.multipath_schedule.local_traffic_reserve_pct
            + plan.multipath_schedule.transit_capacity_budget_pct,
        100
    );
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
    assert_eq!(plan.multipath_schedule.active_weight_sum_pct, 100);
    assert_eq!(plan.multipath_schedule.active_capacity_sum_pct, 90);
    assert_active_weight_contract(&plan);
    assert_carrier_binding_contract(&plan);
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_active_capacity_sum_pct=90"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_lane_admission_requested_active_lanes=2"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_lane_admission_admitted_active_lanes=2"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_lane_admission_rejected_active_lanes=0"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_lane_admission_capacity_status=within_budget"
    ));
}

#[test]
fn standby_lane_has_zero_transit_capacity() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            concat!(
                "mesh_allowed_regions=eu;",
                "mesh_max_peers=2;",
                "mesh_max_selected_per_region=2;",
                "mesh_multipath_mode=standby_only;",
                "mesh_route_binding_id=7011"
            ),
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    let standby = plan
        .multipath_schedule
        .lanes
        .iter()
        .find(|lane| lane.role == MeshMultipathLaneRole::Standby)
        .unwrap_or_else(|| unreachable!("standby lane should exist"));

    assert_eq!(plan.multipath_schedule.active_capacity_sum_pct, 90);
    assert_eq!(standby.weight_pct, 0);
    assert_eq!(standby.capacity_weight_pct, 0);
    assert_eq!(
        plan.multipath_schedule
            .carrier_lane_bindings
            .iter()
            .find(|binding| binding.lane_id == standby.lane_id)
            .unwrap_or_else(|| unreachable!("standby binding should exist"))
            .capacity_weight_pct,
        0
    );
    assert_active_weight_contract(&plan);
    assert_carrier_binding_contract(&plan);
}

#[test]
fn degraded_peer_receives_lower_weight_and_capacity_after_replan() {
    let runtime = runtime_with_peers(vec![
        record("node-strong", "198.51.100.31:443", "eu", 10, 95),
        record("node-degraded", "198.51.100.32:443", "eu", 80, 70),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            concat!(
                "mesh_allowed_regions=eu;",
                "mesh_max_peers=2;",
                "mesh_max_selected_per_region=2;",
                "mesh_multipath_mode=flow_shard;",
                "mesh_route_binding_id=7012"
            ),
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    let strong = plan
        .multipath_schedule
        .lanes
        .iter()
        .find(|lane| lane.peer_node_id == "node-strong")
        .unwrap_or_else(|| unreachable!("strong lane should exist"));
    let degraded = plan
        .multipath_schedule
        .lanes
        .iter()
        .find(|lane| lane.peer_node_id == "node-degraded")
        .unwrap_or_else(|| unreachable!("degraded lane should exist"));

    assert!(strong.weight_pct > degraded.weight_pct);
    assert!(strong.capacity_weight_pct > degraded.capacity_weight_pct);
    assert_eq!(plan.multipath_schedule.active_capacity_sum_pct, 90);
    assert_active_weight_contract(&plan);
}

#[test]
fn aggregate_buffered_capacity_uses_all_selected_active_lanes() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
        record("node-c", "198.51.100.33:443", "eu", 24, 92),
        record("node-d", "198.51.100.34:443", "eu", 26, 93),
        record("node-e", "198.51.100.35:443", "eu", 28, 94),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            concat!(
                "mesh_allowed_regions=eu;",
                "mesh_max_peers=5;",
                "mesh_max_selected_per_region=5;",
                "mesh_multipath_mode=aggregate_buffered;",
                "mesh_route_binding_id=7013"
            ),
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.multipath_schedule.active_lane_count, 5);
    assert_eq!(
        plan.multipath_schedule
            .lane_admission_requested_active_lane_count,
        5
    );
    assert_eq!(
        plan.multipath_schedule
            .lane_admission_admitted_active_lane_count,
        5
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
    assert_eq!(plan.multipath_schedule.active_capacity_sum_pct, 90);
    assert!(
        plan.multipath_schedule
            .lanes
            .iter()
            .filter(|lane| lane.role == MeshMultipathLaneRole::Active)
            .all(|lane| lane.capacity_weight_pct > 0)
    );
    assert_active_weight_contract(&plan);
    assert_carrier_binding_contract(&plan);
}

#[test]
fn aggregate_buffered_active_lanes_have_nonzero_capacity_within_transit_budget() {
    let records = (0..95)
        .map(|idx| {
            let node = format!("node-{idx}");
            let endpoint = format!("198.51.100.{}:443", idx + 1);
            let region = format!("test-region-{idx}");
            record(&node, &endpoint, &region, 20, 90)
        })
        .collect();
    let runtime = runtime_with_peers(records);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            concat!(
                "mesh_max_peers=95;",
                "mesh_max_selected_per_region=95;",
                "mesh_multipath_mode=aggregate_buffered;",
                "mesh_route_binding_id=7015"
            ),
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(
        plan.multipath_schedule
            .lane_admission_requested_active_lane_count,
        95
    );
    assert_eq!(
        plan.multipath_schedule
            .lane_admission_admitted_active_lane_count,
        plan.multipath_schedule.active_lane_count
    );
    assert_eq!(
        plan.multipath_schedule
            .lane_admission_rejected_active_lane_count,
        5
    );
    assert_eq!(
        plan.multipath_schedule.lane_admission_capacity_status,
        "over_budget_truncated"
    );
    assert_eq!(
        plan.multipath_schedule.active_lane_count,
        plan.multipath_schedule
            .lane_admission_admitted_active_lane_count
    );
    assert!(
        plan.multipath_schedule.active_capacity_sum_pct
            <= plan.multipath_schedule.transit_capacity_budget_pct as u16
    );
    assert_eq!(plan.multipath_schedule.active_capacity_sum_pct, 90);
    assert!(
        plan.multipath_schedule
            .lanes
            .iter()
            .all(|lane| lane.capacity_weight_pct > 0)
    );
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_lane_admission_requested_active_lanes=95"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_lane_admission_admitted_active_lanes=90"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_lane_admission_rejected_active_lanes=5"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_lane_admission_capacity_status=over_budget_truncated"
    ));
    assert_active_weight_contract(&plan);
}

#[test]
fn capacity_budget_explain_and_debug_redact_sensitive_material() {
    let runtime = runtime_with_peers(vec![
        record("node-sensitive-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-sensitive-b", "198.51.100.32:9443", "eu", 22, 91),
    ]);
    let payload = concat!(
        "mesh_allowed_regions=eu;",
        "mesh_multipath_mode=flow_shard;",
        "mesh_route_binding_id=7014;",
        "non_mesh_note=SECRET_PAYLOAD_MARKER"
    );

    let plan = runtime
        .plan_path_from_dps_payload(&request(), payload)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    let explain = plan.explain.join("\n");
    let lanes_debug = format!("{:?}", plan.multipath_schedule.lanes);
    let bindings_debug = format!("{:?}", plan.multipath_schedule.carrier_lane_bindings);
    let combined = format!("{explain}\n{lanes_debug}\n{bindings_debug}");

    assert!(explain.contains("multipath_schedule_active_capacity_sum_pct=90"));
    assert!(explain.contains("multipath_schedule_local_reserve_pct=10"));
    assert!(explain.contains("multipath_schedule_transit_capacity_budget_pct=90"));
    assert!(explain.contains("multipath_schedule_lane_admission_requested_active_lanes=2"));
    assert!(explain.contains("multipath_schedule_lane_admission_admitted_active_lanes=2"));
    assert!(explain.contains("multipath_schedule_lane_admission_rejected_active_lanes=0"));
    assert!(explain.contains("multipath_schedule_lane_admission_capacity_status=within_budget"));
    assert!(explain.contains("multipath_schedule_transit_payload_policy=sealed_opaque_only"));
    assert!(explain.contains("multipath_schedule_planner_rebuild_reason=multipath_hint_replan"));
    assert!(!combined.contains("node-sensitive-a"));
    assert!(!combined.contains("node-sensitive-b"));
    assert!(!combined.contains("198.51.100.31"));
    assert!(!combined.contains("198.51.100.32"));
    assert!(!combined.contains("7014"));
    assert!(!combined.contains("SECRET_PAYLOAD_MARKER"));
}
