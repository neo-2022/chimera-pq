use crate::preemptive::ShadowRuntimeDecision;

pub(super) fn append_preemptive_shadow_confirm_lines(
    explain: &mut Vec<String>,
    shadow: &ShadowRuntimeDecision,
    confirm_ratio: f32,
    confirm_missing_signals: u8,
) {
    explain.push(format!(
        "preemptive_shadow_confirm_passed={}",
        shadow.confirmation.passed
    ));
    explain.push(format!(
        "preemptive_shadow_confirm_n={}",
        shadow.confirmation.confirm_n
    ));
    explain.push(format!(
        "preemptive_shadow_confirm_m={}",
        shadow.confirmation.confirm_m
    ));
    explain.push(format!(
        "preemptive_shadow_confirm_signal_hits={}",
        shadow.confirmation.signal_hits
    ));
    explain.push(format!(
        "preemptive_shadow_confirm_ratio={:.4}",
        confirm_ratio
    ));
    explain.push(format!(
        "preemptive_shadow_confirm_missing_signals={}",
        confirm_missing_signals
    ));
    explain.push(format!(
        "preemptive_shadow_confirm_state=hits={}/{};need={};missing={};passed={}",
        shadow.confirmation.signal_hits,
        shadow.confirmation.confirm_m,
        shadow.confirmation.confirm_n,
        confirm_missing_signals,
        shadow.confirmation.passed
    ));
    explain.push(format!(
        "preemptive_shadow_confirm_signal_labels={}",
        shadow.confirmation.signal_labels
    ));
    explain.push(format!(
        "preemptive_shadow_confirm_stage={}",
        shadow.confirmation.stage
    ));
    explain.push(format!(
        "preemptive_shadow_confirm_trigger={}",
        shadow.confirmation.trigger
    ));
    explain.push(format!(
        "preemptive_shadow_confirm_summary=hits={}/need={};stage={};trigger={}",
        shadow.confirmation.signal_hits,
        shadow.confirmation.confirm_n,
        shadow.confirmation.stage,
        shadow.confirmation.trigger
    ));
}
