pub(super) const ADMISSION_WITHIN_BUDGET: &str = "within_budget";
pub(super) const ADMISSION_AT_BUDGET: &str = "at_budget";
pub(super) const ADMISSION_OVER_BUDGET_TRUNCATED: &str = "over_budget_truncated";
pub(super) const MIN_ACTIVE_LANE_CAPACITY_PCT: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MultipathLaneAdmission {
    pub(super) requested_active_lane_count: usize,
    pub(super) admitted_active_lane_count: usize,
    pub(super) rejected_active_lane_count: usize,
    pub(super) capacity_status: &'static str,
}

pub(super) fn evaluate_lane_admission(
    requested_active_lane_count: usize,
    max_active_lane_count: usize,
) -> MultipathLaneAdmission {
    let admitted_active_lane_count = requested_active_lane_count.min(max_active_lane_count);
    let rejected_active_lane_count =
        requested_active_lane_count.saturating_sub(admitted_active_lane_count);
    let capacity_status = if rejected_active_lane_count > 0 {
        ADMISSION_OVER_BUDGET_TRUNCATED
    } else if admitted_active_lane_count > 0 && admitted_active_lane_count == max_active_lane_count
    {
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

pub(super) fn max_active_lane_count_for_capacity(transit_capacity_budget_pct: u8) -> usize {
    usize::from(transit_capacity_budget_pct / MIN_ACTIVE_LANE_CAPACITY_PCT)
}

#[cfg(test)]
mod tests {
    use super::{
        ADMISSION_AT_BUDGET, ADMISSION_OVER_BUDGET_TRUNCATED, ADMISSION_WITHIN_BUDGET,
        evaluate_lane_admission, max_active_lane_count_for_capacity,
    };

    #[test]
    fn aggregate_reports_truncated_lanes_when_request_exceeds_active_lane_capacity() {
        let max_active_lanes = max_active_lane_count_for_capacity(90);
        let admission = evaluate_lane_admission(95, max_active_lanes);

        assert_eq!(admission.requested_active_lane_count, 95);
        assert_eq!(admission.admitted_active_lane_count, 90);
        assert_eq!(admission.rejected_active_lane_count, 5);
        assert_eq!(admission.capacity_status, ADMISSION_OVER_BUDGET_TRUNCATED);
    }

    #[test]
    fn aggregate_reports_at_active_lane_capacity_without_drops() {
        let max_active_lanes = max_active_lane_count_for_capacity(90);
        let admission = evaluate_lane_admission(90, max_active_lanes);

        assert_eq!(admission.requested_active_lane_count, 90);
        assert_eq!(admission.admitted_active_lane_count, 90);
        assert_eq!(admission.rejected_active_lane_count, 0);
        assert_eq!(admission.capacity_status, ADMISSION_AT_BUDGET);
    }

    #[test]
    fn flow_shard_reports_within_budget() {
        let max_active_lanes = max_active_lane_count_for_capacity(90);
        let admission = evaluate_lane_admission(2, max_active_lanes);

        assert_eq!(admission.requested_active_lane_count, 2);
        assert_eq!(admission.admitted_active_lane_count, 2);
        assert_eq!(admission.rejected_active_lane_count, 0);
        assert_eq!(admission.capacity_status, ADMISSION_WITHIN_BUDGET);
    }

    #[test]
    fn active_lane_count_capacity_is_derived_from_minimum_nonzero_capacity_quantum() {
        assert_eq!(max_active_lane_count_for_capacity(90), 90);
        assert_eq!(max_active_lane_count_for_capacity(1), 1);
        assert_eq!(max_active_lane_count_for_capacity(0), 0);
    }
}
