use super::*;
use crate::multipath_model::{
    MeshCarrierLaneBinding, MeshMultipathLane, MeshMultipathLaneRole, MeshRouteBindingId,
};

const LOCAL_TRAFFIC_RESERVE_PCT: u8 = 10;
const TRANSIT_CAPACITY_BUDGET_PCT: u8 = 100 - LOCAL_TRAFFIC_RESERVE_PCT;
const FAIRNESS_POLICY: &str = "weighted_round_robin_v1";
const EXECUTION_STATUS_CARRIER_BINDING_READY: &str = "carrier_lane_binding_contract_ready";
const EXECUTION_STATUS_PLANNER_ONLY_NOT_CARRIER_BOUND: &str = "planner_only_not_carrier_bound";
const TRANSIT_PAYLOAD_POLICY: &str = "sealed_opaque_only";

pub(super) fn build_multipath_schedule(
    selected_peers: &[MeshPeerState],
    mode: MeshMultipathMode,
    route_binding_id: Option<MeshRouteBindingId>,
) -> Result<MeshMultipathSchedule, String> {
    let lanes = build_lanes(selected_peers, &mode);
    schedule_from_lanes(selected_peers, mode, route_binding_id, lanes)
}

pub(super) fn replace_multipath_schedule(
    plan: &mut MeshPathPlan,
    mode: MeshMultipathMode,
    route_binding_id: Option<MeshRouteBindingId>,
) -> Result<(), String> {
    remove_multipath_schedule_explain(&mut plan.explain);
    plan.multipath_schedule =
        build_multipath_schedule(&plan.selected_peers, mode, route_binding_id)?;
    append_multipath_schedule_explain(&mut plan.explain, &plan.multipath_schedule);
    Ok(())
}

pub(super) fn append_multipath_schedule_explain(
    explain: &mut Vec<String>,
    schedule: &MeshMultipathSchedule,
) {
    explain.push(format!(
        "multipath_schedule_mode={}",
        schedule.mode.as_str()
    ));
    explain.push(format!(
        "multipath_schedule_execution_status={}",
        schedule.execution_status
    ));
    explain.push(format!(
        "multipath_schedule_carrier_binding_contract={}",
        schedule.execution_status
    ));
    explain.push(format!(
        "multipath_schedule_carrier_bindings={}",
        schedule.carrier_lane_bindings.len()
    ));
    explain.push(format!(
        "multipath_schedule_route_binding_configured={}",
        schedule.route_binding_id.is_some()
    ));
    explain.push(format!(
        "multipath_schedule_active_lanes={}",
        schedule.active_lane_count
    ));
    explain.push(format!(
        "multipath_schedule_standby_lanes={}",
        schedule.standby_lane_count
    ));
    explain.push(format!(
        "multipath_schedule_active_weight_sum_pct={}",
        schedule.active_weight_sum_pct
    ));
    explain.push(format!(
        "multipath_schedule_local_reserve_pct={}",
        schedule.local_traffic_reserve_pct
    ));
    explain.push(format!(
        "multipath_schedule_transit_capacity_budget_pct={}",
        schedule.transit_capacity_budget_pct
    ));
    explain.push(format!(
        "multipath_schedule_fairness_policy={}",
        schedule.fairness_policy
    ));
    explain.push(format!(
        "multipath_schedule_transit_payload_policy={}",
        schedule.transit_payload_policy
    ));
    explain.push(format!(
        "multipath_schedule_lanes={}",
        format_schedule_lanes(&schedule.lanes)
    ));
}

pub(super) fn schedule_mode_from_multipath_hint(mode: MultipathMode) -> MeshMultipathMode {
    match mode {
        MultipathMode::Off => MeshMultipathMode::Off,
        MultipathMode::StandbyOnly => MeshMultipathMode::StandbyOnly,
        MultipathMode::FlowShard => MeshMultipathMode::FlowShard,
        MultipathMode::AggregateBuffered => MeshMultipathMode::AggregateBuffered,
    }
}

fn build_lanes(
    selected_peers: &[MeshPeerState],
    mode: &MeshMultipathMode,
) -> Vec<MeshMultipathLane> {
    match mode {
        MeshMultipathMode::Off => build_active_lanes(selected_peers, 1),
        MeshMultipathMode::StandbyOnly => build_standby_lanes(selected_peers),
        MeshMultipathMode::FlowShard => build_active_lanes(selected_peers, 2),
        MeshMultipathMode::AggregateBuffered => build_active_lanes(selected_peers, 3),
    }
}

fn build_active_lanes(
    selected_peers: &[MeshPeerState],
    max_active: usize,
) -> Vec<MeshMultipathLane> {
    let active_peers: Vec<&MeshPeerState> = selected_peers.iter().take(max_active).collect();
    let weights = active_lane_weights(&active_peers);
    active_peers
        .into_iter()
        .zip(weights)
        .enumerate()
        .map(|(idx, (peer, weight_pct))| MeshMultipathLane {
            lane_id: idx,
            peer_node_id: peer.node_id.clone(),
            role: MeshMultipathLaneRole::Active,
            weight_pct,
        })
        .collect()
}

fn build_standby_lanes(selected_peers: &[MeshPeerState]) -> Vec<MeshMultipathLane> {
    let mut lanes = build_active_lanes(selected_peers, 1);
    if let Some(peer) = selected_peers.get(1) {
        lanes.push(MeshMultipathLane {
            lane_id: lanes.len(),
            peer_node_id: peer.node_id.clone(),
            role: MeshMultipathLaneRole::Standby,
            weight_pct: 0,
        });
    }
    lanes
}

