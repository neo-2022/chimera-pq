use super::standby_status_lines::append_status_standby_shadow_lines;
use super::*;

#[path = "preemptive_status_lines_sections.rs"]
mod sections;

pub(super) fn append_status_preemptive_shadow_lines(
    lines: &mut Vec<String>,
    report: &MeshRuntimeStatusReport,
) {
    sections::append_status_preemptive_risk_lines(lines, report);
    sections::append_status_preemptive_switch_lines(lines, report);
    sections::append_status_preemptive_confirm_lines(lines, report);
    sections::append_status_preemptive_validation_lines(lines, report);
    sections::append_status_preemptive_tuning_lines(lines, report);
    append_status_standby_shadow_lines(lines, report);
}
