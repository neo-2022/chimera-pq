use super::helpers::{record, request, runtime_with_peers};

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
        schedule_explain.contains("multipath_schedule_lane_admission_requested_active_lanes=2")
    );
    assert!(schedule_explain.contains("multipath_schedule_lane_admission_admitted_active_lanes=2"));
    assert!(schedule_explain.contains("multipath_schedule_lane_admission_rejected_active_lanes=0"));
    assert!(
        schedule_explain
            .contains("multipath_schedule_lane_admission_capacity_status=within_budget")
    );
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
    assert!(explain.contains("multipath_schedule_lane_admission_capacity_status=within_budget"));
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
