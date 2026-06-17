use crate::policy::MultipathDemand;

use super::MeshMultipathMode;

pub(super) const DEMAND_POLICY_SOURCE_DEFAULT: &str = "default_policy";
pub(super) const DEMAND_POLICY_SOURCE_CONTROL: &str = "control_policy";
pub(super) const DEMAND_STATUS_WITHIN_BUDGET: &str = "within_budget";
pub(super) const DEMAND_STATUS_BUDGET_SATURATED: &str = "budget_saturated";
pub(super) const DEMAND_STATUS_NO_ACTIVE_LANES: &str = "no_active_lanes";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MultipathDemandPlan {
    pub(super) demand: MultipathDemand,
    pub(super) policy_source: &'static str,
    pub(super) requested_active_lane_count: usize,
    pub(super) planned_active_lane_count: usize,
    pub(super) admitted_lane_capacity_pct: u8,
    pub(super) unmet_lane_count: usize,
    pub(super) status: &'static str,
    pub(super) rebuild_recommended: bool,
}

impl MultipathDemandPlan {
    pub(super) fn demand_label(self) -> &'static str {
        self.demand.as_str()
    }
}

pub(super) fn plan_multipath_demand(
    mode: &MeshMultipathMode,
    demand: Option<MultipathDemand>,
    selected_peer_count: usize,
    transit_capacity_budget_pct: u8,
) -> MultipathDemandPlan {
    let policy_source = if demand.is_some() {
        DEMAND_POLICY_SOURCE_CONTROL
    } else {
        DEMAND_POLICY_SOURCE_DEFAULT
    };
    let demand = demand.unwrap_or(default_demand_for_mode(mode));
    let requested_active_lane_count =
        requested_active_lanes_for_demand(mode, demand, selected_peer_count);
    let capacity_limit = usize::from(transit_capacity_budget_pct);
    let planned_active_lane_count = requested_active_lane_count.min(capacity_limit);
    let unmet_lane_count = requested_active_lane_count.saturating_sub(planned_active_lane_count);
    let admitted_lane_capacity_pct =
        admitted_capacity_pct(planned_active_lane_count, transit_capacity_budget_pct);
    let status = if planned_active_lane_count == 0 {
        DEMAND_STATUS_NO_ACTIVE_LANES
    } else if unmet_lane_count > 0 {
        DEMAND_STATUS_BUDGET_SATURATED
    } else {
        DEMAND_STATUS_WITHIN_BUDGET
    };
    let rebuild_recommended =
        demand != default_demand_for_mode(mode) && requested_active_lane_count > 0;

    MultipathDemandPlan {
        demand,
        policy_source,
        requested_active_lane_count,
        planned_active_lane_count,
        admitted_lane_capacity_pct,
        unmet_lane_count,
        status,
        rebuild_recommended,
    }
}

fn default_demand_for_mode(mode: &MeshMultipathMode) -> MultipathDemand {
    match mode {
        MeshMultipathMode::Off | MeshMultipathMode::StandbyOnly => MultipathDemand::Normal,
        MeshMultipathMode::FlowShard => MultipathDemand::High,
        MeshMultipathMode::AggregateBuffered => MultipathDemand::Bulk,
    }
}

fn requested_active_lanes_for_demand(
    mode: &MeshMultipathMode,
    demand: MultipathDemand,
    selected_peer_count: usize,
) -> usize {
    if selected_peer_count == 0 {
        return 0;
    }

    let requested = match mode {
        MeshMultipathMode::Off | MeshMultipathMode::StandbyOnly => 1,
        MeshMultipathMode::FlowShard => match demand {
            MultipathDemand::Low | MultipathDemand::Normal => 1,
            MultipathDemand::High | MultipathDemand::Bulk => 2,
        },
        MeshMultipathMode::AggregateBuffered => match demand {
            MultipathDemand::Low => 1,
            MultipathDemand::Normal => selected_peer_count.min(2),
            MultipathDemand::High => selected_peer_count.min(4),
            MultipathDemand::Bulk => selected_peer_count,
        },
    };

    requested.min(selected_peer_count)
}

fn admitted_capacity_pct(planned_active_lane_count: usize, transit_capacity_budget_pct: u8) -> u8 {
    if planned_active_lane_count == 0 {
        0
    } else {
        transit_capacity_budget_pct
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_low_demand_requests_single_lane() {
        let plan = plan_multipath_demand(
            &MeshMultipathMode::AggregateBuffered,
            Some(MultipathDemand::Low),
            6,
            90,
        );

        assert_eq!(plan.requested_active_lane_count, 1);
        assert_eq!(plan.planned_active_lane_count, 1);
        assert_eq!(plan.admitted_lane_capacity_pct, 90);
        assert_eq!(plan.status, DEMAND_STATUS_WITHIN_BUDGET);
    }

    #[test]
    fn aggregate_bulk_demand_is_capped_by_transit_budget() {
        let plan = plan_multipath_demand(
            &MeshMultipathMode::AggregateBuffered,
            Some(MultipathDemand::Bulk),
            95,
            90,
        );

        assert_eq!(plan.requested_active_lane_count, 95);
        assert_eq!(plan.planned_active_lane_count, 90);
        assert_eq!(plan.unmet_lane_count, 5);
        assert_eq!(plan.status, DEMAND_STATUS_BUDGET_SATURATED);
    }
}
