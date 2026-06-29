use crate::policy::MeshPeerTablePolicy;

use super::MeshPeerTableEnforcementReport;
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TableConsistencyStatus {
    pub(super) policy_consistency_all_true: bool,
    pub(super) enforcement_invariants_all_true: bool,
    pub(super) runtime_consistency_all_true: bool,
    pub(super) runtime_consistency_gate: String,
    pub(super) preemptive_degraded_path: bool,
    pub(super) preemptive_degraded_reason: String,
}

impl TableConsistencyStatus {
    pub(super) fn consistency_summary(&self) -> String {
        let mut out = String::with_capacity(self.runtime_consistency_gate.len().saturating_add(16));
        let _ = write!(
            &mut out,
            "gate={};all_true={}",
            self.runtime_consistency_gate, self.runtime_consistency_all_true
        );
        out
    }

    pub(super) fn degraded_summary(&self) -> String {
        let mut out = String::with_capacity(
            self.preemptive_degraded_reason
                .len()
                .saturating_add(self.runtime_consistency_gate.len())
                .saturating_add(32),
        );
        let _ = write!(
            &mut out,
            "path={};reason={};gate={};all_true={}",
            self.preemptive_degraded_path,
            self.preemptive_degraded_reason,
            self.runtime_consistency_gate,
            self.runtime_consistency_all_true
        );
        out
    }
}

fn drop_components_sum(report: &MeshPeerTableEnforcementReport) -> usize {
    report
        .dropped_by_region_cap
        .saturating_add(report.dropped_by_global_cap)
}

fn drop_delta(report: &MeshPeerTableEnforcementReport) -> usize {
    report
        .total_peers_before
        .saturating_sub(report.total_peers_after)
}

