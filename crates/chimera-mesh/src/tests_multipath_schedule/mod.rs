use crate::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshMultipathLaneRole, MeshMultipathMode, MeshRuntime,
};

fn request() -> MeshJoinRequest {
    MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    }
}

fn runtime_with_peers(records: Vec<MeshDiscoveryRecord>) -> MeshRuntime {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    runtime
        .merge_discovery("seed-b", &records)
        .unwrap_or_else(|e| unreachable!("discovery merge should succeed: {e}"));
    runtime
}

fn record(
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

fn explain_has(plan_explain: &[String], expected: &str) -> bool {
    plan_explain.iter().any(|line| line.contains(expected))
}

fn assert_active_weight_contract(plan: &crate::MeshPathPlan) {
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
    assert!(
        plan.multipath_schedule
            .lanes
            .iter()
            .filter(|lane| lane.role == MeshMultipathLaneRole::Active)
            .all(|lane| lane.weight_pct > 0)
    );
    assert!(plan.multipath_schedule.local_traffic_reserve_pct > 0);
}

#[test]
fn multipath_schedule_off_uses_single_active_lane() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            "mesh_allowed_regions=eu;mesh_multipath_mode=off",
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
    assert_eq!(
        plan.multipath_schedule.lanes[0].peer_node_id,
        plan.selected_peers[0].node_id
    );
    assert!(explain_has(&plan.explain, "multipath_schedule_mode=off"));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_execution_status=planner_only_not_carrier_bound"
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
        "mesh_multipath_mode=standby_only"
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
    assert_eq!(
        plan.multipath_schedule.lanes[1].role,
        MeshMultipathLaneRole::Standby
    );
    assert_eq!(plan.multipath_schedule.lanes[1].weight_pct, 0);
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
    assert_active_weight_contract(&plan);
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
            "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard",
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
    assert_active_weight_contract(&plan);
}

#[test]
fn multipath_schedule_aggregate_buffered_uses_three_active_lanes() {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
        record("node-c", "198.51.100.33:443", "eu", 24, 92),
    ]);

    let plan = runtime
        .plan_path_from_dps_payload(
            &request(),
            "mesh_allowed_regions=eu;mesh_multipath_mode=aggregate_buffered",
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.selected_peers.len(), 3);
    assert_eq!(
        plan.multipath_schedule.mode,
        MeshMultipathMode::AggregateBuffered
    );
    assert_eq!(plan.multipath_schedule.active_lane_count, 3);
    assert_eq!(plan.multipath_schedule.standby_lane_count, 0);
    assert_eq!(plan.multipath_schedule.lanes.len(), 3);
    assert!(explain_has(&plan.explain, "effective_max_peers=3"));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_mode=aggregate_buffered"
    ));
    assert!(explain_has(
        &plan.explain,
        "multipath_schedule_active_lanes=3"
    ));
    assert_active_weight_contract(&plan);
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
            "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard",
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));

    assert_eq!(plan.selected_peers[0].node_id, "node-strong");
    assert_eq!(plan.selected_peers[1].node_id, "node-weaker");
    assert_eq!(plan.multipath_schedule.lanes[0].peer_node_id, "node-strong");
    assert_eq!(plan.multipath_schedule.lanes[1].peer_node_id, "node-weaker");
    assert!(
        plan.multipath_schedule.lanes[0].weight_pct > plan.multipath_schedule.lanes[1].weight_pct
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
    assert!(
        schedule_explain
            .contains("multipath_schedule_execution_status=planner_only_not_carrier_bound")
    );
    assert_eq!(
        plan.multipath_schedule.transit_payload_policy,
        "sealed_opaque_only"
    );
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
            "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard",
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    let debug = format!("{:?}", plan.multipath_schedule.lanes);

    assert!(!debug.contains("node-sensitive-a"));
    assert!(!debug.contains("node-sensitive-b"));
    assert!(debug.contains("<redacted>"));
}
