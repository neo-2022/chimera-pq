use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshMultipathMode {
    Off,
    StandbyOnly,
    FlowShard,
    AggregateBuffered,
}

impl MeshMultipathMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::StandbyOnly => "standby_only",
            Self::FlowShard => "flow_shard",
            Self::AggregateBuffered => "aggregate_buffered",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshMultipathLaneRole {
    Active,
    Standby,
    Transit,
}

impl MeshMultipathLaneRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Standby => "standby",
            Self::Transit => "transit",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MeshMultipathLane {
    pub lane_id: usize,
    pub peer_node_id: String,
    pub role: MeshMultipathLaneRole,
    pub weight_pct: u8,
    pub capacity_weight_pct: u8,
}

impl std::fmt::Debug for MeshMultipathLane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshMultipathLane")
            .field("lane_id", &self.lane_id)
            .field("peer_node_id", &"<redacted>")
            .field("role", &self.role)
            .field("weight_pct", &self.weight_pct)
            .field("capacity_weight_pct", &self.capacity_weight_pct)
            .finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MeshRouteBindingId(NonZeroU64);

impl MeshRouteBindingId {
    pub fn new(value: u64) -> Result<Self, String> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or_else(|| "mesh route binding id must be nonzero".to_string())
    }

    pub fn get(&self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Debug for MeshRouteBindingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MeshRouteBindingId(<opaque>)")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MeshCarrierLaneBinding {
    pub route_binding_id: MeshRouteBindingId,
    pub lane_id: usize,
    pub peer_node_id: String,
    pub carrier_endpoint: String,
    pub role: MeshMultipathLaneRole,
    pub weight_pct: u8,
    pub capacity_weight_pct: u8,
}

impl std::fmt::Debug for MeshCarrierLaneBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshCarrierLaneBinding")
            .field("route_binding_id", &self.route_binding_id)
            .field("lane_id", &self.lane_id)
            .field("peer_node_id", &"<redacted>")
            .field("carrier_endpoint", &"<redacted>")
            .field("role", &self.role)
            .field("weight_pct", &self.weight_pct)
            .field("capacity_weight_pct", &self.capacity_weight_pct)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshMultipathSchedule {
    pub mode: MeshMultipathMode,
    pub route_binding_id: Option<MeshRouteBindingId>,
    pub lanes: Vec<MeshMultipathLane>,
    pub carrier_lane_bindings: Vec<MeshCarrierLaneBinding>,
    pub route_announcements: Vec<crate::route_announcement::RouteAnnouncement>,
    pub active_lane_count: usize,
    pub standby_lane_count: usize,
    pub lane_admission_requested_active_lane_count: usize,
    pub lane_admission_admitted_active_lane_count: usize,
    pub lane_admission_rejected_active_lane_count: usize,
    pub lane_admission_capacity_status: String,
    pub active_weight_sum_pct: u16,
    pub active_capacity_sum_pct: u16,
    pub local_traffic_reserve_pct: u8,
    pub transit_capacity_budget_pct: u8,
    pub demand_policy: String,
    pub demand_policy_source: String,
    pub demand_requested_active_lane_count: usize,
    pub demand_planned_active_lane_count: usize,
    pub demand_admitted_lane_capacity_pct: u8,
    pub demand_unmet_lane_count: usize,
    pub demand_status: String,
    pub demand_rebuild_recommended: bool,
    pub fairness_policy: String,
    pub execution_status: String,
    pub transit_payload_policy: String,
    pub planner_rebuild_reason: String,
}
