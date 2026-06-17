use super::helpers::{
    assert_active_weight_contract, explain_has, record, request, runtime_with_peers,
};
use crate::{MeshPathPolicy, MultipathDemand, MultipathMode};

#[test]
fn aggregate_low_demand_uses_single_active_lane_without_weakening_local_reserve() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
        record("node-c", "198.51.100.33:443", "eu", 24, 92),
        record("node-d", "198.51.100.34:443", "eu", 26, 93),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            concat!(
                "mesh_allowed_regions=eu;",
                "mesh_max_peers=4;",
                "mesh_max_selected_per_region=4;",
                "mesh_multipath_mode=aggregate_buffered;",
                "mesh_multipath_demand=low;",
                "mesh_route_binding_id=7020"
            ),
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.selected_peers.len(), 4);
    assert_eq!(plan.multipath_schedule.active_lane_count, 1);
    assert_eq!(plan.multipath_schedule.demand_policy, "low");
    assert_eq!(
        plan.multipath_schedule.demand_policy_source,
        "control_policy"
    );
    assert_eq!(
        plan.multipath_schedule.demand_requested_active_lane_count,
        1
    );
    assert_eq!(plan.multipath_schedule.demand_planned_active_lane_count, 1);
    assert_eq!(
        plan.multipath_schedule.demand_admitted_lane_capacity_pct,
        90
    );
    assert_eq!(plan.multipath_schedule.demand_unmet_lane_count, 0);
    assert_eq!(plan.multipath_schedule.demand_status, "within_budget");
    assert_eq!(
        plan.multipath_schedule.demand_planned_active_lane_count
            + plan.multipath_schedule.demand_unmet_lane_count,
        plan.multipath_schedule.demand_requested_active_lane_count
    );
    assert_eq!(
        plan.multipath_schedule
            .lane_admission_admitted_active_lane_count
            + plan
                .multipath_schedule
                .lane_admission_rejected_active_lane_count,
        plan.multipath_schedule
            .lane_admission_requested_active_lane_count
    );
    assert!(plan.multipath_schedule.demand_rebuild_recommended);
    assert_eq!(plan.multipath_schedule.local_traffic_reserve_pct, 10);
    assert_eq!(plan.multipath_schedule.transit_capacity_budget_pct, 90);
    assert_eq!(plan.multipath_schedule.active_capacity_sum_pct, 90);
    assert_active_weight_contract(&plan);
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_demand_policy=low"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_demand_status=within_budget"
    ));
}

#[test]
fn aggregate_bulk_demand_reports_budget_saturation_without_exceeding_transit_budget() {
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
                "mesh_multipath_demand=bulk;",
                "mesh_route_binding_id=7021"
            ),
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.multipath_schedule.active_lane_count, 90);
    assert_eq!(plan.multipath_schedule.demand_policy, "bulk");
    assert_eq!(
        plan.multipath_schedule.demand_policy_source,
        "control_policy"
    );
    assert_eq!(
        plan.multipath_schedule.demand_requested_active_lane_count,
        95
    );
    assert_eq!(plan.multipath_schedule.demand_planned_active_lane_count, 90);
    assert_eq!(plan.multipath_schedule.demand_unmet_lane_count, 5);
    assert_eq!(plan.multipath_schedule.demand_status, "budget_saturated");
    assert_eq!(
        plan.multipath_schedule.demand_planned_active_lane_count
            + plan.multipath_schedule.demand_unmet_lane_count,
        plan.multipath_schedule.demand_requested_active_lane_count
    );
    assert_eq!(
        plan.multipath_schedule
            .lane_admission_admitted_active_lane_count
            + plan
                .multipath_schedule
                .lane_admission_rejected_active_lane_count,
        plan.multipath_schedule
            .lane_admission_requested_active_lane_count
    );
    assert!(
        plan.multipath_schedule.demand_admitted_lane_capacity_pct
            <= 100 - plan.multipath_schedule.local_traffic_reserve_pct
    );
    assert_eq!(plan.multipath_schedule.active_capacity_sum_pct, 90);
    assert!(
        plan.multipath_schedule.active_lane_count
            <= plan.multipath_schedule.transit_capacity_budget_pct as usize
    );
    assert_active_weight_contract(&plan);
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_demand_requested_active_lanes=95"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_demand_planned_active_lanes=90"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_demand_status=budget_saturated"
    ));
}

