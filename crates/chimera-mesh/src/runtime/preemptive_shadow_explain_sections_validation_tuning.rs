use crate::preemptive::{ShadowPriTuning, ShadowPriTuningSource, ShadowRuntimeDecision};

use super::preemptive_helpers::shadow_antiflap_meta;
use super::*;

pub(super) fn append_preemptive_shadow_validation_lines(
    explain: &mut Vec<String>,
    shadow: &ShadowRuntimeDecision,
    health_blocked_count: usize,
    replacements_window: u64,
    replacements_limit: u64,
    switch_confidence_gate_passed: bool,
    switch_candidate_sample_age_ticks: Option<u64>,
) {
    let (antiflap_blocked, antiflap_reason) = shadow_antiflap_meta(shadow.switch.reason.as_str());
    explain.push(format!(
        "preemptive_shadow_risk_valid={}",
        shadow.risk_valid
    ));
    explain.push(format!(
        "preemptive_shadow_switch_valid={}",
        shadow.switch_valid
    ));
    explain.push(format!(
        "preemptive_shadow_eligible_candidates={}",
        shadow.eligible_candidates
    ));
    explain.push(format!(
        "preemptive_shadow_health_blocked_count={}",
        health_blocked_count
    ));
    explain.push(format!(
        "preemptive_shadow_antiflap_blocked={}",
        antiflap_blocked
    ));
    explain.push(format!(
        "preemptive_shadow_antiflap_reason={}",
        antiflap_reason
    ));
    explain.push(format!(
        "preemptive_shadow_antiflap_replacements_window={}",
        replacements_window
    ));
    explain.push(format!(
        "preemptive_shadow_antiflap_replacements_limit={}",
        replacements_limit
    ));
    explain.push(format!(
        "preemptive_shadow_candidate_readiness_summary=eligible={};switch_valid={};health_blocked={};confidence_gate_passed={};sample_age_ticks={}",
        shadow.eligible_candidates,
        shadow.switch_valid,
        health_blocked_count,
        switch_confidence_gate_passed,
        switch_candidate_sample_age_ticks
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
}

pub(super) fn append_preemptive_shadow_tuning_lines(
    explain: &mut Vec<String>,
    tuning: &ShadowPriTuning,
    profile: MeshPathProfile,
    tuning_source_label: fn(ShadowPriTuningSource) -> &'static str,
) {
    explain.push(format!(
        "preemptive_shadow_tuning_source={}",
        tuning_source_label(tuning.source)
    ));
    explain.push(format!(
        "preemptive_shadow_tuning_confirmation={}",
        format_confirmation_tuning(tuning)
    ));
    explain.push(format!(
        "preemptive_shadow_tuning_weights={}",
        format_profile_tuning_weights(tuning, profile)
    ));
    explain.push(format!(
        "preemptive_shadow_tuning_thresholds={}",
        format_profile_tuning_thresholds(tuning, profile)
    ));
}
