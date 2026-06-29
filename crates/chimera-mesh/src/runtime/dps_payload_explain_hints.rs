use super::super::preemptive_helpers::{
    format_hints_summary_with_source, hints_reason_from_presence, hints_source_from_status,
};
use crate::MeshTrafficHints;
use std::fmt::{Display, Write};

pub(super) const HINT_EXPLAIN_KEYS: &[&str] = &[
    "preemptive_shadow_switch_mode=",
    "preemptive_shadow_hints_status=",
    "preemptive_shadow_hints_present=",
    "preemptive_shadow_hints_reason=",
    "preemptive_shadow_hints_multipath_mode=",
    "preemptive_shadow_hints_multipath_demand=",
    "preemptive_shadow_hints_continuity_policy=",
    "preemptive_shadow_hints_summary=",
    "preemptive_shadow_hints_source=",
    "dps_payload_hints_source=",
];

pub(super) fn remove_explain_keys(explain: &mut Vec<String>, keys: &[&str]) {
    explain.retain(|line| !keys.iter().any(|key| line.starts_with(key)));
}

pub(super) fn append_hints_ok(explain: &mut Vec<String>, hints: &MeshTrafficHints) {
    let hints_status = "ok";
    let hints_present = hints.has_any_hint();
    let hints_reason = hints_reason_from_presence(hints.has_any_hint());
    let hints_source = hints_source_from_status(hints_status);
    let hints_multipath_mode = hints.multipath_mode.map(|v| v.as_str()).unwrap_or("none");
    let hints_continuity_policy = hints
        .continuity_policy
        .map(|v| v.as_str())
        .unwrap_or("none");
    let hints_multipath_demand = hints.multipath_demand.map(|v| v.as_str()).unwrap_or("none");
    let hints_summary = format_hints_summary_with_source(
        hints_status,
        hints_present,
        hints_reason,
        hints_multipath_mode,
        hints_continuity_policy,
    );

    push_line_str(
        explain,
        "preemptive_shadow_switch_mode",
        hints.shadow_switch_mode.as_str(),
    );
    push_line_display(explain, "preemptive_shadow_hints_status", hints_status);
    push_line_display(explain, "preemptive_shadow_hints_present", hints_present);
    push_line_display(explain, "preemptive_shadow_hints_reason", hints_reason);
    push_line_display(explain, "preemptive_shadow_hints_source", hints_source);
    push_line_str(
        explain,
        "preemptive_shadow_hints_multipath_mode",
        hints_multipath_mode,
    );
    push_line_str(
        explain,
        "preemptive_shadow_hints_multipath_demand",
        hints_multipath_demand,
    );
    push_line_str(
        explain,
        "preemptive_shadow_hints_continuity_policy",
        hints_continuity_policy,
    );
    push_line_str(explain, "preemptive_shadow_hints_summary", &hints_summary);

    push_line_display(explain, "dps_payload_hints_status", hints_status);
    push_line_display(explain, "dps_payload_hints_present", hints_present);
    push_line_display(explain, "dps_payload_hints_reason", hints_reason);
    push_line_display(explain, "dps_payload_hints_source", hints_source);
    push_line_str(explain, "dps_payload_hints_summary", &hints_summary);
    push_line_str(
        explain,
        "dps_payload_shadow_switch_mode",
        hints.shadow_switch_mode.as_str(),
    );
    push_line_str(explain, "dps_payload_multipath_mode", hints_multipath_mode);
    push_line_str(
        explain,
        "dps_payload_multipath_demand",
        hints_multipath_demand,
    );
    push_line_str(
        explain,
        "dps_payload_continuity_policy",
        hints_continuity_policy,
    );
}

fn push_line_display<T: Display>(explain: &mut Vec<String>, key: &str, value: T) {
    let mut out = String::with_capacity(key.len().saturating_add(32));
    out.push_str(key);
    out.push('=');
    let _ = write!(&mut out, "{}", value);
    explain.push(out);
}

fn push_line_str(explain: &mut Vec<String>, key: &str, value: &str) {
    let mut out = String::with_capacity(key.len().saturating_add(value.len()).saturating_add(1));
    out.push_str(key);
    out.push('=');
    out.push_str(value);
    explain.push(out);
}
