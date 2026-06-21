use super::multipath_demand::plan_multipath_demand;
use super::multipath_schedule::{build_multipath_schedule, schedule_from_lanes};
use crate::model::MeshPeerState;
use crate::multipath_model::{
    MeshMultipathLane, MeshMultipathLaneRole, MeshMultipathMode, MeshRouteBindingId,
};

fn peer(node_id: &str) -> MeshPeerState {
    MeshPeerState {
        node_id: node_id.to_string(),
        endpoint: "198.51.100.50:443".to_string(),
        region: "eu".to_string(),
        reliability_score: 90,
        load_score: 10,
        latency_ms: None,
        throughput_mbps: None,
        selection_score: 170,
    }
}

#[test]
fn carrier_lane_binding_fails_closed_when_lane_peer_is_not_selected() -> Result<(), String> {
    let selected_peers = vec![peer("selected-peer")];
    let lanes = vec![MeshMultipathLane {
        lane_id: 0,
        peer_node_id: "missing-peer".to_string(),
        role: MeshMultipathLaneRole::Active,
        weight_pct: 100,
        capacity_weight_pct: 90,
    }];
    let route_binding_id = MeshRouteBindingId::new(77)?;

    let error = match schedule_from_lanes(
        &selected_peers,
        MeshMultipathMode::FlowShard,
        Some(route_binding_id),
        plan_multipath_demand(
            &MeshMultipathMode::FlowShard,
            None,
            selected_peers.len(),
            90,
        ),
        lanes,
        "initial_plan",
    ) {
        Ok(_) => {
            return Err(
                "missing lane peer must fail instead of silently dropping binding".to_string(),
            );
        }
        Err(error) => error,
    };

    assert!(error.contains("missing selected peer"));
    Ok(())
}

#[test]
fn aggregate_buffered_limits_active_lanes_to_capacity_budget() -> Result<(), String> {
    let selected_peers = (0..95)
        .map(|idx| peer(&format!("node-{idx}")))
        .collect::<Vec<MeshPeerState>>();

    let schedule = build_multipath_schedule(
        &selected_peers,
        MeshMultipathMode::AggregateBuffered,
        Some(MeshRouteBindingId::new(78)?),
        None,
    )?;

    assert_eq!(
        schedule.active_lane_count,
        schedule.transit_capacity_budget_pct as usize
    );
    assert_eq!(
        schedule.active_capacity_sum_pct,
        schedule.transit_capacity_budget_pct as u16
    );
    assert!(
        schedule
            .lanes
            .iter()
            .all(|lane| lane.capacity_weight_pct > 0)
    );
    Ok(())
}
