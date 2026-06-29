use super::*;
use std::fmt::{Display, Write};

pub(super) fn append_status_preemptive_validation_lines(
    lines: &mut Vec<String>,
    report: &MeshRuntimeStatusReport,
) {
    push_line_display(
        lines,
        "status_preemptive_shadow_risk_valid",
        report.preemptive_shadow_risk_valid,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_switch_valid",
        report.preemptive_shadow_switch_valid,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_eligible_candidates",
        report.preemptive_shadow_eligible_candidates,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_health_blocked_count",
        report.preemptive_shadow_health_blocked_count,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_antiflap_blocked",
        report.preemptive_shadow_antiflap_blocked,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_antiflap_reason",
        &report.preemptive_shadow_antiflap_reason,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_antiflap_replacements_window",
        report.preemptive_shadow_antiflap_replacements_window,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_antiflap_replacements_limit",
        report.preemptive_shadow_antiflap_replacements_limit,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_degraded_path",
        report.preemptive_shadow_degraded_path,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_degraded_reason",
        &report.preemptive_shadow_degraded_reason,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_degraded_summary",
        &report.preemptive_shadow_degraded_summary,
    );
    push_line_display(
        lines,
        "status_table_runtime_consistency_gate",
        &report.table_runtime_consistency_gate,
    );
    push_line_display(
        lines,
        "status_table_runtime_consistency_all_true",
        report.table_runtime_consistency_all_true,
    );
    push_line_display(
        lines,
        "status_table_runtime_consistency_summary",
        &report.table_runtime_consistency_summary,
    );
}

pub(super) fn append_status_preemptive_tuning_lines(
    lines: &mut Vec<String>,
    report: &MeshRuntimeStatusReport,
) {
    push_line_display(
        lines,
        "status_preemptive_shadow_tuning_source",
        &report.preemptive_shadow_tuning_source,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_tuning_confirmation",
        &report.preemptive_shadow_tuning_confirmation,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_tuning_weights",
        &report.preemptive_shadow_tuning_weights,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_tuning_thresholds",
        &report.preemptive_shadow_tuning_thresholds,
    );
}

fn push_line_display<T: Display>(lines: &mut Vec<String>, key: &str, value: T) {
    let mut out = String::with_capacity(key.len().saturating_add(32));
    out.push_str(key);
    out.push('=');
    let _ = write!(&mut out, "{}", value);
    lines.push(out);
}
