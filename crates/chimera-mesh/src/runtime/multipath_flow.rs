use crate::multipath_model::MeshMultipathSchedule;

#[path = "multipath_flow/selection.rs"]
mod selection;
use selection::{
    ActiveBindingScan, plan_multipath_flow_slow_sorted, scan_sorted_active_bindings,
    select_weighted_lane_id_from_sorted_bindings,
};

const FLOW_FAIRNESS_POLICY: &str = "weighted_round_robin_v1";
const FLOW_TRANSIT_PAYLOAD_POLICY: &str = "sealed_opaque_only";
const FLOW_ACTION_ASSIGNED: &str = "assigned";
const FLOW_ACTION_FAIL_CLOSED: &str = "fail_closed";
const REBUILD_REASON_NONE: &str = "none";
const REBUILD_REASON_DEMAND: &str = "demand_rebuild_recommended";
const REBUILD_REASON_ACTIVE_LANES_BELOW_PLAN: &str = "active_lanes_below_plan";
const REBUILD_REASON_CAPACITY_PRESSURE: &str = "capacity_pressure";
pub(super) const REBUILD_REASON_CAPACITY_OVERFLOW: &str = "capacity_overflow";

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MeshMultipathFlowKey {
    stable_hash: u64,
}

impl MeshMultipathFlowKey {
    pub fn from_opaque_flow_id(flow_id: &str) -> Result<Self, String> {
        let flow_id = flow_id.trim();
        if flow_id.is_empty() {
            return Err("multipath flow id is empty".to_string());
        }
        if flow_id.len() > 256 {
            return Err("multipath flow id is too long".to_string());
        }
        if flow_id.contains('\n') || flow_id.contains('\r') || flow_id.contains('\t') {
            return Err("multipath flow id contains control whitespace".to_string());
        }
        Self::from_opaque_flow_bytes(flow_id.as_bytes())
    }

    pub fn from_opaque_flow_bytes(flow_bytes: &[u8]) -> Result<Self, String> {
        if flow_bytes.is_empty() {
            return Err("multipath flow bytes are empty".to_string());
        }
        Ok(Self {
            stable_hash: stable_hash(flow_bytes),
        })
    }

    pub fn from_stable_hash(stable_hash: u64) -> Self {
        Self { stable_hash }
    }

    pub fn select_slot_index(self, candidate_count: usize) -> Result<usize, String> {
        if candidate_count == 0 {
            return Err("multipath flow has no candidates".to_string());
        }
        let candidate_count = u64::try_from(candidate_count)
            .map_err(|_| "multipath candidate count overflow".to_string())?;
        Ok((self.stable_hash % candidate_count) as usize)
    }
}

impl std::fmt::Debug for MeshMultipathFlowKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MeshMultipathFlowKey(<opaque>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshMultipathFlowAction {
    Assigned,
    FailClosed,
}

impl MeshMultipathFlowAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Assigned => FLOW_ACTION_ASSIGNED,
            Self::FailClosed => FLOW_ACTION_FAIL_CLOSED,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MeshMultipathFlowPlan {
    pub action: MeshMultipathFlowAction,
    pub reason: String,
    pub selected_lane_id: Option<usize>,
    pub active_binding_count: usize,
    pub total_capacity_weight_pct: u16,
    pub route_binding_configured: bool,
    pub rebuild_recommended: bool,
    pub rebuild_reason: String,
    pub fairness_policy: String,
    pub transit_payload_policy: String,
    pub explain: Vec<String>,
}

impl std::fmt::Debug for MeshMultipathFlowPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshMultipathFlowPlan")
            .field("action", &self.action)
            .field("reason", &self.reason)
            .field(
                "selected_lane_id",
                &self.selected_lane_id.map(|_| "<redacted>"),
            )
            .field("active_binding_count", &self.active_binding_count)
            .field("total_capacity_weight_pct", &self.total_capacity_weight_pct)
            .field("route_binding_configured", &self.route_binding_configured)
            .field("rebuild_recommended", &self.rebuild_recommended)
            .field("rebuild_reason", &self.rebuild_reason)
            .field("fairness_policy", &self.fairness_policy)
            .field("transit_payload_policy", &self.transit_payload_policy)
            .finish()
    }
}