pub(super) fn evaluate_table_consistency(
    policy: &MeshPeerTablePolicy,
    report: &MeshPeerTableEnforcementReport,
) -> TableConsistencyStatus {
    let policy_target_within_capacity = policy.target_distinct_regions <= policy.max_entries;
    let policy_region_quota_within_capacity = policy.max_entries_per_region <= policy.max_entries;
    let policy_limits_invariants_all_true = policy.max_entries > 0
        && policy.max_entries_per_region > 0
        && policy.max_entries_per_region <= policy.max_entries
        && policy.target_distinct_regions > 0
        && policy.target_distinct_regions <= policy.max_entries
        && policy.stale_after_ticks > 0
        && policy.replacement_min_score_delta > 0
        && policy.degraded_replacement_min_score_delta > 0
        && policy.max_replacements_per_window > 0
        && policy.stability_window_ticks > 0
        && policy.profile_hysteresis_ticks > 0
        && policy.resilient_region_spread_bonus_weight > 0;
    let policy_consistency_all_true = policy_target_within_capacity
        && policy_region_quota_within_capacity
        && policy_limits_invariants_all_true;

    let components = drop_components_sum(report);
    let delta = drop_delta(report);
    let drop_breakdown_match = report.dropped_total == components;
    let count_transition_valid = if report.total_peers_before < report.total_peers_after {
        false
    } else {
        delta == report.dropped_total
    };
    let non_negative_drops = report.total_peers_after <= report.total_peers_before;
    let drop_delta_matches_total = delta == report.dropped_total;
    let drop_accounting_matches =
        report.dropped_total == components && report.dropped_total == delta;
    let capacity_valid = report.total_peers_after <= policy.max_entries;
    let enforcement_invariants_all_true = drop_breakdown_match
        && count_transition_valid
        && non_negative_drops
        && drop_delta_matches_total
        && drop_accounting_matches
        && capacity_valid;

    let runtime_consistency_all_true =
        policy_consistency_all_true && enforcement_invariants_all_true;
    let runtime_consistency_gate =
        match (policy_consistency_all_true, enforcement_invariants_all_true) {
            (true, true) => "ok".to_string(),
            (false, false) => "warn:policy_consistency,enforcement_invariants".to_string(),
            (false, true) => "warn:policy_consistency".to_string(),
            (true, false) => "warn:enforcement_invariants".to_string(),
        };
    let preemptive_degraded_path = !runtime_consistency_all_true;
    let preemptive_degraded_reason = if preemptive_degraded_path {
        runtime_consistency_gate.clone()
    } else {
        "none".to_string()
    };

    TableConsistencyStatus {
        policy_consistency_all_true,
        enforcement_invariants_all_true,
        runtime_consistency_all_true,
        runtime_consistency_gate,
        preemptive_degraded_path,
        preemptive_degraded_reason,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn setup_compact_consistency_summary(
    setup_compact: &str,
    consistency_gate: &str,
    degraded_path: bool,
) -> String {
    let (gate_match, degraded_match) =
        setup_compact_field_matches(setup_compact, consistency_gate, degraded_path);
    format!(
        "gate_match:{};degraded_match:{}",
        gate_match, degraded_match
    )
}

fn setup_compact_field_matches(
    setup_compact: &str,
    consistency_gate: &str,
    degraded_path: bool,
) -> (bool, bool) {
    let degraded_value = if degraded_path { "true" } else { "false" };
    let mut gate_match = false;
    let mut degraded_match = false;
    for part in setup_compact.split(';') {
        if let Some(value) = part.strip_prefix("consistency_gate:") {
            if consistency_gate.is_empty() {
                gate_match = true;
            } else {
                gate_match |= value == consistency_gate;
            }
        } else if let Some(value) = part.strip_prefix("degraded:") {
            degraded_match |= value == degraded_value;
        }
    }
    (gate_match, degraded_match)
}

pub(super) fn format_setup_compact_with_join_mode(
    join_mode: &str,
    sources: usize,
    entries_after: usize,
    consistency_gate: &str,
    degraded_path: bool,
) -> String {
    format!(
        "join_mode:{join_mode};sources:{sources};entries_after:{entries_after};consistency_gate:{consistency_gate};degraded:{degraded_path}"
    )
}

pub(super) fn format_setup_compact(
    sources: usize,
    entries_after: usize,
    consistency_gate: &str,
    degraded_path: bool,
) -> String {
    format!(
        "sources:{sources};entries_after:{entries_after};consistency_gate:{consistency_gate};degraded:{degraded_path}"
    )
}

pub(super) fn setup_compact_consistency(
    setup_compact: &str,
    consistency_gate: &str,
    degraded_path: bool,
) -> (String, bool) {
    let (gate_match, degraded_match) =
        setup_compact_field_matches(setup_compact, consistency_gate, degraded_path);
    let summary = format!(
        "gate_match:{};degraded_match:{}",
        gate_match, degraded_match
    );
    let matched = gate_match && degraded_match;
    (summary, matched)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_policy() -> MeshPeerTablePolicy {
        MeshPeerTablePolicy {
            max_entries: 8,
            max_entries_per_region: 4,
            stale_after_ticks: 16,
            target_distinct_regions: 2,
            replacement_min_score_delta: 1,
            degraded_replacement_min_score_delta: 1,
            max_replacements_per_window: 8,
            stability_window_ticks: 8,
            profile_hysteresis_ticks: 4,
            resilient_region_spread_bonus_weight: 9,
        }
    }

    fn sample_report() -> MeshPeerTableEnforcementReport {
        MeshPeerTableEnforcementReport {
            tick: 1,
            total_peers_before: 10,
            total_peers_after: 8,
            dropped_total: 2,
            dropped_by_region_cap: 1,
            dropped_by_global_cap: 1,
            protected_region_skips: 0,
            effective_profile: "balanced".to_string(),
            effective_target_distinct_regions: 2,
            effective_target_source: "balanced:configured".to_string(),
        }
    }

    #[test]
    fn evaluate_table_consistency_returns_ok_for_consistent_inputs() {
        let status = evaluate_table_consistency(&sample_policy(), &sample_report());
        assert!(status.policy_consistency_all_true);
        assert!(status.enforcement_invariants_all_true);
        assert!(status.runtime_consistency_all_true);
        assert_eq!(status.runtime_consistency_gate, "ok");
        assert!(!status.preemptive_degraded_path);
        assert_eq!(status.preemptive_degraded_reason, "none");
    }

    #[test]
    fn evaluate_table_consistency_flags_policy_consistency_violation() {
        let mut policy = sample_policy();
        policy.max_entries_per_region = policy.max_entries.saturating_add(1);
        let status = evaluate_table_consistency(&policy, &sample_report());
        assert!(!status.policy_consistency_all_true);
        assert!(!status.runtime_consistency_all_true);
        assert!(
            status
                .runtime_consistency_gate
                .contains("policy_consistency")
        );
        assert!(status.preemptive_degraded_path);
        assert!(status.preemptive_degraded_reason.starts_with("warn:"));
    }

    #[test]
    fn evaluate_table_consistency_flags_enforcement_invariant_violation() {
        let mut report = sample_report();
        report.dropped_total = 3;
        let status = evaluate_table_consistency(&sample_policy(), &report);
        assert!(!status.enforcement_invariants_all_true);
        assert!(!status.runtime_consistency_all_true);
        assert!(
            status
                .runtime_consistency_gate
                .contains("enforcement_invariants")
        );
        assert!(status.preemptive_degraded_path);
        assert!(status.preemptive_degraded_reason.starts_with("warn:"));
    }

    #[test]
    fn consistency_and_degraded_summary_have_stable_format() {
        let ok = evaluate_table_consistency(&sample_policy(), &sample_report());
        assert_eq!(ok.consistency_summary(), "gate=ok;all_true=true");
        assert_eq!(
            ok.degraded_summary(),
            "path=false;reason=none;gate=ok;all_true=true"
        );

        let mut policy = sample_policy();
        policy.max_entries_per_region = policy.max_entries.saturating_add(1);
        let warn = evaluate_table_consistency(&policy, &sample_report());
        assert!(warn.consistency_summary().starts_with("gate=warn:"));
        assert!(warn.consistency_summary().ends_with(";all_true=false"));
        assert!(
            warn.degraded_summary()
                .starts_with("path=true;reason=warn:")
        );
        assert!(warn.degraded_summary().contains(";all_true=false"));
    }

    #[test]
    fn setup_compact_consistency_summary_matches_fields_without_temp_strings() {
        let setup_compact =
            "join_mode:InvitationOnly;sources:2;entries_after:2;consistency_gate:ok;degraded:false";
        assert_eq!(
            setup_compact_consistency_summary(setup_compact, "ok", false),
            "gate_match:true;degraded_match:true"
        );
        assert_eq!(
            setup_compact_consistency_summary(setup_compact, "warn:policy_consistency", true),
            "gate_match:false;degraded_match:false"
        );
    }

    #[test]
    fn setup_compact_consistency_returns_summary_and_match_together() {
        let setup_compact =
            "join_mode:InvitationOnly;sources:2;entries_after:2;consistency_gate:ok;degraded:false";
        let (summary, matched) = setup_compact_consistency(setup_compact, "ok", false);
        assert_eq!(summary, "gate_match:true;degraded_match:true");
        assert!(matched);
        let (summary, matched) =
            setup_compact_consistency(setup_compact, "warn:policy_consistency", true);
        assert_eq!(summary, "gate_match:false;degraded_match:false");
        assert!(!matched);
    }

    #[test]
    fn evaluate_table_consistency_combines_warning_reasons_in_order() {
        let mut policy = sample_policy();
        policy.max_entries_per_region = policy.max_entries.saturating_add(1);
        let mut report = sample_report();
        report.dropped_total = 3;
        let status = evaluate_table_consistency(&policy, &report);
        assert_eq!(
            status.runtime_consistency_gate,
            "warn:policy_consistency,enforcement_invariants"
        );
        assert!(status.preemptive_degraded_path);
        assert_eq!(
            status.preemptive_degraded_reason,
            "warn:policy_consistency,enforcement_invariants"
        );
        assert_eq!(
            status.degraded_summary(),
            "path=true;reason=warn:policy_consistency,enforcement_invariants;gate=warn:policy_consistency,enforcement_invariants;all_true=false"
        );
    }
}
