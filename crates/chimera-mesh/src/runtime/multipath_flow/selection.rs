use std::collections::BTreeSet;

use crate::multipath_model::{
    MeshCarrierLaneBinding, MeshMultipathLaneRole, MeshMultipathSchedule, MeshRouteBindingId,
};

use super::{
    MeshMultipathFlowAction, MeshMultipathFlowKey, MeshMultipathFlowPlan,
    REBUILD_REASON_CAPACITY_OVERFLOW, flow_plan,
};

pub(super) struct ActiveBindingSummary {
    pub(super) active_binding_count: usize,
    pub(super) total_capacity_weight_pct: u16,
}

pub(super) enum ActiveBindingScan {
    Ready(ActiveBindingSummary),
    Unsorted,
    FailClosed {
        reason: &'static str,
        active_binding_count: usize,
    },
}

pub(super) fn scan_sorted_active_bindings(
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

pub(super) fn plan_multipath_flow_slow_sorted(
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

pub(super) fn select_weighted_lane_id_from_sorted_bindings(
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
