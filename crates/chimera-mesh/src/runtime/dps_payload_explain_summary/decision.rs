use super::snapshot::DpsPayloadExplainSnapshot;
use crate::runtime::preemptive_helpers::shadow_switch_guard_meta;
use crate::runtime::table_consistency::setup_compact_consistency;
use std::fmt::{Display, Write};

pub(in crate::runtime::dps_payload_explain) fn append_decision_summaries(
    explain: &mut Vec<String>,
    snapshot: &DpsPayloadExplainSnapshot<'_>,
) {
    explain.reserve(24);
    let switch_reason = snapshot.switch_reason.unwrap_or("none");
    let (switch_guard, switch_guard_source) = shadow_switch_guard_meta(switch_reason);
    push_line_str(explain, "dps_payload_switch_guard", switch_guard);
    push_line_str(
        explain,
        "dps_payload_switch_guard_source",
        switch_guard_source,
    );
    push_line_fmt(
        explain,
        "dps_payload_switch_guard_summary",
        format_args!("{switch_guard}|{switch_guard_source}"),
    );
    push_line_fmt(
        explain,
        "dps_payload_switch_block_reason_chain",
        format_args!(
            "reason={};guard={};source={}",
            switch_reason, switch_guard, switch_guard_source
        ),
    );

    let confirm_n = snapshot.confirm_n.unwrap_or("0");
    let confirm_hits = snapshot.confirm_hits.unwrap_or("0");
    let confirm_stage = snapshot.confirm_stage.unwrap_or("none");
    let confirm_trigger = snapshot.confirm_trigger.unwrap_or("none");
    push_line_fmt(
        explain,
        "dps_payload_confirm_summary",
        format_args!(
            "hits={confirm_hits}/need={confirm_n};stage={confirm_stage};trigger={confirm_trigger}"
        ),
    );

    let pri = snapshot.pri.unwrap_or("0.00");
    let stage = snapshot.stage.unwrap_or("none");
    let trigger = snapshot.trigger.unwrap_or("none");
    let switch_candidate_confidence = snapshot
        .switch_candidate_confidence
        .or(snapshot.switch_confidence)
        .unwrap_or("0.0000");
    let switch_confidence_gate_min = snapshot.switch_confidence_gate_min.unwrap_or("0.0000");
    let switch_confidence_gate_passed = snapshot.switch_confidence_gate_passed.unwrap_or("false");
    let switch_candidate_sample_age_ticks = snapshot
        .switch_candidate_sample_age_ticks
        .unwrap_or("unknown");
    push_line_fmt(
        explain,
        "dps_payload_risk_summary",
        format_args!("pri={pri};stage={stage};trigger={trigger}"),
    );
    push_line_str(
        explain,
        "dps_payload_preemptive_switch_candidate_confidence",
        switch_candidate_confidence,
    );
    push_line_str(
        explain,
        "dps_payload_preemptive_switch_confidence_gate_min",
        switch_confidence_gate_min,
    );
    push_line_str(
        explain,
        "dps_payload_preemptive_switch_confidence_gate_passed",
        switch_confidence_gate_passed,
    );
    push_line_str(
        explain,
        "dps_payload_preemptive_switch_candidate_sample_age_ticks",
        switch_candidate_sample_age_ticks,
    );
    push_line_fmt(
        explain,
        "dps_payload_preemptive_switch_confidence_summary",
        format_args!(
            "conf={switch_candidate_confidence};min={switch_confidence_gate_min};passed={switch_confidence_gate_passed};sample_age_ticks={switch_candidate_sample_age_ticks};reason_chain=reason={switch_reason};guard={switch_guard};source={switch_guard_source}"
        ),
    );
    append_selection_pressure_summaries(explain, snapshot);
    append_consistency_summaries(explain, snapshot, pri, stage, trigger);
}

fn append_selection_pressure_summaries(
    explain: &mut Vec<String>,
    snapshot: &DpsPayloadExplainSnapshot<'_>,
) {
    let candidate_readiness_summary = snapshot.candidate_readiness_summary.unwrap_or(
        "eligible=0;switch_valid=false;health_blocked=0;confidence_gate_passed=false;sample_age_ticks=unknown",
    );
    push_line_str(
        explain,
        "dps_payload_candidate_readiness_summary",
        candidate_readiness_summary,
    );
    let selection_pressure_summary = snapshot.selection_pressure_summary.unwrap_or(
        "considered:0;selected:0;rejected:0;limit_skipped:0;utilization_pct:0;headroom:0",
    );
    let selection_pressure_level = snapshot.selection_pressure_level.unwrap_or("unknown");
    let selection_pressure_score = snapshot.selection_pressure_score.unwrap_or("0");
    let selection_pressure_dominant = snapshot.selection_pressure_dominant.unwrap_or("none");
    let selection_pressure_action_hint = snapshot.selection_pressure_action_hint.unwrap_or("none");
    let selection_pressure_compact = snapshot
        .selection_pressure_compact
        .unwrap_or("level:unknown;score:0;dominant:none;action:none");
    let selection_pressure_reason = snapshot.selection_pressure_reason.unwrap_or(
        "level=unknown;dominant=none;blocked=0;health=0;region=0;reliability=0;load=0;limit_skipped=0;headroom=0",
    );
    push_line_str(
        explain,
        "dps_payload_selection_pressure_summary",
        selection_pressure_summary,
    );
    push_line_str(
        explain,
        "dps_payload_selection_pressure_level",
        selection_pressure_level,
    );
    push_line_str(
        explain,
        "dps_payload_selection_pressure_score",
        selection_pressure_score,
    );
    push_line_str(
        explain,
        "dps_payload_selection_pressure_dominant",
        selection_pressure_dominant,
    );
    push_line_str(
        explain,
        "dps_payload_selection_pressure_action_hint",
        selection_pressure_action_hint,
    );
    push_line_str(
        explain,
        "dps_payload_selection_pressure_compact",
        selection_pressure_compact,
    );
    push_line_str(
        explain,
        "dps_payload_selection_pressure_reason",
        selection_pressure_reason,
    );
}

