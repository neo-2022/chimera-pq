use super::snapshot::DpsPayloadExplainSnapshot;
use std::fmt::Write;

pub(in crate::runtime::dps_payload_explain) fn append_standby_summaries(
    explain: &mut Vec<String>,
    snapshot: &DpsPayloadExplainSnapshot<'_>,
) {
    explain.reserve(9);
    let standby_mode = snapshot.standby_mode.unwrap_or("off");
    let standby_target = snapshot.standby_target.unwrap_or("none");
    let standby_target_source = snapshot.standby_target_source.unwrap_or("none");
    let standby_reason = snapshot.standby_reason.unwrap_or("no_action");
    let standby_source = snapshot.standby_source.unwrap_or("preemptive_shadow");
    let standby_stage_source = snapshot
        .standby_stage_source
        .unwrap_or("stage:clear;trigger:none");
    let standby_warm = snapshot.standby_warm.unwrap_or("false");
    let standby_hot = snapshot.standby_hot.unwrap_or("false");

    push_line_str(explain, "dps_payload_standby_mode", standby_mode);
    push_line_str(explain, "dps_payload_standby_target", standby_target);
    push_line_str(
        explain,
        "dps_payload_standby_target_source",
        standby_target_source,
    );
    push_line_str(explain, "dps_payload_standby_reason", standby_reason);
    push_line_str(explain, "dps_payload_standby_source", standby_source);
    push_line_str(
        explain,
        "dps_payload_standby_stage_source",
        standby_stage_source,
    );
    push_line_str(explain, "dps_payload_standby_warm_ready", standby_warm);
    push_line_str(explain, "dps_payload_standby_hot_ready", standby_hot);
    push_line_fmt(
        explain,
        "dps_payload_standby_summary",
        format_args!(
            "mode={standby_mode};target={standby_target};target_source={standby_target_source};reason={standby_reason};source={standby_source};stage_source={standby_stage_source};warm={standby_warm};hot={standby_hot}"
        ),
    );
}

fn push_line_str(explain: &mut Vec<String>, key: &str, value: &str) {
    let mut out = String::with_capacity(key.len().saturating_add(value.len()).saturating_add(1));
    out.push_str(key);
    out.push('=');
    out.push_str(value);
    explain.push(out);
}

fn push_line_fmt(explain: &mut Vec<String>, key: &str, args: std::fmt::Arguments<'_>) {
    let mut out = String::with_capacity(key.len().saturating_add(128));
    out.push_str(key);
    out.push('=');
    let _ = out.write_fmt(args);
    explain.push(out);
}