#[test]
fn flow_shard_normal_demand_reduces_to_one_active_lane() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            concat!(
                "mesh_allowed_regions=eu;",
                "mesh_multipath_mode=flow_shard;",
                "mesh_multipath_demand=normal;",
                "mesh_route_binding_id=7022"
            ),
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.multipath_schedule.active_lane_count, 1);
    assert_eq!(plan.multipath_schedule.demand_policy, "normal");
    assert_eq!(
        plan.multipath_schedule.demand_requested_active_lane_count,
        1
    );
    assert_eq!(
        plan.multipath_schedule
            .lane_admission_requested_active_lane_count,
        1
    );
    assert_eq!(plan.multipath_schedule.active_capacity_sum_pct, 90);
    assert_active_weight_contract(&plan);
}

#[test]
fn demand_route_explain_fields_match_scheduler_state() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
        record("node-c", "198.51.100.33:443", "eu", 24, 92),
        record("node-d", "198.51.100.34:443", "eu", 26, 93),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            concat!(
                "mesh_allowed_regions=eu;",
                "mesh_max_peers=4;",
                "mesh_max_selected_per_region=4;",
                "mesh_multipath_mode=aggregate_buffered;",
                "mesh_multipath_demand=high;",
                "mesh_route_binding_id=7024"
            ),
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    let schedule = &plan.multipath_schedule;

    assert!(explain_has(
        &plan.explain,
        &format!(
            "multipath_schedule_demand_requested_active_lanes={}",
            schedule.demand_requested_active_lane_count
        )
    ));
    assert!(explain_has(
        &plan.explain,
        &format!(
            "multipath_schedule_demand_planned_active_lanes={}",
            schedule.demand_planned_active_lane_count
        )
    ));
    assert!(explain_has(
        &plan.explain,
        &format!(
            "multipath_schedule_demand_admitted_lane_capacity_pct={}",
            schedule.demand_admitted_lane_capacity_pct
        )
    ));
    assert!(explain_has(
        &plan.explain,
        &format!(
            "multipath_schedule_demand_status={}",
            schedule.demand_status
        )
    ));
    assert!(explain_has(
        &plan.explain,
        &format!(
            "multipath_schedule_active_lanes={}",
            schedule.active_lane_count
        )
    ));
}

#[test]
fn direct_policy_demand_hint_rebuilds_source_level_schedule() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
        record("node-c", "198.51.100.33:443", "eu", 24, 92),
        record("node-d", "198.51.100.34:443", "eu", 26, 93),
    ]);
    let mut policy = MeshPathPolicy::default_auto();
    policy.allowed_regions = vec!["eu".to_string()];
    policy.max_peers = 4;
    policy.max_selected_per_region = 4;
    policy.multipath_mode = Some(MultipathMode::AggregateBuffered);
    policy.multipath_demand = Some(MultipathDemand::High);

    let plan = runtime
        .plan_path(&request(), &policy)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.multipath_schedule.active_lane_count, 4);
    assert_eq!(plan.multipath_schedule.demand_policy, "high");
    assert_eq!(
        plan.multipath_schedule.demand_policy_source,
        "control_policy"
    );
    assert_eq!(
        plan.multipath_schedule.demand_requested_active_lane_count,
        4
    );
    assert_eq!(plan.multipath_schedule.demand_planned_active_lane_count, 4);
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_demand_policy=high"
    ));
    assert_active_weight_contract(&plan);
}

#[test]
fn demand_explain_and_debug_do_not_leak_payload_endpoint_or_route_binding_id() {
    let runtime = runtime_with_peers(vec![
        record("node-sensitive-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-sensitive-b", "198.51.100.32:9443", "eu", 22, 91),
    ]);
    let payload = concat!(
        "mesh_allowed_regions=eu;",
        "mesh_multipath_mode=flow_shard;",
        "mesh_multipath_demand=high;",
        "mesh_route_binding_id=7023;",
        "non_mesh_note=SECRET_PAYLOAD_MARKER"
    );

    let plan = runtime
        .plan_path_from_dps_payload(&request(), payload)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    let schedule_explain = plan
        .explain
        .iter()
        .filter(|line| line.starts_with("multipath_schedule_"))
        .cloned()
        .collect::<Vec<String>>()
        .join("\n");
    let combined = format!("{schedule_explain}\n{:?}", plan.multipath_schedule);

    assert!(combined.contains("multipath_schedule_demand_policy=high"));
    assert!(combined.contains("multipath_schedule_transit_payload_policy=sealed_opaque_only"));
    assert!(!combined.contains("node-sensitive-a"));
    assert!(!combined.contains("node-sensitive-b"));
    assert!(!combined.contains("198.51.100.31"));
    assert!(!combined.contains("198.51.100.32"));
    assert!(!combined.contains("7023"));
    assert!(!combined.contains("SECRET_PAYLOAD_MARKER"));
}
