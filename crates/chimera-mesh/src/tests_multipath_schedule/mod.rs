mod capacity_budget;
mod direct_planning;
mod helpers;

use crate::{MeshMultipathLaneRole, MeshMultipathMode};
use helpers::{
    assert_active_weight_contract, assert_binding_matches_lane, assert_carrier_binding_contract,
    explain_has, record, request, runtime_with_peers,
};

#[test]
fn multipath_schedule_off_uses_single_active_lane() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            "mesh_allowed_regions=eu;mesh_multipath_mode=off;mesh_route_binding_id=7001",
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.selected_peers.len(), 1);
    assert_eq!(plan.multipath_schedule.mode, MeshMultipathMode::Off);
    assert_eq!(plan.multipath_schedule.active_lane_count, 1);
    assert_eq!(plan.multipath_schedule.standby_lane_count, 0);
    assert_eq!(
        plan.multipath_schedule.lanes[0].role,
        MeshMultipathLaneRole::Active
    );
    assert_eq!(plan.multipath_schedule.lanes[0].weight_pct, 100);
    assert_eq!(plan.multipath_schedule.lanes[0].capacity_weight_pct, 90);
    assert_eq!(
        plan.multipath_schedule.lanes[0].peer_node_id,
        plan.selected_peers[0].node_id
    );
    assert!(explain_has(&plan.explain, "multipath_schedule_mode=off"));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_execution_status=carrier_lane_binding_contract_ready"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_carrier_binding_contract=carrier_lane_binding_contract_ready"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_carrier_bindings=1"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_route_binding_configured=true"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_transit_capacity_budget_pct=90"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_active_capacity_sum_pct=90"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_planner_rebuild_reason=multipath_hint_replan"
    ));
    assert_eq!(plan.multipath_schedule.carrier_lane_bindings.len(), 1);
    assert_binding_matches_lane(&plan, 0, "node-a", "198.51.100.31:443");
    assert_active_weight_contract(&plan);
    assert_carrier_binding_contract(&plan);
}

#[test]
fn multipath_schedule_without_route_binding_id_keeps_carrier_bindings_closed() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard",
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.multipath_schedule.active_lane_count, 2);
    assert_eq!(plan.multipath_schedule.carrier_lane_bindings.len(), 0);
    assert!(plan.multipath_schedule.route_binding_id.is_none());
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_carrier_bindings=0"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_route_binding_configured=false"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_execution_status=planner_only_not_carrier_bound"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_carrier_binding_contract=planner_only_not_carrier_bound"
    ));
    assert_active_weight_contract(&plan);
}

#[test]
fn multipath_schedule_standby_only_keeps_one_active_and_one_standby() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
    ]);
    let payload = concat!(
        "mesh_allowed_regions=eu;",
        "mesh_max_peers=2;",
        "mesh_max_selected_per_region=2;",
        "mesh_multipath_mode=standby_only;",
        "mesh_route_binding_id=7002"
    );

    let plan = runtime
        .plan_path_from_dps_payload(&request(), payload)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.selected_peers.len(), 2);
    assert_eq!(plan.multipath_schedule.mode, MeshMultipathMode::StandbyOnly);
    assert_eq!(plan.multipath_schedule.active_lane_count, 1);
    assert_eq!(plan.multipath_schedule.standby_lane_count, 1);
    assert_eq!(
        plan.multipath_schedule.lanes[0].role,
        MeshMultipathLaneRole::Active
    );
    assert_eq!(plan.multipath_schedule.lanes[0].weight_pct, 100);
    assert_eq!(plan.multipath_schedule.lanes[0].capacity_weight_pct, 90);
    assert_eq!(
        plan.multipath_schedule.lanes[1].role,
        MeshMultipathLaneRole::Standby
    );
    assert_eq!(plan.multipath_schedule.lanes[1].weight_pct, 0);
    assert_eq!(plan.multipath_schedule.lanes[1].capacity_weight_pct, 0);
    assert_eq!(
        plan.multipath_schedule.lanes[0].peer_node_id,
        plan.selected_peers[0].node_id
    );
    assert_eq!(
        plan.multipath_schedule.lanes[1].peer_node_id,
        plan.selected_peers[1].node_id
    );
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_mode=standby_only"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_active_lanes=1"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_standby_lanes=1"
    ));
    assert_eq!(plan.multipath_schedule.carrier_lane_bindings.len(), 2);
    assert_binding_matches_lane(&plan, 0, "node-a", "198.51.100.31:443");
    assert_binding_matches_lane(&plan, 1, "node-b", "198.51.100.32:443");
    assert_active_weight_contract(&plan);
    assert_carrier_binding_contract(&plan);
}

