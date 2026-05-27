use super::*;

#[path = "preemptive_status_lines_sections_confirm.rs"]
mod confirm;
#[path = "preemptive_status_lines_sections_risk_switch.rs"]
mod risk_switch;
#[path = "preemptive_status_lines_sections_validation_tuning.rs"]
mod validation_tuning;

pub(super) fn append_status_preemptive_confirm_lines(
    lines: &mut Vec<String>,
    report: &MeshRuntimeStatusReport,
) {
    confirm::append_status_preemptive_confirm_lines(lines, report);
}

pub(super) fn append_status_preemptive_risk_lines(
    lines: &mut Vec<String>,
    report: &MeshRuntimeStatusReport,
) {
    risk_switch::append_status_preemptive_risk_lines(lines, report);
}

pub(super) fn append_status_preemptive_switch_lines(
    lines: &mut Vec<String>,
    report: &MeshRuntimeStatusReport,
) {
    risk_switch::append_status_preemptive_switch_lines(lines, report);
}

pub(super) fn append_status_preemptive_tuning_lines(
    lines: &mut Vec<String>,
    report: &MeshRuntimeStatusReport,
) {
    validation_tuning::append_status_preemptive_tuning_lines(lines, report);
}

pub(super) fn append_status_preemptive_validation_lines(
    lines: &mut Vec<String>,
    report: &MeshRuntimeStatusReport,
) {
    validation_tuning::append_status_preemptive_validation_lines(lines, report);
}