pub fn plan_multipath_flow(
    schedule: &MeshMultipathSchedule,
    flow_key: MeshMultipathFlowKey,
) -> MeshMultipathFlowPlan {
    if schedule.transit_payload_policy != FLOW_TRANSIT_PAYLOAD_POLICY {
        return flow_plan(
            MeshMultipathFlowAction::FailClosed,
            "transit_payload_policy_not_opaque",
            None,
            0,
            0,
            schedule,
        );
    }
    if !local_reserve_is_safe(schedule) {
        return flow_plan(
            MeshMultipathFlowAction::FailClosed,
            "local_reserve_invalid",
            None,
            0,
            0,
            schedule,
        );
    }
    let Some(route_binding_id) = schedule.route_binding_id.as_ref() else {
        return flow_plan(
            MeshMultipathFlowAction::FailClosed,
            "route_binding_missing",
            None,
            0,
            0,
            schedule,
        );
    };

    let active = match scan_sorted_active_bindings(schedule, route_binding_id) {
        ActiveBindingScan::Ready(summary) => summary,
        ActiveBindingScan::Unsorted => {
            return plan_multipath_flow_slow_sorted(schedule, flow_key, route_binding_id);
        }
        ActiveBindingScan::FailClosed {
            reason,
            active_binding_count,
        } => {
            return flow_plan(
                MeshMultipathFlowAction::FailClosed,
                reason,
                None,
                active_binding_count,
                0,
                schedule,
            );
        }
    };
    if active.total_capacity_weight_pct > u16::from(schedule.transit_capacity_budget_pct) {
        return flow_plan(
            MeshMultipathFlowAction::FailClosed,
            "active_binding_capacity_over_budget",
            None,
            active.active_binding_count,
            active.total_capacity_weight_pct,
            schedule,
        );
    }

    let selected_lane_id = select_weighted_lane_id_from_sorted_bindings(
        schedule,
        flow_key.stable_hash,
        active.total_capacity_weight_pct,
    );
    let Some(selected_lane_id) = selected_lane_id else {
        return flow_plan(
            MeshMultipathFlowAction::FailClosed,
            "weighted_selection_no_match",
            None,
            active.active_binding_count,
            active.total_capacity_weight_pct,
            schedule,
        );
    };
    flow_plan(
        MeshMultipathFlowAction::Assigned,
        "active_carrier_binding_selected",
        Some(selected_lane_id),
        active.active_binding_count,
        active.total_capacity_weight_pct,
        schedule,
    )
}

pub(super) fn flow_plan(
    action: MeshMultipathFlowAction,
    reason: &str,
    selected_lane_id: Option<usize>,
    active_binding_count: usize,
    total_capacity_weight_pct: u16,
    schedule: &MeshMultipathSchedule,
) -> MeshMultipathFlowPlan {
    let rebuild_reason = rebuild_reason(schedule, active_binding_count).to_string();
    let rebuild_recommended = rebuild_reason != REBUILD_REASON_NONE;
    let selected_lane_status = if selected_lane_id.is_some() {
        "active"
    } else {
        "none"
    };
    let explain = vec![
        format!("multipath_flow_action={}", action.as_str()),
        format!("multipath_flow_reason={reason}"),
        format!("multipath_flow_selected_lane={selected_lane_status}"),
        format!("multipath_flow_active_bindings={active_binding_count}"),
        format!("multipath_flow_total_capacity_weight_pct={total_capacity_weight_pct}"),
        format!(
            "multipath_flow_route_binding_configured={}",
            schedule.route_binding_id.is_some()
        ),
        format!("multipath_flow_rebuild_recommended={rebuild_recommended}"),
        format!("multipath_flow_rebuild_reason={rebuild_reason}"),
        format!("multipath_flow_fairness_policy={FLOW_FAIRNESS_POLICY}"),
        format!("multipath_flow_privacy={FLOW_TRANSIT_PAYLOAD_POLICY}"),
    ];

    MeshMultipathFlowPlan {
        action,
        reason: reason.to_string(),
        selected_lane_id,
        active_binding_count,
        total_capacity_weight_pct,
        route_binding_configured: schedule.route_binding_id.is_some(),
        rebuild_recommended,
        rebuild_reason,
        fairness_policy: FLOW_FAIRNESS_POLICY.to_string(),
        transit_payload_policy: FLOW_TRANSIT_PAYLOAD_POLICY.to_string(),
        explain,
    }
}

fn rebuild_reason(schedule: &MeshMultipathSchedule, active_binding_count: usize) -> &'static str {
    if schedule.demand_rebuild_recommended {
        return REBUILD_REASON_DEMAND;
    }
    if active_binding_count < schedule.demand_planned_active_lane_count
        || active_binding_count < schedule.lane_admission_admitted_active_lane_count
    {
        return REBUILD_REASON_ACTIVE_LANES_BELOW_PLAN;
    }
    if schedule.demand_unmet_lane_count > 0
        || schedule.lane_admission_rejected_active_lane_count > 0
        || schedule.active_capacity_sum_pct > u16::from(schedule.transit_capacity_budget_pct)
    {
        return REBUILD_REASON_CAPACITY_PRESSURE;
    }
    REBUILD_REASON_NONE
}

fn local_reserve_is_safe(schedule: &MeshMultipathSchedule) -> bool {
    schedule.local_traffic_reserve_pct > 0
        && schedule
            .local_traffic_reserve_pct
            .saturating_add(schedule.transit_capacity_budget_pct)
            <= 100
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
#[path = "multipath_flow_tests.rs"]
mod tests;
