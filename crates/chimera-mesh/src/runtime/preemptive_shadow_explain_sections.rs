use crate::preemptive::{ShadowPriTuning, ShadowPriTuningSource, ShadowRuntimeDecision};

use super::*;

#[path = "preemptive_shadow_explain_sections_confirm.rs"]
mod confirm;
#[path = "preemptive_shadow_explain_sections_risk_switch.rs"]
mod risk_switch;
#[path = "preemptive_shadow_explain_sections_validation_tuning.rs"]
mod validation_tuning;

pub(super) struct PreemptiveShadowSwitchLineMeta<'a> {
    pub(super) switch_guard: &'a str,
    pub(super) switch_guard_source: &'a str,
    pub(super) switch_target: &'a str,
    pub(super) switch_candidate_sample_age_ticks: Option<u64>,
    pub(super) switch_confidence_gate_min: f64,
    pub(super) switch_confidence_gate_passed: bool,
}

pub(super) fn append_preemptive_shadow_confirm_lines(
    explain: &mut Vec<String>,
    shadow: &ShadowRuntimeDecision,
    confirm_ratio: f32,
    confirm_missing_signals: u8,
) {
    confirm::append_preemptive_shadow_confirm_lines(
        explain,
        shadow,
        confirm_ratio,
        confirm_missing_signals,
    );
}

pub(super) fn append_preemptive_shadow_risk_lines(
    explain: &mut Vec<String>,
    shadow: &ShadowRuntimeDecision,
) {
    risk_switch::append_preemptive_shadow_risk_lines(explain, shadow);
}

pub(super) fn append_preemptive_shadow_switch_lines(
    explain: &mut Vec<String>,
    shadow: &ShadowRuntimeDecision,
    meta: PreemptiveShadowSwitchLineMeta<'_>,
) {
    risk_switch::append_preemptive_shadow_switch_lines(explain, shadow, meta);
}

pub(super) fn append_preemptive_shadow_validation_lines(
    explain: &mut Vec<String>,
    shadow: &ShadowRuntimeDecision,
    health_blocked_count: usize,
    replacements_window: u64,
    replacements_limit: u64,
    switch_confidence_gate_passed: bool,
    switch_candidate_sample_age_ticks: Option<u64>,
) {
    validation_tuning::append_preemptive_shadow_validation_lines(
        explain,
        shadow,
        health_blocked_count,
        replacements_window,
        replacements_limit,
        switch_confidence_gate_passed,
        switch_candidate_sample_age_ticks,
    );
}

pub(super) fn append_preemptive_shadow_tuning_lines(
    explain: &mut Vec<String>,
    tuning: &ShadowPriTuning,
    profile: MeshPathProfile,
    tuning_source_label: fn(ShadowPriTuningSource) -> &'static str,
) {
    validation_tuning::append_preemptive_shadow_tuning_lines(
        explain,
        tuning,
        profile,
        tuning_source_label,
    );
}
