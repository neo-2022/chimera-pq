use std::collections::BTreeSet;

use crate::multipath_model::{
    MeshCarrierLaneBinding, MeshMultipathLaneRole, MeshMultipathSchedule, MeshRouteBindingId,
};

use super::{
    AGGREGATE_FAIRNESS_POLICY, AGGREGATE_PRIVACY_POLICY, MeshMultipathAggregateAction,
    MeshMultipathAggregatePlan, MeshMultipathAggregateShard,
};

pub(super) fn aggregate_plan(
    action: MeshMultipathAggregateAction,
    reason: &str,
    object_bytes: u64,
    active_binding_count: usize,
    total_capacity_weight_pct: u16,
    schedule: &MeshMultipathSchedule,
    shards: Vec<MeshMultipathAggregateShard>,
) -> MeshMultipathAggregatePlan {
    let rebuild_reason = rebuild_reason(schedule, active_binding_count).to_string();
    let rebuild_recommended = rebuild_reason != "none";
    let explain = vec![
        format!("multipath_aggregate_action={}", action.as_str()),
        format!("multipath_aggregate_reason={reason}"),
        format!("multipath_aggregate_privacy={AGGREGATE_PRIVACY_POLICY}"),
        format!("multipath_aggregate_fairness_policy={AGGREGATE_FAIRNESS_POLICY}"),
        format!("multipath_aggregate_active_bindings={active_binding_count}"),
        format!("multipath_aggregate_shards={}", shards.len()),
        format!("multipath_aggregate_total_capacity_weight_pct={total_capacity_weight_pct}"),
        format!(
            "multipath_aggregate_local_reserve_pct={}",
            schedule.local_traffic_reserve_pct
        ),
        format!(
            "multipath_aggregate_transit_capacity_budget_pct={}",
            schedule.transit_capacity_budget_pct
        ),
        format!("multipath_aggregate_rebuild_recommended={rebuild_recommended}"),
        format!("multipath_aggregate_rebuild_reason={rebuild_reason}"),
    ];

    MeshMultipathAggregatePlan {
        action,
        reason: reason.to_string(),
        object_bytes,
        active_binding_count,
        total_capacity_weight_pct,
        local_traffic_reserve_pct: schedule.local_traffic_reserve_pct,
        transit_capacity_budget_pct: schedule.transit_capacity_budget_pct,
        rebuild_recommended,
        rebuild_reason,
        fairness_policy: AGGREGATE_FAIRNESS_POLICY.to_string(),
        transit_payload_policy: AGGREGATE_PRIVACY_POLICY.to_string(),
        shards,
        explain,
    }
}

pub(super) fn active_carrier_bindings(
    schedule: &MeshMultipathSchedule,
) -> Vec<&MeshCarrierLaneBinding> {
    let mut bindings: Vec<&MeshCarrierLaneBinding> = schedule
        .carrier_lane_bindings
        .iter()
        .filter(|binding| binding.role == MeshMultipathLaneRole::Active)
        .collect();
    bindings.sort_by_key(|binding| binding.lane_id);
    bindings
}

pub(super) fn duplicate_lane_id(bindings: &[&MeshCarrierLaneBinding]) -> bool {
    let mut seen = BTreeSet::new();
    bindings.iter().any(|binding| !seen.insert(binding.lane_id))
}

pub(super) fn total_active_capacity_weight_pct(
    bindings: &[&MeshCarrierLaneBinding],
) -> Result<u16, &'static str> {
    let mut total: u32 = 0;
    let mut saw_positive = false;
    for binding in bindings {
        let weight = u32::from(binding.capacity_weight_pct);
        if weight > 0 {
            saw_positive = true;
        }
        total = total.checked_add(weight).ok_or("capacity_overflow")?;
    }
    if !saw_positive || total == 0 {
        return Err("active_binding_capacity_missing");
    }
    if total > u32::from(u16::MAX) {
        return Err("capacity_overflow");
    }
    Ok(total as u16)
}

pub(super) fn build_weighted_shards(
    bindings: &[&MeshCarrierLaneBinding],
    route_binding_id: &MeshRouteBindingId,
    object_bytes: u64,
    total_capacity_weight_pct: u16,
) -> Vec<MeshMultipathAggregateShard> {
    let mut slices = bindings
        .iter()
        .map(|binding| {
            let numerator = u128::from(object_bytes) * u128::from(binding.capacity_weight_pct);
            let denominator = u128::from(total_capacity_weight_pct);
            let base = (numerator / denominator) as u64;
            let remainder = numerator % denominator;
            (*binding, base, remainder)
        })
        .collect::<Vec<_>>();

    let assigned = slices
        .iter()
        .fold(0_u64, |total, (_, base, _)| total.saturating_add(*base));
    let mut remaining = object_bytes.saturating_sub(assigned);
    slices.sort_by(|left, right| {
        right
            .2
            .cmp(&left.2)
            .then_with(|| left.0.lane_id.cmp(&right.0.lane_id))
    });
    for (_, base, _) in &mut slices {
        if remaining == 0 {
            break;
        }
        *base = base.saturating_add(1);
        remaining = remaining.saturating_sub(1);
    }
    slices.sort_by_key(|(binding, _, _)| binding.lane_id);

    let mut byte_offset = 0_u64;
    let mut shards = Vec::with_capacity(slices.len());
    for (binding, byte_len, _) in slices {
        if byte_len == 0 {
            continue;
        }
        shards.push(MeshMultipathAggregateShard {
            lane_id: binding.lane_id,
            route_binding_id: route_binding_id.clone(),
            byte_offset,
            byte_len,
            capacity_weight_pct: binding.capacity_weight_pct,
        });
        byte_offset = byte_offset.saturating_add(byte_len);
    }
    shards
}

fn rebuild_reason(schedule: &MeshMultipathSchedule, active_binding_count: usize) -> &'static str {
    if schedule.demand_rebuild_recommended {
        return "demand_rebuild_recommended";
    }
    if active_binding_count < schedule.demand_planned_active_lane_count
        || active_binding_count < schedule.lane_admission_admitted_active_lane_count
    {
        return "active_lanes_below_plan";
    }
    if schedule.demand_unmet_lane_count > 0
        || schedule.lane_admission_rejected_active_lane_count > 0
        || schedule.active_capacity_sum_pct > u16::from(schedule.transit_capacity_budget_pct)
    {
        return "capacity_pressure";
    }
    "none"
}

pub(super) fn local_reserve_is_safe(schedule: &MeshMultipathSchedule) -> bool {
    schedule.local_traffic_reserve_pct > 0
        && schedule
            .local_traffic_reserve_pct
            .saturating_add(schedule.transit_capacity_budget_pct)
            <= 100
}
