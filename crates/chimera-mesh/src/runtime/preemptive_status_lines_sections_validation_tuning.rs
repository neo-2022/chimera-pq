use super::*;

pub(super) fn append_status_preemptive_validation_lines(
    lines: &mut Vec<String>,
    report: &MeshRuntimeStatusReport,
) {
    lines.push(format!(
        "status_preemptive_shadow_risk_valid={}",
        report.preemptive_shadow_risk_valid
    ));
    lines.push(format!(
        "status_preemptive_shadow_switch_valid={}",
        report.preemptive_shadow_switch_valid
    ));
    lines.push(format!(
        "status_preemptive_shadow_eligible_candidates={}",
        report.preemptive_shadow_eligible_candidates
    ));
    lines.push(format!(
        "status_preemptive_shadow_health_blocked_count={}",
        report.preemptive_shadow_health_blocked_count
    ));
    lines.push(format!(
        "status_preemptive_shadow_antiflap_blocked={}",
        report.preemptive_shadow_antiflap_blocked
    ));
    lines.push(format!(
        "status_preemptive_shadow_antiflap_reason={}",
        report.preemptive_shadow_antiflap_reason
    ));
    lines.push(format!(
        "status_preemptive_shadow_antiflap_replacements_window={}",
        report.preemptive_shadow_antiflap_replacements_window
    ));
    lines.push(format!(
        "status_preemptive_shadow_antiflap_replacements_limit={}",
        report.preemptive_shadow_antiflap_replacements_limit
    ));
    lines.push(format!(
        "status_preemptive_shadow_degraded_path={}",
        report.preemptive_shadow_degraded_path
    ));
    lines.push(format!(
        "status_preemptive_shadow_degraded_reason={}",
        report.preemptive_shadow_degraded_reason
    ));
    lines.push(format!(
        "status_preemptive_shadow_degraded_summary={}",
        report.preemptive_shadow_degraded_summary
    ));
    lines.push(format!(
        "status_table_runtime_consistency_gate={}",
        report.table_runtime_consistency_gate
    ));
    lines.push(format!(
        "status_table_runtime_consistency_all_true={}",
        report.table_runtime_consistency_all_true
    ));
    lines.push(format!(
        "status_table_runtime_consistency_summary={}",
        report.table_runtime_consistency_summary
    ));
}

pub(super) fn append_status_preemptive_tuning_lines(
    lines: &mut Vec<String>,
    report: &MeshRuntimeStatusReport,
) {
    lines.push(format!(
        "status_preemptive_shadow_tuning_source={}",
        report.preemptive_shadow_tuning_source
    ));
    lines.push(format!(
        "status_preemptive_shadow_tuning_confirmation={}",
        report.preemptive_shadow_tuning_confirmation
    ));
    lines.push(format!(
        "status_preemptive_shadow_tuning_weights={}",
        report.preemptive_shadow_tuning_weights
    ));
    lines.push(format!(
        "status_preemptive_shadow_tuning_thresholds={}",
        report.preemptive_shadow_tuning_thresholds
    ));
}
