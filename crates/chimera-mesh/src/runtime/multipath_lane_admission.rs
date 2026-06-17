use super::MeshMultipathMode;

pub(super) const ADMISSION_WITHIN_BUDGET: &str = "within_budget";
pub(super) const ADMISSION_AT_BUDGET: &str = "at_budget";
pub(super) const ADMISSION_OVER_BUDGET_TRUNCATED: &str = "over_budget_truncated";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MultipathLaneAdmission {
    pub(super) requested_active_lane_count: usize,
    pub(super) admitted_active_lane_count: usize,
    pub(super) rejected_active_lane_count: usize,
    pub(super) capacity_status: &'static str,
}

pub(super) fn evaluate_lane_admission(
    mode: &MeshMultipathMode,
    selected_peer_count: usize,
    transit_capacity_budget_pct: u8,
) -> MultipathLaneAdmission {
    let requested_active_lane_count = requested_active_lanes(mode, selected_peer_count);
    let capacity_limit = usize::from(transit_capacity_budget_pct);
    let admitted_active_lane_count = requested_active_lane_count.min(capacity_limit);
    let rejected_active_lane_count =
        requested_active_lane_count.saturating_sub(admitted_active_lane_count);
    let capacity_status = if rejected_active_lane_count > 0 {
        ADMISSION_OVER_BUDGET_TRUNCATED
    } else if admitted_active_lane_count > 0 && admitted_active_lane_count == capacity_limit {
        ADMISSION_AT_BUDGET
    } else {
        ADMISSION_WITHIN_BUDGET
    };

    MultipathLaneAdmission {
        requested_active_lane_count,
        admitted_active_lane_count,
        rejected_active_lane_count,
        capacity_status,
    }
}

fn requested_active_lanes(mode: &MeshMultipathMode, selected_peer_count: usize) -> usize {
    match mode {
        MeshMultipathMode::Off => selected_peer_count.min(1),
        MeshMultipathMode::StandbyOnly => selected_peer_count.min(1),
        MeshMultipathMode::FlowShard => selected_peer_count.min(2),
        MeshMultipathMode::AggregateBuffered => selected_peer_count,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ADMISSION_AT_BUDGET, ADMISSION_OVER_BUDGET_TRUNCATED, ADMISSION_WITHIN_BUDGET,
        evaluate_lane_admission,
    };
    use crate::MeshMultipathMode;

    #[test]
    fn aggregate_reports_truncated_lanes_when_request_exceeds_budget() {
        let admission = evaluate_lane_admission(&MeshMultipathMode::AggregateBuffered, 95, 90);

        assert_eq!(admission.requested_active_lane_count, 95);
        assert_eq!(admission.admitted_active_lane_count, 90);
        assert_eq!(admission.rejected_active_lane_count, 5);
        assert_eq!(admission.capacity_status, ADMISSION_OVER_BUDGET_TRUNCATED);
    }

    #[test]
    fn aggregate_reports_at_budget_without_drops() {
        let admission = evaluate_lane_admission(&MeshMultipathMode::AggregateBuffered, 90, 90);

        assert_eq!(admission.requested_active_lane_count, 90);
        assert_eq!(admission.admitted_active_lane_count, 90);
        assert_eq!(admission.rejected_active_lane_count, 0);
        assert_eq!(admission.capacity_status, ADMISSION_AT_BUDGET);
    }

    #[test]
    fn flow_shard_reports_within_budget() {
        let admission = evaluate_lane_admission(&MeshMultipathMode::FlowShard, 12, 90);

        assert_eq!(admission.requested_active_lane_count, 2);
        assert_eq!(admission.admitted_active_lane_count, 2);
        assert_eq!(admission.rejected_active_lane_count, 0);
        assert_eq!(admission.capacity_status, ADMISSION_WITHIN_BUDGET);
    }
}
