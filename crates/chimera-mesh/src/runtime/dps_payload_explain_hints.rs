use super::super::preemptive_helpers::{
    format_hints_summary_with_source, hints_reason_from_presence, hints_source_from_status,
};
use crate::MeshTrafficHints;

pub(super) const HINT_EXPLAIN_KEYS: &[&str] = &[
    "preemptive_shadow_switch_mode=",
    "preemptive_shadow_hints_status=",
    "preemptive_shadow_hints_present=",
    "preemptive_shadow_hints_reason=",
    "preemptive_shadow_hints_multipath_mode=",
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
    let hints_multipath_mode = hints
        .multipath_mode
        .map(|v| v.as_str().to_string())
        .unwrap_or_else(|| "none".to_string());
    let hints_continuity_policy = hints
        .continuity_policy
        .map(|v| v.as_str().to_string())
        .unwrap_or_else(|| "none".to_string());

    explain.push(format!(
        "preemptive_shadow_switch_mode={}",
        hints.shadow_switch_mode.as_str()
    ));
    explain.push(format!("preemptive_shadow_hints_status={hints_status}"));
    explain.push(format!("preemptive_shadow_hints_present={hints_present}"));
    explain.push(format!("preemptive_shadow_hints_reason={hints_reason}"));
    explain.push(format!("preemptive_shadow_hints_source={hints_source}"));
    explain.push(format!(
        "preemptive_shadow_hints_multipath_mode={hints_multipath_mode}"
    ));
    explain.push(format!(
        "preemptive_shadow_hints_continuity_policy={hints_continuity_policy}"
    ));
    explain.push(format!(
        "preemptive_shadow_hints_summary={}",
        format_hints_summary_with_source(
            hints_status,
            hints_present,
            hints_reason,
            &hints_multipath_mode,
            &hints_continuity_policy
        )
    ));

    explain.push(format!("dps_payload_hints_status={hints_status}"));
    explain.push(format!("dps_payload_hints_present={hints_present}"));
    explain.push(format!("dps_payload_hints_reason={hints_reason}"));
    explain.push(format!("dps_payload_hints_source={hints_source}"));
    explain.push(format!(
        "dps_payload_hints_summary={}",
        format_hints_summary_with_source(
            hints_status,
            hints_present,
            hints_reason,
            &hints_multipath_mode,
            &hints_continuity_policy
        )
    ));
    explain.push(format!(
        "dps_payload_shadow_switch_mode={}",
        hints.shadow_switch_mode.as_str()
    ));
    explain.push(format!("dps_payload_multipath_mode={hints_multipath_mode}"));
    explain.push(format!(
        "dps_payload_continuity_policy={hints_continuity_policy}"
    ));
}

pub(super) fn append_hints_invalid(explain: &mut Vec<String>) {
    let hints_status = "invalid";
    let hints_present = false;
    let hints_reason = "dps_payload_invalid";
    let hints_source = hints_source_from_status(hints_status);
    let hints_multipath_mode = "invalid";
    let hints_continuity_policy = "invalid";

    explain.push("preemptive_shadow_switch_mode=unknown".to_string());
    explain.push(format!("preemptive_shadow_hints_status={hints_status}"));
    explain.push(format!("preemptive_shadow_hints_present={hints_present}"));
    explain.push(format!("preemptive_shadow_hints_reason={hints_reason}"));
    explain.push(format!("preemptive_shadow_hints_source={hints_source}"));
    explain.push(format!(
        "preemptive_shadow_hints_multipath_mode={hints_multipath_mode}"
    ));
    explain.push(format!(
        "preemptive_shadow_hints_continuity_policy={hints_continuity_policy}"
    ));
    explain.push(format!(
        "preemptive_shadow_hints_summary={}",
        format_hints_summary_with_source(
            hints_status,
            hints_present,
            hints_reason,
            hints_multipath_mode,
            hints_continuity_policy
        )
    ));

    explain.push(format!("dps_payload_hints_status={hints_status}"));
    explain.push(format!("dps_payload_hints_present={hints_present}"));
    explain.push(format!("dps_payload_hints_reason={hints_reason}"));
    explain.push(format!("dps_payload_hints_source={hints_source}"));
    explain.push(format!(
        "dps_payload_hints_summary={}",
        format_hints_summary_with_source(
            hints_status,
            hints_present,
            hints_reason,
            hints_multipath_mode,
            hints_continuity_policy
        )
    ));
    explain.push("dps_payload_shadow_switch_mode=unknown".to_string());
    explain.push("dps_payload_multipath_mode=invalid".to_string());
    explain.push("dps_payload_continuity_policy=invalid".to_string());
}
