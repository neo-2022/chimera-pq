use super::*;
use std::fmt::{Display, Write};

pub(super) fn append_status_standby_shadow_lines(
    lines: &mut Vec<String>,
    report: &MeshRuntimeStatusReport,
) {
    push_line_display(
        lines,
        "status_standby_shadow_mode",
        &report.standby_shadow_mode,
    );
    push_line_display(
        lines,
        "status_standby_shadow_target",
        &report.standby_shadow_target,
    );
    push_line_display(
        lines,
        "status_standby_shadow_target_source",
        &report.standby_shadow_target_source,
    );
    push_line_display(
        lines,
        "status_standby_shadow_reason",
        &report.standby_shadow_reason,
    );
    push_line_display(
        lines,
        "status_standby_shadow_source",
        &report.standby_shadow_source,
    );
    push_line_display(
        lines,
        "status_standby_shadow_warm_ready",
        report.standby_shadow_warm_ready,
    );
    push_line_display(
        lines,
        "status_standby_shadow_hot_ready",
        report.standby_shadow_hot_ready,
    );
    push_line_display(
        lines,
        "status_standby_shadow_stage_source",
        &report.standby_shadow_stage_source,
    );
    push_line_display(
        lines,
        "status_standby_shadow_summary",
        &report.standby_shadow_summary,
    );
}

fn push_line_display<T: Display>(lines: &mut Vec<String>, key: &str, value: T) {
    let mut out = String::with_capacity(key.len().saturating_add(32));
    out.push_str(key);
    out.push('=');
    let _ = write!(&mut out, "{}", value);
    lines.push(out);
}
