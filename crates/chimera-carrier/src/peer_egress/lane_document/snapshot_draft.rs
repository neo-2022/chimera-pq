use std::collections::BTreeMap;

use chimera_mesh::{
    MeshCarrierLaneBinding, MeshJoinMode, MeshMultipathLane, MeshMultipathLaneRole,
    MeshMultipathMode, MeshMultipathSchedule, MeshPathPlan, MeshPeerState, MeshRouteBindingId,
};

use super::format::validate_snapshot_capacity_contract;

#[derive(Default)]
pub(super) struct TransitLanePlanSnapshotDraft {
    pub(super) snapshot_seen: bool,
    pub(super) version_seen: bool,
    pub(super) namespace: Option<String>,
    pub(super) join_mode: Option<MeshJoinMode>,
    pub(super) selected_peers: BTreeMap<usize, MeshPeerState>,
    pub(super) mode: Option<MeshMultipathMode>,
    pub(super) route_binding_id: Option<MeshRouteBindingId>,
    pub(super) lane_requested: Option<usize>,
    pub(super) lane_admitted: Option<usize>,
    pub(super) lane_rejected: Option<usize>,
    pub(super) lane_capacity_status: Option<String>,
    pub(super) local_reserve_pct: Option<u8>,
    pub(super) transit_budget_pct: Option<u8>,
    pub(super) demand_policy: Option<String>,
    pub(super) demand_policy_source: Option<String>,
    pub(super) demand_requested: Option<usize>,
    pub(super) demand_planned: Option<usize>,
    pub(super) demand_admitted_capacity_pct: Option<u8>,
    pub(super) demand_unmet: Option<usize>,
    pub(super) demand_status: Option<String>,
    pub(super) demand_rebuild_recommended: Option<bool>,
    pub(super) fairness_policy: Option<String>,
    pub(super) execution_status: Option<String>,
    pub(super) transit_payload_policy: Option<String>,
    pub(super) planner_rebuild_reason: Option<String>,
    pub(super) active_lane_count: Option<usize>,
    pub(super) standby_lane_count: Option<usize>,
    pub(super) active_weight_sum_pct: Option<u16>,
    pub(super) active_capacity_sum_pct: Option<u16>,
    pub(super) carrier_bindings: BTreeMap<usize, MeshCarrierLaneBinding>,
    pub(super) explain: BTreeMap<usize, String>,
}