#[test]
fn multipath_schedule_flow_shard_uses_multiple_active_lanes() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
        record("node-c", "198.51.100.33:443", "eu", 35, 89),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard;mesh_route_binding_id=7003",
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.selected_peers.len(), 2);
    assert_eq!(plan.multipath_schedule.mode, MeshMultipathMode::FlowShard);
    assert_eq!(plan.multipath_schedule.active_lane_count, 2);
    assert_eq!(plan.multipath_schedule.standby_lane_count, 0);
    assert_eq!(plan.multipath_schedule.lanes.len(), 2);
    assert!(
        plan.multipath_schedule
            .lanes
            .iter()
            .all(|lane| lane.role == MeshMultipathLaneRole::Active)
    );
    assert_eq!(
        plan.multipath_schedule.lanes[0].peer_node_id,
        plan.selected_peers[0].node_id
    );
    assert_eq!(
        plan.multipath_schedule.lanes[1].peer_node_id,
        plan.selected_peers[1].node_id
    );
    assert!(explain_has(&plan.explain, "effective_max_peers=2"));
    assert!(explain_has(
        &plan.explain,
        "effective_max_selected_per_region=2"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_mode=flow_shard"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_active_lanes=2"
    ));
    assert_eq!(plan.multipath_schedule.carrier_lane_bindings.len(), 2);
    assert_binding_matches_lane(&plan, 0, "node-a", "198.51.100.31:443");
    assert_binding_matches_lane(&plan, 1, "node-b", "198.51.100.32:443");
    assert_active_weight_contract(&plan);
    assert_carrier_binding_contract(&plan);
}

#[test]
fn multipath_schedule_aggregate_buffered_uses_all_policy_selected_peers() {
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
            "mesh_allowed_regions=eu;mesh_max_peers=5;mesh_max_selected_per_region=5;mesh_multipath_mode=aggregate_buffered;mesh_route_binding_id=7004",
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.selected_peers.len(), 5);
    assert_eq!(
        plan.multipath_schedule.mode,
        MeshMultipathMode::AggregateBuffered
    );
    assert_eq!(plan.multipath_schedule.active_lane_count, 5);
    assert_eq!(plan.multipath_schedule.standby_lane_count, 0);
    assert_eq!(plan.multipath_schedule.lanes.len(), 5);
    assert!(explain_has(&plan.explain, "effective_max_peers=5"));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_mode=aggregate_buffered"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_active_lanes=5"
    ));
    assert_eq!(plan.multipath_schedule.carrier_lane_bindings.len(), 5);
    assert_binding_matches_lane(&plan, 0, "node-a", "198.51.100.31:443");
    assert_binding_matches_lane(&plan, 1, "node-b", "198.51.100.32:443");
    assert_binding_matches_lane(&plan, 2, "node-c", "198.51.100.33:443");
    assert_binding_matches_lane(&plan, 3, "node-d", "198.51.100.34:443");
    assert_binding_matches_lane(&plan, 4, "node-e", "198.51.100.35:443");
    assert_active_weight_contract(&plan);
    assert_carrier_binding_contract(&plan);
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
fn multipath_schedule_explain_does_not_leak_payload_destination_or_endpoint() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
    ]);
    let payload = concat!(
        "mesh_allowed_regions=eu;",
        "mesh_multipath_mode=flow_shard;",
        "mesh_route_binding_id=7006;",
        "non_mesh_note=SECRET_DESTINATION_EXAMPLE"
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

    assert!(!schedule_explain.contains("SECRET_DESTINATION_EXAMPLE"));
    assert!(!schedule_explain.contains("198.51.100.31:443"));
    assert!(!schedule_explain.contains("198.51.100.32:443"));
    assert!(
        schedule_explain.contains("multipath_schedule_transit_payload_policy=sealed_opaque_only")
    );
    assert!(schedule_explain.contains("multipath_schedule_active_capacity_sum_pct=90"));
    assert!(
        schedule_explain
            .contains("multipath_schedule_planner_rebuild_reason=multipath_hint_replan")
    );
    assert!(
        schedule_explain
            .contains("multipath_schedule_execution_status=carrier_lane_binding_contract_ready")
    );
    assert_eq!(
        plan.multipath_schedule.transit_payload_policy,
        "sealed_opaque_only"
    );
}

