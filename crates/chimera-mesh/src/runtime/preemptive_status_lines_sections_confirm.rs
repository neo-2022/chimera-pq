use super::*;

pub(super) fn append_status_preemptive_confirm_lines(
    lines: &mut Vec<String>,
    report: &MeshRuntimeStatusReport,
) {
    lines.push(format!(
        "status_preemptive_shadow_confirm_passed={}",
        report.preemptive_shadow_confirm_passed
    ));
    lines.push(format!(
        "status_preemptive_shadow_confirm_n={}",
        report.preemptive_shadow_confirm_n
    ));
    lines.push(format!(
        "status_preemptive_shadow_confirm_m={}",
        report.preemptive_shadow_confirm_m
    ));
    lines.push(format!(
        "status_preemptive_shadow_confirm_signal_hits={}",
        report.preemptive_shadow_confirm_signal_hits
    ));
    lines.push(format!(
        "status_preemptive_shadow_confirm_ratio={:.4}",
        report.preemptive_shadow_confirm_ratio
    ));
    lines.push(format!(
        "status_preemptive_shadow_confirm_missing_signals={}",
        report.preemptive_shadow_confirm_missing_signals
    ));
    lines.push(format!(
        "status_preemptive_shadow_confirm_state={}",
        report.preemptive_shadow_confirm_state
    ));
    lines.push(format!(
        "status_preemptive_shadow_confirm_signal_labels={}",
        report.preemptive_shadow_confirm_signal_labels
    ));
    lines.push(format!(
        "status_preemptive_shadow_confirm_stage={}",
        report.preemptive_shadow_confirm_stage
    ));
    lines.push(format!(
        "status_preemptive_shadow_confirm_trigger={}",
        report.preemptive_shadow_confirm_trigger
    ));
    lines.push(format!(
        "status_preemptive_shadow_confirm_summary={}",
        report.preemptive_shadow_confirm_summary
    ));
}