fn schedule_from_lanes(
    selected_peers: &[MeshPeerState],
    mode: MeshMultipathMode,
    route_binding_id: Option<MeshRouteBindingId>,
    lanes: Vec<MeshMultipathLane>,
) -> Result<MeshMultipathSchedule, String> {
    let active_lane_count = lanes
        .iter()
        .filter(|lane| lane.role == MeshMultipathLaneRole::Active)
        .count();
    let standby_lane_count = lanes
        .iter()
        .filter(|lane| lane.role == MeshMultipathLaneRole::Standby)
        .count();
    let active_weight_sum_pct = lanes
        .iter()
        .filter(|lane| lane.role == MeshMultipathLaneRole::Active)
        .map(|lane| lane.weight_pct as u16)
        .sum();

    let carrier_lane_bindings = match route_binding_id.as_ref() {
        Some(route_binding_id) => {
            carrier_lane_bindings(selected_peers, &lanes, route_binding_id.clone())?
        }
        None => Vec::new(),
    };
    let execution_status = if route_binding_id.is_some() && !carrier_lane_bindings.is_empty() {
        EXECUTION_STATUS_CARRIER_BINDING_READY
    } else {
        EXECUTION_STATUS_PLANNER_ONLY_NOT_CARRIER_BOUND
    };

    Ok(MeshMultipathSchedule {
        mode,
        carrier_lane_bindings,
        route_binding_id,
        lanes,
        active_lane_count,
        standby_lane_count,
        active_weight_sum_pct,
        local_traffic_reserve_pct: LOCAL_TRAFFIC_RESERVE_PCT,
        transit_capacity_budget_pct: TRANSIT_CAPACITY_BUDGET_PCT,
        fairness_policy: FAIRNESS_POLICY.to_string(),
        execution_status: execution_status.to_string(),
        transit_payload_policy: TRANSIT_PAYLOAD_POLICY.to_string(),
    })
}

fn carrier_lane_bindings(
    selected_peers: &[MeshPeerState],
    lanes: &[MeshMultipathLane],
    route_binding_id: MeshRouteBindingId,
) -> Result<Vec<MeshCarrierLaneBinding>, String> {
    lanes
        .iter()
        .map(|lane| {
            let peer = selected_peers
                .iter()
                .find(|peer| peer.node_id == lane.peer_node_id)
                .ok_or_else(|| "mesh carrier lane binding missing selected peer".to_string())?;
            Ok(MeshCarrierLaneBinding {
                route_binding_id: route_binding_id.clone(),
                lane_id: lane.lane_id,
                peer_node_id: lane.peer_node_id.clone(),
                carrier_endpoint: peer.endpoint.clone(),
                role: lane.role.clone(),
                weight_pct: lane.weight_pct,
            })
        })
        .collect()
}

fn active_lane_weights(active_peers: &[&MeshPeerState]) -> Vec<u8> {
    if active_peers.is_empty() {
        return Vec::new();
    }
    let scores: Vec<u16> = active_peers
        .iter()
        .map(|peer| lane_weight_score(peer))
        .collect();
    let total: u16 = scores.iter().sum();
    if total == 0 {
        return even_weights(active_peers.len());
    }

    let mut weights: Vec<u8> = scores
        .iter()
        .map(|score| ((*score as usize * 100) / total as usize).max(1) as u8)
        .collect();
    normalize_weights_to_100(&mut weights);
    weights
}

fn lane_weight_score(peer: &MeshPeerState) -> u16 {
    let reliability = peer.reliability_score.max(1) as u16;
    let load_headroom = 100_u16.saturating_sub(peer.load_score as u16).max(1);
    let selected_score = peer.selection_score.max(0) as u16;
    reliability
        .saturating_add(load_headroom)
        .saturating_add(selected_score / 4)
        .max(1)
}

fn even_weights(count: usize) -> Vec<u8> {
    let base = 100 / count;
    let remainder = 100 % count;
    (0..count)
        .map(|idx| {
            let extra = usize::from(idx < remainder);
            (base + extra) as u8
        })
        .collect()
}

fn normalize_weights_to_100(weights: &mut [u8]) {
    let sum: i16 = weights.iter().map(|weight| *weight as i16).sum();
    let delta = 100_i16.saturating_sub(sum);
    if delta == 0 || weights.is_empty() {
        return;
    }
    if delta > 0 {
        if let Some(first) = weights.first_mut() {
            *first = first.saturating_add(delta as u8);
        }
        return;
    }
    let mut remaining = delta.unsigned_abs() as u8;
    for weight in weights.iter_mut().rev() {
        if remaining == 0 {
            break;
        }
        let removable = weight.saturating_sub(1).min(remaining);
        *weight = weight.saturating_sub(removable);
        remaining = remaining.saturating_sub(removable);
    }
}

fn format_schedule_lanes(lanes: &[MeshMultipathLane]) -> String {
    if lanes.is_empty() {
        return "none".to_string();
    }
    lanes
        .iter()
        .map(|lane| {
            format!(
                "lane{}:{}:{}",
                lane.lane_id,
                lane.role.as_str(),
                lane.weight_pct
            )
        })
        .collect::<Vec<String>>()
        .join("|")
}

fn remove_multipath_schedule_explain(explain: &mut Vec<String>) {
    explain.retain(|line| !line.starts_with("multipath_schedule_"));
}

#[cfg(test)]
mod tests {
    use super::schedule_from_lanes;
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
        }];
        let route_binding_id = MeshRouteBindingId::new(77)?;

        let error = match schedule_from_lanes(
            &selected_peers,
            MeshMultipathMode::FlowShard,
            Some(route_binding_id),
            lanes,
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
}