#[test]
fn multipath_plan_public_explain_redacts_peer_identity_endpoint_and_connect_plan() {
    let runtime = runtime_with_peers(vec![
        record("node-sensitive-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-sensitive-b", "198.51.100.32:9443", "eu", 22, 91),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard;mesh_route_binding_id=7009",
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    let explain = plan.explain.join("\n");

    assert!(explain.contains("selected_peer_ids=peer#1,peer#2"));
    assert!(
        explain.contains("selected_peer_endpoints=endpoint#1:<redacted>,endpoint#2:<redacted>")
    );
    assert!(
        explain.contains("selected_peer_connect_priority=1:peer#1@<redacted>,2:peer#2@<redacted>")
    );
    assert!(explain.contains("selected_peer_scores=peer#1:"));
    assert!(explain.contains("standby_shadow_target=peer#"));
    assert!(explain.contains("multipath_schedule_transit_payload_policy=sealed_opaque_only"));
    assert!(!explain.contains("node-sensitive-a"));
    assert!(!explain.contains("node-sensitive-b"));
    assert!(!explain.contains("198.51.100.31"));
    assert!(!explain.contains("198.51.100.32"));
    assert!(!explain.contains("7009"));
}

#[test]
fn multipath_schedule_debug_redacts_lane_peer_identity() {
    let runtime = runtime_with_peers(vec![
        record("node-sensitive-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-sensitive-b", "198.51.100.32:443", "eu", 22, 91),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard;mesh_route_binding_id=7007",
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    let debug = format!("{:?}", plan.multipath_schedule.lanes);

    assert!(!debug.contains("node-sensitive-a"));
    assert!(!debug.contains("node-sensitive-b"));
    assert!(!debug.contains("198.51.100.31:443"));
    assert!(!debug.contains("198.51.100.32:443"));
    assert!(debug.contains("<redacted>"));
    assert!(debug.contains("capacity_weight_pct"));
}

#[test]
fn multipath_carrier_binding_debug_redacts_peer_identity_and_endpoint() {
    let runtime = runtime_with_peers(vec![
        record("node-sensitive-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-sensitive-b", "198.51.100.32:443", "eu", 22, 91),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard;mesh_route_binding_id=7008",
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    let debug = format!("{:?}", plan.multipath_schedule.carrier_lane_bindings);

    assert!(!debug.contains("node-sensitive-a"));
    assert!(!debug.contains("node-sensitive-b"));
    assert!(!debug.contains("198.51.100.31:443"));
    assert!(!debug.contains("198.51.100.32:443"));
    let route_binding_id = plan
        .multipath_schedule
        .route_binding_id
        .as_ref()
        .unwrap_or_else(|| unreachable!("route binding id should be configured"));
    assert!(!debug.contains(&route_binding_id.get().to_string()));
    assert!(debug.contains("<opaque>"));
    assert!(debug.contains("<redacted>"));
    assert!(debug.contains("capacity_weight_pct"));
}