impl TransitLanePlanSnapshotDraft {
    pub(super) fn finish(self) -> Result<Option<MeshPathPlan>, String> {
        if !self.snapshot_seen {
            return Ok(None);
        }
        if !self.version_seen {
            return Err("transit plan snapshot version missing".to_string());
        }
        let namespace = self
            .namespace
            .ok_or_else(|| "transit plan snapshot namespace missing".to_string())?;
        let join_mode = self
            .join_mode
            .ok_or_else(|| "transit plan snapshot join mode missing".to_string())?;
        let mode = self
            .mode
            .ok_or_else(|| "transit plan snapshot mode missing".to_string())?;
        let lane_requested = self.lane_requested.ok_or_else(|| {
            "transit plan snapshot lane admission requested count missing".to_string()
        })?;
        let lane_admitted = self.lane_admitted.ok_or_else(|| {
            "transit plan snapshot lane admission admitted count missing".to_string()
        })?;
        let lane_rejected = self.lane_rejected.ok_or_else(|| {
            "transit plan snapshot lane admission rejected count missing".to_string()
        })?;
        let lane_capacity_status = self
            .lane_capacity_status
            .ok_or_else(|| "transit plan snapshot lane capacity status missing".to_string())?;
        let local_reserve_pct = self
            .local_reserve_pct
            .ok_or_else(|| "transit plan snapshot local reserve pct missing".to_string())?;
        let transit_budget_pct = self
            .transit_budget_pct
            .ok_or_else(|| "transit plan snapshot transit budget pct missing".to_string())?;
        let demand_policy = self
            .demand_policy
            .ok_or_else(|| "transit plan snapshot demand policy missing".to_string())?;
        let demand_policy_source = self
            .demand_policy_source
            .ok_or_else(|| "transit plan snapshot demand policy source missing".to_string())?;
        let demand_requested = self
            .demand_requested
            .ok_or_else(|| "transit plan snapshot demand requested count missing".to_string())?;
        let demand_planned = self
            .demand_planned
            .ok_or_else(|| "transit plan snapshot demand planned count missing".to_string())?;
        let demand_admitted_capacity_pct = self.demand_admitted_capacity_pct.ok_or_else(|| {
            "transit plan snapshot demand admitted capacity pct missing".to_string()
        })?;
        let demand_unmet = self
            .demand_unmet
            .ok_or_else(|| "transit plan snapshot demand unmet count missing".to_string())?;
        let demand_status = self
            .demand_status
            .ok_or_else(|| "transit plan snapshot demand status missing".to_string())?;
        let demand_rebuild_recommended = self
            .demand_rebuild_recommended
            .ok_or_else(|| "transit plan snapshot demand rebuild flag missing".to_string())?;
        let fairness_policy = self
            .fairness_policy
            .ok_or_else(|| "transit plan snapshot fairness policy missing".to_string())?;
        let execution_status = self
            .execution_status
            .ok_or_else(|| "transit plan snapshot execution status missing".to_string())?;
        let transit_payload_policy = self
            .transit_payload_policy
            .ok_or_else(|| "transit plan snapshot transit payload policy missing".to_string())?;
        let planner_rebuild_reason = self
            .planner_rebuild_reason
            .ok_or_else(|| "transit plan snapshot planner rebuild reason missing".to_string())?;
        let active_lane_count = self
            .active_lane_count
            .ok_or_else(|| "transit plan snapshot active lane count missing".to_string())?;
        let standby_lane_count = self
            .standby_lane_count
            .ok_or_else(|| "transit plan snapshot standby lane count missing".to_string())?;
        let active_weight_sum_pct = self
            .active_weight_sum_pct
            .ok_or_else(|| "transit plan snapshot active weight sum missing".to_string())?;
        let active_capacity_sum_pct = self
            .active_capacity_sum_pct
            .ok_or_else(|| "transit plan snapshot active capacity sum missing".to_string())?;

        let selected_peers = ordered_selected_peers(self.selected_peers)?;
        let carrier_lane_bindings = ordered_carrier_bindings(self.carrier_bindings)?;
        let lanes = carrier_lane_bindings
            .iter()
            .map(|binding| MeshMultipathLane {
                lane_id: binding.lane_id,
                peer_node_id: binding.peer_node_id.clone(),
                role: binding.role.clone(),
                weight_pct: binding.weight_pct,
                capacity_weight_pct: binding.capacity_weight_pct,
            })
            .collect::<Vec<_>>();
        let derived_active_lane_count = lanes
            .iter()
            .filter(|lane| lane.role == MeshMultipathLaneRole::Active)
            .count();
        let derived_standby_lane_count = lanes
            .iter()
            .filter(|lane| lane.role == MeshMultipathLaneRole::Standby)
            .count();
        let derived_active_weight_sum_pct: u16 = lanes
            .iter()
            .filter(|lane| lane.role == MeshMultipathLaneRole::Active)
            .map(|lane| lane.weight_pct as u16)
            .sum();
        let derived_active_capacity_sum_pct: u16 = lanes
            .iter()
            .filter(|lane| lane.role == MeshMultipathLaneRole::Active)
            .map(|lane| lane.capacity_weight_pct as u16)
            .sum();
        if derived_active_lane_count != active_lane_count {
            return Err("transit plan snapshot active lane count mismatch".to_string());
        }
        if derived_standby_lane_count != standby_lane_count {
            return Err("transit plan snapshot standby lane count mismatch".to_string());
        }
        if derived_active_weight_sum_pct != active_weight_sum_pct {
            return Err("transit plan snapshot active weight sum mismatch".to_string());
        }
        if derived_active_capacity_sum_pct != active_capacity_sum_pct {
            return Err("transit plan snapshot active capacity sum mismatch".to_string());
        }
        validate_snapshot_capacity_contract(
            local_reserve_pct,
            transit_budget_pct,
            active_capacity_sum_pct,
            &transit_payload_policy,
        )?;

        let route_binding_id = self.route_binding_id;

        Ok(Some(MeshPathPlan {
            namespace,
            join_mode,
            selected_peers,
            multipath_schedule: MeshMultipathSchedule {
                mode,
                route_binding_id,
                lanes,
                carrier_lane_bindings,
                active_lane_count,
                standby_lane_count,
                lane_admission_requested_active_lane_count: lane_requested,
                lane_admission_admitted_active_lane_count: lane_admitted,
                lane_admission_rejected_active_lane_count: lane_rejected,
                lane_admission_capacity_status: lane_capacity_status,
                active_weight_sum_pct,
                active_capacity_sum_pct,
                local_traffic_reserve_pct: local_reserve_pct,
                transit_capacity_budget_pct: transit_budget_pct,
                demand_policy,
                demand_policy_source,
                demand_requested_active_lane_count: demand_requested,
                demand_planned_active_lane_count: demand_planned,
                demand_admitted_lane_capacity_pct: demand_admitted_capacity_pct,
                demand_unmet_lane_count: demand_unmet,
                demand_status,
                demand_rebuild_recommended,
                fairness_policy,
                execution_status,
                transit_payload_policy,
                planner_rebuild_reason,
            },
            explain: ordered_explain(self.explain)?,
        }))
    }
}

fn ordered_selected_peers(
    selected_peers: BTreeMap<usize, MeshPeerState>,
) -> Result<Vec<MeshPeerState>, String> {
    let mut ordered = Vec::with_capacity(selected_peers.len());
    for (expected_index, (index, peer)) in selected_peers.into_iter().enumerate() {
        if index != expected_index {
            return Err("transit plan snapshot selected peer index gap".to_string());
        }
        ordered.push(peer);
    }
    Ok(ordered)
}

fn ordered_carrier_bindings(
    carrier_bindings: BTreeMap<usize, MeshCarrierLaneBinding>,
) -> Result<Vec<MeshCarrierLaneBinding>, String> {
    let mut ordered = Vec::with_capacity(carrier_bindings.len());
    for (_index, binding) in carrier_bindings {
        ordered.push(binding);
    }
    ordered.sort_by_key(|binding| binding.lane_id);
    Ok(ordered)
}

fn ordered_explain(explain: BTreeMap<usize, String>) -> Result<Vec<String>, String> {
    let mut ordered = Vec::with_capacity(explain.len());
    for (expected_index, (index, value)) in explain.into_iter().enumerate() {
        if index != expected_index {
            return Err("transit plan snapshot explain index gap".to_string());
        }
        ordered.push(value);
    }
    Ok(ordered)
}
