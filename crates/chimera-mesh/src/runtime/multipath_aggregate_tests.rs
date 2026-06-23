use super::multipath_aggregate::plan_multipath_aggregate_object;
use crate::model::{MeshDiscoveryRecord, MeshJoinRequest, MeshPathPlan};
use crate::multipath_model::{MeshMultipathMode, MeshRouteBindingId};

fn runtime_with_peers(records: Vec<MeshDiscoveryRecord>) -> super::MeshRuntime {
    let mut runtime = super::MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    runtime
        .merge_discovery("seed-b", &records)
        .unwrap_or_else(|e| unreachable!("discovery merge should succeed: {e}"));
    runtime
}

fn request() -> MeshJoinRequest {
    MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    }
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

fn plan() -> MeshPathPlan {
    let runtime = runtime_with_peers(vec![
        record("node-a", "198.51.100.31:443", "eu", 20, 90),
        record("node-b", "198.51.100.32:443", "eu", 22, 91),
        record("node-c", "198.51.100.33:443", "eu", 24, 92),
    ]);
    runtime
        .plan_path_from_dps_payload(
            &request(),
            "mesh_allowed_regions=eu;mesh_max_peers=3;mesh_max_selected_per_region=3;mesh_multipath_mode=aggregate_buffered;mesh_route_binding_id=7301",
        )
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"))
}

#[test]
fn aggregate_object_plan_shards_object_across_active_lanes() -> Result<(), String> {
    let plan = plan();
    let aggregate = plan_multipath_aggregate_object(&plan.multipath_schedule, 3072);

    assert_eq!(
        aggregate.action,
        super::MeshMultipathAggregateAction::Assigned
    );
    assert_eq!(aggregate.active_binding_count, 3);
    assert_eq!(aggregate.shards.len(), 3);
    assert_eq!(aggregate.object_bytes, 3072);
    assert_eq!(aggregate.total_capacity_weight_pct, 90);
    assert_eq!(aggregate.local_traffic_reserve_pct, 10);
    assert_eq!(aggregate.transit_capacity_budget_pct, 90);
    assert!(aggregate.rebuild_reason == "none" || aggregate.rebuild_recommended);
    assert!(
        aggregate
            .explain
            .iter()
            .any(|line| line == "multipath_aggregate_action=assigned")
    );
    assert!(
        aggregate
            .explain
            .iter()
            .any(|line| line == "multipath_aggregate_privacy=sealed_opaque_only")
    );

    let total_bytes: u64 = aggregate.shards.iter().map(|shard| shard.byte_len).sum();
    assert_eq!(total_bytes, aggregate.object_bytes);

    let mut expected_offset = 0_u64;
    for shard in &aggregate.shards {
        assert!(shard.byte_len > 0);
        assert_eq!(shard.byte_offset, expected_offset);
        expected_offset = expected_offset.saturating_add(shard.byte_len);
        assert_eq!(
            shard.route_binding_id.get(),
            plan.multipath_schedule
                .route_binding_id
                .as_ref()
                .ok_or_else(|| "route binding missing".to_string())?
                .get()
        );
    }

    let debug = format!("{aggregate:?}");
    assert!(!debug.contains("3072"));
    assert!(!debug.contains("198.51.100.31"));
    Ok(())
}

#[test]
fn aggregate_object_plan_fail_closed_when_mode_is_not_aggregate() -> Result<(), String> {
    let mut plan = plan();
    plan.multipath_schedule.mode = MeshMultipathMode::FlowShard;

    let aggregate = plan_multipath_aggregate_object(&plan.multipath_schedule, 1024);
    assert_eq!(
        aggregate.action,
        super::MeshMultipathAggregateAction::FailClosed
    );
    assert_eq!(aggregate.reason, "aggregate_mode_required");
    assert!(aggregate.shards.is_empty());
    Ok(())
}

#[test]
fn aggregate_object_plan_fail_closed_when_local_reserve_is_invalid() -> Result<(), String> {
    let mut plan = plan();
    plan.multipath_schedule.local_traffic_reserve_pct = 0;

    let aggregate = plan_multipath_aggregate_object(&plan.multipath_schedule, 1024);
    assert_eq!(
        aggregate.action,
        super::MeshMultipathAggregateAction::FailClosed
    );
    assert_eq!(aggregate.reason, "local_reserve_invalid");
    assert!(aggregate.shards.is_empty());
    Ok(())
}

#[test]
fn aggregate_object_plan_fail_closed_when_route_binding_is_missing() -> Result<(), String> {
    let mut plan = plan();
    plan.multipath_schedule.route_binding_id = None;
    plan.multipath_schedule.carrier_lane_bindings.clear();

    let aggregate = plan_multipath_aggregate_object(&plan.multipath_schedule, 1024);

    assert_eq!(
        aggregate.action,
        super::MeshMultipathAggregateAction::FailClosed
    );
    assert_eq!(aggregate.reason, "route_binding_missing");
    assert!(aggregate.shards.is_empty());
    assert!(
        aggregate
            .explain
            .iter()
            .any(|line| line == "multipath_aggregate_action=fail_closed")
    );
    Ok(())
}

#[test]
fn aggregate_object_plan_fail_closed_when_lane_binding_mismatches_route() -> Result<(), String> {
    let mut plan = plan();
    plan.multipath_schedule.carrier_lane_bindings[0].route_binding_id =
        MeshRouteBindingId::new(9001)?;

    let aggregate = plan_multipath_aggregate_object(&plan.multipath_schedule, 1024);

    assert_eq!(
        aggregate.action,
        super::MeshMultipathAggregateAction::FailClosed
    );
    assert_eq!(aggregate.reason, "route_binding_mismatch");
    assert!(aggregate.shards.is_empty());
    Ok(())
}
