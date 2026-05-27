use crate::preemptive::ShadowRuntimeDecision;

use super::*;

pub(super) fn append_preemptive_shadow_risk_lines(
    explain: &mut Vec<String>,
    shadow: &ShadowRuntimeDecision,
) {
    explain.push(format!(
        "preemptive_shadow_pri={:.2}",
        shadow.report.risk.pri * 100.0
    ));
    explain.push(format!(
        "preemptive_shadow_instant_risk={:.4}",
        shadow.report.risk.instant_risk
    ));
    explain.push(format!(
        "preemptive_shadow_trend_risk={:.4}",
        shadow.report.risk.trend_risk
    ));
    explain.push(format!("preemptive_shadow_stage={}", shadow.report.stage));
    explain.push(format!(
        "preemptive_shadow_trigger={}",
        shadow.report.trigger
    ));
    explain.push(format!(
        "preemptive_shadow_risk_summary=pri={:.2};stage={};trigger={}",
        shadow.report.risk.pri * 100.0,
        shadow.report.stage,
        shadow.report.trigger
    ));
}

pub(super) fn append_preemptive_shadow_switch_lines(
    explain: &mut Vec<String>,
    shadow: &ShadowRuntimeDecision,
    meta: PreemptiveShadowSwitchLineMeta<'_>,
) {
    explain.push(format!(
        "preemptive_shadow_switch_prepare={}",
        shadow.switch.should_prepare
    ));
    explain.push(format!(
        "preemptive_shadow_switch_recommend={}",
        shadow.switch.should_switch
    ));
    explain.push(format!(
        "preemptive_shadow_switch_reason={}",
        shadow.switch.reason
    ));
    explain.push(format!(
        "preemptive_shadow_switch_guard={}",
        meta.switch_guard
    ));
    explain.push(format!(
        "preemptive_shadow_switch_guard_source={}",
        meta.switch_guard_source
    ));
    explain.push(format!(
        "preemptive_shadow_switch_guard_summary={}|{}",
        meta.switch_guard, meta.switch_guard_source
    ));
    explain.push(format!(
        "preemptive_shadow_switch_block_reason_chain=reason={};guard={};source={}",
        shadow.switch.reason, meta.switch_guard, meta.switch_guard_source
    ));
    explain.push(format!(
        "preemptive_shadow_switch_confidence={:.4}",
        shadow.switch.confidence
    ));
    explain.push(format!(
        "preemptive_shadow_switch_candidate_confidence={:.4}",
        shadow.switch.confidence
    ));
    explain.push(format!(
        "preemptive_shadow_switch_confidence_gate_min={:.4}",
        meta.switch_confidence_gate_min
    ));
    explain.push(format!(
        "preemptive_shadow_switch_confidence_gate_passed={}",
        meta.switch_confidence_gate_passed
    ));
    explain.push(format!(
        "preemptive_shadow_switch_target={}",
        meta.switch_target
    ));
    match meta.switch_candidate_sample_age_ticks {
        Some(age) => explain.push(format!(
            "preemptive_shadow_switch_candidate_sample_age_ticks={age}"
        )),
        None => {
            explain.push("preemptive_shadow_switch_candidate_sample_age_ticks=unknown".to_string())
        }
    }
    explain.push(format!(
        "preemptive_shadow_switch_confidence_summary=conf={:.4};min={:.4};passed={};sample_age_ticks={}",
        shadow.switch.confidence,
        meta.switch_confidence_gate_min,
        meta.switch_confidence_gate_passed,
        meta.switch_candidate_sample_age_ticks
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    explain.push(format!(
        "preemptive_shadow_action={}",
        format_shadow_action(shadow.action)
    ));
    explain.push(format!(
        "preemptive_shadow_action_reason={}",
        shadow.action_reason
    ));
    explain.push(format!(
        "preemptive_shadow_action_state={}",
        format_shadow_action_state(
            shadow.action,
            shadow.action_reason,
            shadow.eligible_candidates
        )
    ));
    explain.push(format!(
        "preemptive_shadow_action_priority={}",
        shadow_action_priority(shadow.action)
    ));
}