fn append_consistency_summaries(
    explain: &mut Vec<String>,
    snapshot: &DpsPayloadExplainSnapshot<'_>,
    pri: &str,
    stage: &str,
    trigger: &str,
) {
    let tuning_source = snapshot.tuning_source.unwrap_or("default");
    let tuning_weights = snapshot.tuning_weights.unwrap_or("none");
    let tuning_thresholds = snapshot.tuning_thresholds.unwrap_or("none");
    let tuning_confirmation = snapshot.tuning_confirmation.unwrap_or("none");
    push_line_fmt(
        explain,
        "dps_payload_tuning_summary",
        format_args!(
            "src={tuning_source};w={tuning_weights};thr={tuning_thresholds};conf={tuning_confirmation}"
        ),
    );
    let degraded_path = snapshot.degraded_path.unwrap_or("false");
    let degraded_reason = snapshot.degraded_reason.unwrap_or("none");
    let consistency_gate = snapshot.consistency_gate.unwrap_or("unknown");
    let consistency_all_true = snapshot.consistency_all_true.unwrap_or("false");
    let setup_compact = snapshot.setup_compact.unwrap_or(
        "join_mode:Unknown;sources:0;entries_after:0;consistency_gate:unknown;degraded:false",
    );
    let setup_compact_consistency_from_plan = snapshot
        .setup_compact_consistency_from_plan
        .unwrap_or("gate_match:unknown;degraded_match:unknown");
    let plan_setup_compact_consistency_match = snapshot
        .plan_setup_compact_consistency_match
        .unwrap_or("false");
    push_line_str(
        explain,
        "dps_payload_table_runtime_consistency_gate",
        consistency_gate,
    );
    push_line_str(
        explain,
        "dps_payload_table_runtime_consistency_all_true",
        consistency_all_true,
    );
    push_line_fmt(
        explain,
        "dps_payload_table_runtime_consistency_summary",
        format_args!("gate={consistency_gate};all_true={consistency_all_true}"),
    );
    push_line_str(
        explain,
        "dps_payload_plan_setup_discovery_table_compact",
        setup_compact,
    );
    push_line_str(
        explain,
        "dps_payload_plan_setup_discovery_table_compact_consistency",
        setup_compact_consistency_from_plan,
    );
    push_line_display(
        explain,
        "dps_payload_plan_setup_discovery_table_compact_consistency_match",
        plan_setup_compact_consistency_match,
    );
    push_line_str(
        explain,
        "dps_payload_plan_setup_discovery_table_compact_consistency_match_source",
        "plan_setup_explain",
    );
    let (compact_consistency, setup_compact_consistency_match) =
        setup_compact_consistency(setup_compact, consistency_gate, degraded_path == "true");
    push_line_str(
        explain,
        "dps_payload_plan_setup_compact_consistency",
        &compact_consistency,
    );
    push_line_display(
        explain,
        "dps_payload_setup_compact_consistency_match",
        setup_compact_consistency_match,
    );
    push_line_str(
        explain,
        "dps_payload_setup_compact_consistency_match_source",
        "computed_from_setup_compact",
    );
    push_line_str(
        explain,
        "dps_payload_preemptive_degraded_path",
        degraded_path,
    );
    push_line_str(
        explain,
        "dps_payload_preemptive_degraded_reason",
        degraded_reason,
    );
    push_line_fmt(
        explain,
        "dps_payload_preemptive_degraded_summary",
        format_args!(
            "path={degraded_path};reason={degraded_reason};gate={consistency_gate};all_true={consistency_all_true}"
        ),
    );
    push_line_fmt(
        explain,
        "dps_payload_preemptive_shadow_compact",
        format_args!(
            "pri={pri};stage={stage};trigger={trigger};degraded={degraded_path};consistency_gate={consistency_gate};setup_consistency={compact_consistency};setup_match={setup_compact_consistency_match};setup_match_source=computed_from_setup_compact;plan_setup_match_source=plan_setup_explain"
        ),
    );
    push_line_str(
        explain,
        "dps_payload_consistency_source_matrix",
        "setup=computed_from_setup_compact;plan_setup=plan_setup_explain;compact_setup=computed_from_setup_compact;compact_plan_setup=plan_setup_explain",
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

fn push_line_fmt(explain: &mut Vec<String>, key: &str, args: std::fmt::Arguments<'_>) {
    let mut out = String::with_capacity(key.len().saturating_add(128));
    out.push_str(key);
    out.push('=');
    let _ = out.write_fmt(args);
    explain.push(out);
}
