use std::collections::BTreeSet;

use crate::multipath_model::{
    MeshCarrierLaneBinding, MeshMultipathLaneRole, MeshMultipathSchedule, MeshRouteBindingId,
};

const FLOW_FAIRNESS_POLICY: &str = "weighted_round_robin_v1";
const FLOW_TRANSIT_PAYLOAD_POLICY: &str = "sealed_opaque_only";
const FLOW_ACTION_ASSIGNED: &str = "assigned";
const FLOW_ACTION_FAIL_CLOSED: &str = "fail_closed";
const REBUILD_REASON_NONE: &str = "none";
const REBUILD_REASON_DEMAND: &str = "demand_rebuild_recommended";
const REBUILD_REASON_ACTIVE_LANES_BELOW_PLAN: &str = "active_lanes_below_plan";
const REBUILD_REASON_CAPACITY_PRESSURE: &str = "capacity_pressure";
const REBUILD_REASON_CAPACITY_OVERFLOW: &str = "capacity_overflow";

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

fn flow_plan(
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

struct ActiveBindingSummary {
    active_binding_count: usize,
    total_capacity_weight_pct: u16,
}

enum ActiveBindingScan {
    Ready(ActiveBindingSummary),
    Unsorted,
    FailClosed {
        reason: &'static str,
        active_binding_count: usize,
    },
}

fn scan_sorted_active_bindings(
    schedule: &MeshMultipathSchedule,
    route_binding_id: &MeshRouteBindingId,
) -> ActiveBindingScan {
    let mut active_binding_count = 0usize;
    let mut last_lane_id = None;
    let mut active_bindings_sorted = true;
    let mut duplicate_lane_id = false;
    let mut route_binding_mismatch = false;
    let mut total_capacity_weight_pct = 0u32;
    let mut saw_positive_capacity = false;
    let mut capacity_overflow = false;

    for binding in schedule
        .carrier_lane_bindings
        .iter()
        .filter(|binding| binding.role == MeshMultipathLaneRole::Active)
    {
        active_binding_count = active_binding_count.saturating_add(1);
        route_binding_mismatch |= &binding.route_binding_id != route_binding_id;
        if let Some(last_lane_id) = last_lane_id {
            if binding.lane_id < last_lane_id {
                active_bindings_sorted = false;
            } else if binding.lane_id == last_lane_id {
                duplicate_lane_id = true;
            }
        }
        last_lane_id = Some(binding.lane_id);

        let weight = u32::from(binding.capacity_weight_pct);
        saw_positive_capacity |= weight > 0;
        match total_capacity_weight_pct.checked_add(weight) {
            Some(total) => total_capacity_weight_pct = total,
            None => capacity_overflow = true,
        }
    }

    if active_binding_count == 0 {
        return ActiveBindingScan::FailClosed {
            reason: "active_binding_missing",
            active_binding_count,
        };
    }
    if route_binding_mismatch {
        return ActiveBindingScan::FailClosed {
            reason: "route_binding_mismatch",
            active_binding_count,
        };
    }
    if !active_bindings_sorted {
        return ActiveBindingScan::Unsorted;
    }
    if duplicate_lane_id {
        return ActiveBindingScan::FailClosed {
            reason: "duplicate_active_lane",
            active_binding_count,
        };
    }
    if capacity_overflow || total_capacity_weight_pct > u32::from(u16::MAX) {
        return ActiveBindingScan::FailClosed {
            reason: REBUILD_REASON_CAPACITY_OVERFLOW,
            active_binding_count,
        };
    }
    if !saw_positive_capacity || total_capacity_weight_pct == 0 {
        return ActiveBindingScan::FailClosed {
            reason: "active_binding_capacity_missing",
            active_binding_count,
        };
    }
    ActiveBindingScan::Ready(ActiveBindingSummary {
        active_binding_count,
        total_capacity_weight_pct: total_capacity_weight_pct as u16,
    })
}

fn plan_multipath_flow_slow_sorted(
    schedule: &MeshMultipathSchedule,
    flow_key: MeshMultipathFlowKey,
    route_binding_id: &MeshRouteBindingId,
) -> MeshMultipathFlowPlan {
    let active_bindings = active_carrier_bindings(schedule);
    let active_binding_count = active_bindings.len();
    if active_bindings.is_empty() {
        return flow_plan(
            MeshMultipathFlowAction::FailClosed,
            "active_binding_missing",
            None,
            active_binding_count,
            0,
            schedule,
        );
    }
    if active_bindings
        .iter()
        .any(|binding| &binding.route_binding_id != route_binding_id)
    {
        return flow_plan(
            MeshMultipathFlowAction::FailClosed,
            "route_binding_mismatch",
            None,
            active_binding_count,
            0,
            schedule,
        );
    }
    if duplicate_lane_id(&active_bindings) {
        return flow_plan(
            MeshMultipathFlowAction::FailClosed,
            "duplicate_active_lane",
            None,
            active_binding_count,
            0,
            schedule,
        );
    }

    let total_capacity_weight_pct = match total_active_capacity_weight_pct(&active_bindings) {
        Ok(total) => total,
        Err(reason) => {
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
    if total_capacity_weight_pct > u16::from(schedule.transit_capacity_budget_pct) {
        return flow_plan(
            MeshMultipathFlowAction::FailClosed,
            "active_binding_capacity_over_budget",
            None,
            active_binding_count,
            total_capacity_weight_pct,
            schedule,
        );
    }

    let selected = select_weighted_binding(
        &active_bindings,
        flow_key.stable_hash,
        total_capacity_weight_pct,
    )
    .map(|binding| binding.lane_id);
    let Some(selected_lane_id) = selected else {
        return flow_plan(
            MeshMultipathFlowAction::FailClosed,
            "weighted_selection_no_match",
            None,
            active_binding_count,
            total_capacity_weight_pct,
            schedule,
        );
    };
    flow_plan(
        MeshMultipathFlowAction::Assigned,
        "active_carrier_binding_selected",
        Some(selected_lane_id),
        active_binding_count,
        total_capacity_weight_pct,
        schedule,
    )
}

fn active_carrier_bindings(schedule: &MeshMultipathSchedule) -> Vec<&MeshCarrierLaneBinding> {
    let mut bindings: Vec<&MeshCarrierLaneBinding> = schedule
        .carrier_lane_bindings
        .iter()
        .filter(|binding| binding.role == MeshMultipathLaneRole::Active)
        .collect();
    bindings.sort_by_key(|binding| binding.lane_id);
    bindings
}

fn duplicate_lane_id(bindings: &[&MeshCarrierLaneBinding]) -> bool {
    let mut seen = BTreeSet::new();
    bindings.iter().any(|binding| !seen.insert(binding.lane_id))
}

fn select_weighted_binding<'a>(
    bindings: &'a [&'a MeshCarrierLaneBinding],
    stable_hash: u64,
    total_capacity_weight_pct: u16,
) -> Option<&'a MeshCarrierLaneBinding> {
    let mut bucket = (stable_hash % u64::from(total_capacity_weight_pct)) as u16;
    for binding in bindings {
        let weight = u16::from(binding.capacity_weight_pct);
        if weight == 0 {
            continue;
        }
        if bucket < weight {
            return Some(*binding);
        }
        bucket = bucket.saturating_sub(weight);
    }
    None
}

