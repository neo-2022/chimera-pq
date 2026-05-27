use super::standby_status_lines::status_standby_shadow_lines;
use super::*;

#[path = "preemptive_status_lines_sections.rs"]
mod sections;

pub(super) fn status_preemptive_shadow_lines(report: &MeshRuntimeStatusReport) -> Vec<String> {
    let mut lines = Vec::new();
    sections::append_status_preemptive_risk_lines(&mut lines, report);
    sections::append_status_preemptive_switch_lines(&mut lines, report);
    sections::append_status_preemptive_confirm_lines(&mut lines, report);
    sections::append_status_preemptive_validation_lines(&mut lines, report);
    sections::append_status_preemptive_tuning_lines(&mut lines, report);
    lines.extend(status_standby_shadow_lines(report));
    lines
}
