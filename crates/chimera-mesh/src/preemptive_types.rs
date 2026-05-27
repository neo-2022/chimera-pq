use crate::model::{PreemptiveRisk, SwitchDecision};
use crate::policy::MeshPathProfile;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShadowPriReport {
    pub(crate) risk: PreemptiveRisk,
    pub(crate) stage: &'static str,
    pub(crate) trigger: &'static str,
    pub(crate) load_risk: f32,
    pub(crate) reliability_risk: f32,
    pub(crate) health_risk: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShadowConfirmation {
    pub(crate) passed: bool,
    pub(crate) confirm_n: u8,
    pub(crate) confirm_m: u8,
    pub(crate) signal_hits: u8,
    pub(crate) signal_labels: &'static str,
    pub(crate) stage: &'static str,
    pub(crate) trigger: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShadowRuntimeDecision {
    pub(crate) report: ShadowPriReport,
    pub(crate) confirmation: ShadowConfirmation,
    pub(crate) switch: SwitchDecision,
    pub(crate) action: ShadowAction,
    pub(crate) action_reason: &'static str,
    pub(crate) risk_valid: bool,
    pub(crate) switch_valid: bool,
    pub(crate) eligible_candidates: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShadowAction {
    Hold,
    PrepareStandby,
    KeepHotStandby,
    RecommendSwitch,
}

pub(crate) fn format_shadow_action(action: ShadowAction) -> &'static str {
    match action {
        ShadowAction::Hold => "hold",
        ShadowAction::PrepareStandby => "prepare_standby",
        ShadowAction::KeepHotStandby => "keep_hot_standby",
        ShadowAction::RecommendSwitch => "recommend_switch",
    }
}

pub(crate) fn shadow_action_priority(action: ShadowAction) -> u8 {
    match action {
        ShadowAction::Hold => 0,
        ShadowAction::PrepareStandby => 1,
        ShadowAction::KeepHotStandby => 2,
        ShadowAction::RecommendSwitch => 3,
    }
}

pub(crate) fn format_shadow_action_state(
    action: ShadowAction,
    action_reason: &str,
    eligible_candidates: usize,
) -> String {
    format!(
        "action={};reason={};priority={};eligible={}",
        format_shadow_action(action),
        action_reason,
        shadow_action_priority(action),
        eligible_candidates
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShadowPriTuning {
    pub(crate) fast_w_load: f32,
    pub(crate) fast_w_rel: f32,
    pub(crate) fast_w_health: f32,
    pub(crate) fast_prepare: f32,
    pub(crate) fast_hot: f32,
    pub(crate) fast_switch: f32,
    pub(crate) fast_hard: f32,
    pub(crate) balanced_w_load: f32,
    pub(crate) balanced_w_rel: f32,
    pub(crate) balanced_w_health: f32,
    pub(crate) balanced_prepare: f32,
    pub(crate) balanced_hot: f32,
    pub(crate) balanced_switch: f32,
    pub(crate) balanced_hard: f32,
    pub(crate) resilient_w_load: f32,
    pub(crate) resilient_w_rel: f32,
    pub(crate) resilient_w_health: f32,
    pub(crate) resilient_prepare: f32,
    pub(crate) resilient_hot: f32,
    pub(crate) resilient_switch: f32,
    pub(crate) resilient_hard: f32,
    pub(crate) source: ShadowPriTuningSource,
    pub(crate) confirm_load_threshold: f32,
    pub(crate) confirm_reliability_threshold: f32,
    pub(crate) confirm_health_threshold: f32,
    pub(crate) confirm_fast_prepare_n: u8,
    pub(crate) confirm_fast_hot_n: u8,
    pub(crate) confirm_fast_switch_n: u8,
    pub(crate) confirm_fast_hard_n: u8,
    pub(crate) confirm_balanced_prepare_n: u8,
    pub(crate) confirm_balanced_hot_n: u8,
    pub(crate) confirm_balanced_switch_n: u8,
    pub(crate) confirm_balanced_hard_n: u8,
    pub(crate) confirm_resilient_prepare_n: u8,
    pub(crate) confirm_resilient_hot_n: u8,
    pub(crate) confirm_resilient_switch_n: u8,
    pub(crate) confirm_resilient_hard_n: u8,
    pub(crate) confirm_m: u8,
    pub(crate) min_switch_conf_fast: f32,
    pub(crate) min_switch_conf_balanced: f32,
    pub(crate) min_switch_conf_resilient: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShadowPriTuningSource {
    Default,
    Env,
}

pub(crate) fn format_tuning_source(source: ShadowPriTuningSource) -> &'static str {
    match source {
        ShadowPriTuningSource::Default => "default",
        ShadowPriTuningSource::Env => "env",
    }
}

pub(crate) fn profile_weights(
    tuning: &ShadowPriTuning,
    profile: MeshPathProfile,
) -> (f32, f32, f32) {
    match profile {
        MeshPathProfile::Fast => (tuning.fast_w_load, tuning.fast_w_rel, tuning.fast_w_health),
        MeshPathProfile::Balanced => (
            tuning.balanced_w_load,
            tuning.balanced_w_rel,
            tuning.balanced_w_health,
        ),
        MeshPathProfile::Resilient => (
            tuning.resilient_w_load,
            tuning.resilient_w_rel,
            tuning.resilient_w_health,
        ),
    }
}

pub(crate) fn profile_thresholds(
    tuning: &ShadowPriTuning,
    profile: MeshPathProfile,
) -> (f32, f32, f32, f32) {
    match profile {
        MeshPathProfile::Fast => (
            tuning.fast_prepare,
            tuning.fast_hot,
            tuning.fast_switch,
            tuning.fast_hard,
        ),
        MeshPathProfile::Balanced => (
            tuning.balanced_prepare,
            tuning.balanced_hot,
            tuning.balanced_switch,
            tuning.balanced_hard,
        ),
        MeshPathProfile::Resilient => (
            tuning.resilient_prepare,
            tuning.resilient_hot,
            tuning.resilient_switch,
            tuning.resilient_hard,
        ),
    }
}

pub(crate) fn format_profile_tuning_weights(
    tuning: &ShadowPriTuning,
    profile: MeshPathProfile,
) -> String {
    let (w_load, w_rel, w_health) = profile_weights(tuning, profile);
    format!("{w_load:.3},{w_rel:.3},{w_health:.3}")
}

pub(crate) fn format_profile_tuning_thresholds(
    tuning: &ShadowPriTuning,
    profile: MeshPathProfile,
) -> String {
    let (prepare, hot, switch, hard) = profile_thresholds(tuning, profile);
    format!("{prepare:.1},{hot:.1},{switch:.1},{hard:.1}")
}

impl Default for ShadowPriTuning {
    fn default() -> Self {
        Self {
            fast_w_load: 0.35,
            fast_w_rel: 0.50,
            fast_w_health: 0.15,
            fast_prepare: 30.0,
            fast_hot: 40.0,
            fast_switch: 60.0,
            fast_hard: 85.0,
            balanced_w_load: 0.45,
            balanced_w_rel: 0.40,
            balanced_w_health: 0.15,
            balanced_prepare: 35.0,
            balanced_hot: 45.0,
            balanced_switch: 65.0,
            balanced_hard: 85.0,
            resilient_w_load: 0.50,
            resilient_w_rel: 0.30,
            resilient_w_health: 0.20,
            resilient_prepare: 25.0,
            resilient_hot: 35.0,
            resilient_switch: 55.0,
            resilient_hard: 85.0,
            source: ShadowPriTuningSource::Default,
            confirm_load_threshold: 0.60,
            confirm_reliability_threshold: 0.50,
            confirm_health_threshold: 0.34,
            confirm_fast_prepare_n: 1,
            confirm_fast_hot_n: 2,
            confirm_fast_switch_n: 2,
            confirm_fast_hard_n: 3,
            confirm_balanced_prepare_n: 1,
            confirm_balanced_hot_n: 2,
            confirm_balanced_switch_n: 3,
            confirm_balanced_hard_n: 3,
            confirm_resilient_prepare_n: 1,
            confirm_resilient_hot_n: 2,
            confirm_resilient_switch_n: 2,
            confirm_resilient_hard_n: 3,
            confirm_m: 3,
            min_switch_conf_fast: 0.55,
            min_switch_conf_balanced: 0.60,
            min_switch_conf_resilient: 0.65,
        }
    }
}

pub(crate) fn profile_min_switch_conf(tuning: &ShadowPriTuning, profile: MeshPathProfile) -> f64 {
    let value = match profile {
        MeshPathProfile::Fast => tuning.min_switch_conf_fast,
        MeshPathProfile::Balanced => tuning.min_switch_conf_balanced,
        MeshPathProfile::Resilient => tuning.min_switch_conf_resilient,
    };
    f64::from(value).clamp(0.0, 1.0)
}
