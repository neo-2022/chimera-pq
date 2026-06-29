use super::*;
use std::fmt::{Arguments, Display, Write};

pub(super) fn append_status_preemptive_confirm_lines(
    lines: &mut Vec<String>,
    report: &MeshRuntimeStatusReport,
) {
    push_line_display(
        lines,
        "status_preemptive_shadow_confirm_passed",
        report.preemptive_shadow_confirm_passed,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_confirm_n",
        report.preemptive_shadow_confirm_n,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_confirm_m",
        report.preemptive_shadow_confirm_m,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_confirm_signal_hits",
        report.preemptive_shadow_confirm_signal_hits,
    );
    push_line_fmt(
        lines,
        "status_preemptive_shadow_confirm_ratio",
        format_args!("{:.4}", report.preemptive_shadow_confirm_ratio),
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_confirm_missing_signals",
        report.preemptive_shadow_confirm_missing_signals,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_confirm_state",
        &report.preemptive_shadow_confirm_state,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_confirm_signal_labels",
        &report.preemptive_shadow_confirm_signal_labels,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_confirm_stage",
        &report.preemptive_shadow_confirm_stage,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_confirm_trigger",
        &report.preemptive_shadow_confirm_trigger,
    );
    push_line_display(
        lines,
        "status_preemptive_shadow_confirm_summary",
        &report.preemptive_shadow_confirm_summary,
    );
}

fn push_line_display<T: Display>(lines: &mut Vec<String>, key: &str, value: T) {
    let mut out = String::with_capacity(key.len().saturating_add(32));
    out.push_str(key);
    out.push('=');
    let _ = write!(&mut out, "{}", value);
    lines.push(out);
}

fn push_line_fmt(lines: &mut Vec<String>, key: &str, args: Arguments<'_>) {
    let mut out = String::with_capacity(key.len().saturating_add(128));
    out.push_str(key);
    out.push('=');
    let _ = out.write_fmt(args);
    lines.push(out);
}
