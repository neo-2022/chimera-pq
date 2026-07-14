use super::multipath_demand::plan_multipath_demand;
use super::multipath_schedule::{build_multipath_schedule, schedule_from_lanes};
use crate::model::MeshPeerState;
use crate::multipath_model::{
    MeshMultipathLane, MeshMultipathLaneRole, MeshMultipathMode, MeshRouteBindingId,
};
use crate::route_announcement::{CapabilityToken, PeerId, RouteAnnouncement, RouteDestination};
use std::time::Duration;

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
            90,
        ),
        lanes,
        &[],
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
fn route_announcement_creates_transit_binding_for_selected_via_peer() -> Result<(), String> {
    let selected_peers = vec![peer("active-node"), peer("via-node")];
    let route_binding_id = MeshRouteBindingId::new(77)?;
    let announcements = vec![RouteAnnouncement::Static {
        destination: RouteDestination::Domain("example.com".to_string()),
        via: PeerId::new("via-node")?,
        route_binding_id,
        ttl: Duration::from_secs(300),
        auth: CapabilityToken::new(
            PeerId::new("issuer")?,
            RouteDestination::Domain("example.com".to_string()),
            None,
            vec![],
        ),
    }];

    let schedule = build_multipath_schedule(
        &selected_peers,
        MeshMultipathMode::Off,
        Some(route_binding_id),
        None,
        &announcements,
    )?;

    assert_eq!(schedule.route_announcements.len(), 1);
    let transit = schedule
        .carrier_lane_bindings
        .iter()
        .find(|binding| binding.peer_node_id == "via-node")
        .ok_or("missing transit carrier binding for announced via peer")?;
    assert_eq!(transit.role, MeshMultipathLaneRole::Transit);
    assert_eq!(transit.route_binding_id, route_binding_id);
    assert!(
        schedule
            .carrier_lane_bindings
            .iter()
            .any(|binding| binding.peer_node_id == "active-node"
                && binding.role == MeshMultipathLaneRole::Active)
    );
    Ok(())
}

#[test]
fn route_announcement_requires_route_binding_id_to_build_bindings() {
    let selected_peers = vec![peer("active-node"), peer("via-node")];
    let via = PeerId::new("via-node").unwrap_or_else(|error| unreachable!("{error}"));
    let route_binding_id =
        MeshRouteBindingId::new(77).unwrap_or_else(|error| unreachable!("{error}"));
    let issuer = PeerId::new("issuer").unwrap_or_else(|error| unreachable!("{error}"));
    let announcements = vec![RouteAnnouncement::Static {
        destination: RouteDestination::Domain("example.com".to_string()),
        via,
        route_binding_id,
        ttl: Duration::from_secs(300),
        auth: CapabilityToken::new(
            issuer,
            RouteDestination::Domain("example.com".to_string()),
            None,
            vec![],
        ),
    }];

    let result = build_multipath_schedule(
        &selected_peers,
        MeshMultipathMode::Off,
        None,
        None,
        &announcements,
    );

    assert!(result.is_err());
}

#[test]
fn route_announcement_for_unknown_via_is_ignored() -> Result<(), String> {
    let selected_peers = vec![peer("active-node")];
    let route_binding_id = MeshRouteBindingId::new(77)?;
    let announcements = vec![RouteAnnouncement::Static {
        destination: RouteDestination::Domain("example.com".to_string()),
        via: PeerId::new("unknown-node")?,
        route_binding_id,
        ttl: Duration::from_secs(300),
        auth: CapabilityToken::new(
            PeerId::new("issuer")?,
            RouteDestination::Domain("example.com".to_string()),
            None,
            vec![],
        ),
    }];

    let schedule = build_multipath_schedule(
        &selected_peers,
        MeshMultipathMode::Off,
        Some(route_binding_id),
        None,
        &announcements,
    )?;

    assert!(
        schedule
            .carrier_lane_bindings
            .iter()
            .all(|binding| binding.peer_node_id == "active-node")
    );
    Ok(())
}

#[test]
fn aggregate_buffered_limits_active_lanes_to_nonzero_capacity_slots() -> Result<(), String> {
    let selected_peers = (0..95)
        .map(|idx| peer(&format!("node-{idx}")))
        .collect::<Vec<MeshPeerState>>();

    let schedule = build_multipath_schedule(
        &selected_peers,
        MeshMultipathMode::AggregateBuffered,
        Some(MeshRouteBindingId::new(78)?),
        None,
        &[],
    )?;

    assert_eq!(
        schedule.active_lane_count,
        schedule.lane_admission_admitted_active_lane_count
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
