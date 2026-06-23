mod planner;

use crate::multipath_model::{MeshMultipathMode, MeshMultipathSchedule, MeshRouteBindingId};

use self::planner::{
    active_carrier_bindings, aggregate_plan, build_weighted_shards, duplicate_lane_id,
    local_reserve_is_safe, total_active_capacity_weight_pct,
};

const AGGREGATE_ACTION_ASSIGNED: &str = "assigned";
const AGGREGATE_ACTION_FAIL_CLOSED: &str = "fail_closed";
pub(super) const AGGREGATE_PRIVACY_POLICY: &str = "sealed_opaque_only";
pub(super) const AGGREGATE_FAIRNESS_POLICY: &str = "weighted_capacity_shards_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshMultipathAggregateAction {
    Assigned,
    FailClosed,
}

impl MeshMultipathAggregateAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Assigned => AGGREGATE_ACTION_ASSIGNED,
            Self::FailClosed => AGGREGATE_ACTION_FAIL_CLOSED,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MeshMultipathAggregateShard {
    pub lane_id: usize,
    pub route_binding_id: MeshRouteBindingId,
    pub byte_offset: u64,
    pub byte_len: u64,
    pub capacity_weight_pct: u8,
}

impl std::fmt::Debug for MeshMultipathAggregateShard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshMultipathAggregateShard")
            .field("lane_id", &"<opaque>")
            .field("route_binding_id", &self.route_binding_id)
            .field("byte_range", &"<redacted>")
            .field("capacity_weight_pct", &self.capacity_weight_pct)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MeshMultipathAggregatePlan {
    pub action: MeshMultipathAggregateAction,
    pub reason: String,
    pub object_bytes: u64,
    pub active_binding_count: usize,
    pub total_capacity_weight_pct: u16,
    pub local_traffic_reserve_pct: u8,
    pub transit_capacity_budget_pct: u8,
    pub rebuild_recommended: bool,
    pub rebuild_reason: String,
    pub fairness_policy: String,
    pub transit_payload_policy: String,
    pub shards: Vec<MeshMultipathAggregateShard>,
    pub explain: Vec<String>,
}

impl std::fmt::Debug for MeshMultipathAggregatePlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshMultipathAggregatePlan")
            .field("action", &self.action)
            .field("reason", &self.reason)
            .field("object_bytes", &"<redacted>")
            .field("active_binding_count", &self.active_binding_count)
            .field("total_capacity_weight_pct", &self.total_capacity_weight_pct)
            .field("local_traffic_reserve_pct", &self.local_traffic_reserve_pct)
            .field(
                "transit_capacity_budget_pct",
                &self.transit_capacity_budget_pct,
            )
            .field("rebuild_recommended", &self.rebuild_recommended)
            .field("rebuild_reason", &self.rebuild_reason)
            .field("fairness_policy", &self.fairness_policy)
            .field("transit_payload_policy", &self.transit_payload_policy)
            .field("shard_count", &self.shards.len())
            .finish()
    }
}

pub fn plan_multipath_aggregate_object(
    schedule: &MeshMultipathSchedule,
    object_bytes: u64,
) -> MeshMultipathAggregatePlan {
    if object_bytes == 0 {
        return aggregate_plan(
            MeshMultipathAggregateAction::FailClosed,
            "object_empty",
            object_bytes,
            0,
            0,
            schedule,
            Vec::new(),
        );
    }
    if schedule.mode != MeshMultipathMode::AggregateBuffered {
        return aggregate_plan(
            MeshMultipathAggregateAction::FailClosed,
            "aggregate_mode_required",
            object_bytes,
            0,
            0,
            schedule,
            Vec::new(),
        );
    }
    if schedule.transit_payload_policy != AGGREGATE_PRIVACY_POLICY {
        return aggregate_plan(
            MeshMultipathAggregateAction::FailClosed,
            "transit_payload_policy_not_opaque",
            object_bytes,
            0,
            0,
            schedule,
            Vec::new(),
        );
    }
    if !local_reserve_is_safe(schedule) {
        return aggregate_plan(
            MeshMultipathAggregateAction::FailClosed,
            "local_reserve_invalid",
            object_bytes,
            0,
            0,
            schedule,
            Vec::new(),
        );
    }
    let Some(route_binding_id) = schedule.route_binding_id.as_ref() else {
        return aggregate_plan(
            MeshMultipathAggregateAction::FailClosed,
            "route_binding_missing",
            object_bytes,
            0,
            0,
            schedule,
            Vec::new(),
        );
    };

    let active_bindings = active_carrier_bindings(schedule);
    let active_binding_count = active_bindings.len();
    if active_bindings.is_empty() {
        return aggregate_plan(
            MeshMultipathAggregateAction::FailClosed,
            "active_binding_missing",
            object_bytes,
            active_binding_count,
            0,
            schedule,
            Vec::new(),
        );
    }
    if active_bindings
        .iter()
        .any(|binding| &binding.route_binding_id != route_binding_id)
    {
        return aggregate_plan(
            MeshMultipathAggregateAction::FailClosed,
            "route_binding_mismatch",
            object_bytes,
            active_binding_count,
            0,
            schedule,
            Vec::new(),
        );
    }
    if duplicate_lane_id(&active_bindings) {
        return aggregate_plan(
            MeshMultipathAggregateAction::FailClosed,
            "duplicate_active_lane",
            object_bytes,
            active_binding_count,
            0,
            schedule,
            Vec::new(),
        );
    }

    let total_capacity_weight_pct = match total_active_capacity_weight_pct(&active_bindings) {
        Ok(total) => total,
        Err(reason) => {
            return aggregate_plan(
                MeshMultipathAggregateAction::FailClosed,
                reason,
                object_bytes,
                active_binding_count,
                0,
                schedule,
                Vec::new(),
            );
        }
    };
    if total_capacity_weight_pct > u16::from(schedule.transit_capacity_budget_pct) {
        return aggregate_plan(
            MeshMultipathAggregateAction::FailClosed,
            "active_binding_capacity_over_budget",
            object_bytes,
            active_binding_count,
            total_capacity_weight_pct,
            schedule,
            Vec::new(),
        );
    }

    let shards = build_weighted_shards(
        &active_bindings,
        route_binding_id,
        object_bytes,
        total_capacity_weight_pct,
    );
    if shards.is_empty() {
        return aggregate_plan(
            MeshMultipathAggregateAction::FailClosed,
            "aggregate_shards_empty",
            object_bytes,
            active_binding_count,
            total_capacity_weight_pct,
            schedule,
            Vec::new(),
        );
    }

    aggregate_plan(
        MeshMultipathAggregateAction::Assigned,
        "aggregate_object_sharded",
        object_bytes,
        active_binding_count,
        total_capacity_weight_pct,
        schedule,
        shards,
    )
}