fn select_weighted_lane_id_from_sorted_bindings(
    schedule: &MeshMultipathSchedule,
    stable_hash: u64,
    total_capacity_weight_pct: u16,
) -> Option<usize> {
    let mut bucket = (stable_hash % u64::from(total_capacity_weight_pct)) as u16;
    for binding in schedule
        .carrier_lane_bindings
        .iter()
        .filter(|binding| binding.role == MeshMultipathLaneRole::Active)
    {
        let weight = u16::from(binding.capacity_weight_pct);
        if weight == 0 {
            continue;
        }
        if bucket < weight {
            return Some(binding.lane_id);
        }
        bucket = bucket.saturating_sub(weight);
    }
    None
}

fn total_active_capacity_weight_pct(
    bindings: &[&MeshCarrierLaneBinding],
) -> Result<u16, &'static str> {
    let mut total: u32 = 0;
    let mut saw_positive = false;
    for binding in bindings {
        let weight = u32::from(binding.capacity_weight_pct);
        if weight > 0 {
            saw_positive = true;
        }
        total = total
            .checked_add(weight)
            .ok_or(REBUILD_REASON_CAPACITY_OVERFLOW)?;
    }
    if !saw_positive {
        return Err("active_binding_capacity_missing");
    }
    if total == 0 {
        return Err("active_binding_capacity_missing");
    }
    if total > u32::from(u16::MAX) {
        return Err(REBUILD_REASON_CAPACITY_OVERFLOW);
    }
    Ok(total as u16)
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
