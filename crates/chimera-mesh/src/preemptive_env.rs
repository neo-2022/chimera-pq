use super::preemptive_types::{ShadowPriTuning, ShadowPriTuningSource};
#[path = "preemptive_env_apply.rs"]
mod apply;
#[path = "preemptive_env_sanitize.rs"]
mod sanitize;
use apply::{apply_f32_unit_value, apply_shadow_pri_value, apply_u8_range_value};
use sanitize::{
    normalize_shadow_pri_weights, sanitize_confirmation_thresholds, sanitize_shadow_thresholds,
    sanitize_switch_conf_thresholds,
};

pub(crate) fn shadow_pri_tuning_from_env() -> ShadowPriTuning {
    shadow_pri_tuning_from_kv(|key| std::env::var(key).ok())
}

pub(crate) fn shadow_pri_tuning_from_kv<F>(lookup: F) -> ShadowPriTuning
where
    F: Fn(&str) -> Option<String>,
{
    let mut tuning = ShadowPriTuning::default();
    let mut changed = false;
    let apply_shadow_pri_env = |tuning: &mut ShadowPriTuning,
                                key: &str,
                                apply: fn(&mut ShadowPriTuning, f32),
                                min: f32,
                                max: f32| {
        apply_shadow_pri_value(tuning, lookup(key).as_deref(), apply, min, max)
    };
    let apply_env_f32_unit =
        |key: &str, target: &mut f32| apply_f32_unit_value(lookup(key).as_deref(), target);
    let apply_env_u8_range = |key: &str, target: &mut u8, min: u8, max: u8| {
        apply_u8_range_value(lookup(key).as_deref(), target, min, max)
    };

    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_FAST_W_LOAD",
        |t, v| t.fast_w_load = v,
        0.0,
        1.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_FAST_W_REL",
        |t, v| t.fast_w_rel = v,
        0.0,
        1.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_FAST_W_HEALTH",
        |t, v| t.fast_w_health = v,
        0.0,
        1.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_BALANCED_W_LOAD",
        |t, v| t.balanced_w_load = v,
        0.0,
        1.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_BALANCED_W_REL",
        |t, v| t.balanced_w_rel = v,
        0.0,
        1.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_BALANCED_W_HEALTH",
        |t, v| t.balanced_w_health = v,
        0.0,
        1.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_RESILIENT_W_LOAD",
        |t, v| t.resilient_w_load = v,
        0.0,
        1.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_RESILIENT_W_REL",
        |t, v| t.resilient_w_rel = v,
        0.0,
        1.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_RESILIENT_W_HEALTH",
        |t, v| t.resilient_w_health = v,
        0.0,
        1.0,
    );

    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_FAST_PREPARE",
        |t, v| t.fast_prepare = v,
        0.0,
        100.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_FAST_HOT",
        |t, v| t.fast_hot = v,
        0.0,
        100.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_FAST_SWITCH",
        |t, v| t.fast_switch = v,
        0.0,
        100.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_FAST_HARD",
        |t, v| t.fast_hard = v,
        0.0,
        100.0,
    );

    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_BALANCED_PREPARE",
        |t, v| t.balanced_prepare = v,
        0.0,
        100.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_BALANCED_HOT",
        |t, v| t.balanced_hot = v,
        0.0,
        100.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_BALANCED_SWITCH",
        |t, v| t.balanced_switch = v,
        0.0,
        100.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_BALANCED_HARD",
        |t, v| t.balanced_hard = v,
        0.0,
        100.0,
    );

    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_RESILIENT_PREPARE",
        |t, v| t.resilient_prepare = v,
        0.0,
        100.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_RESILIENT_HOT",
        |t, v| t.resilient_hot = v,
        0.0,
        100.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_RESILIENT_SWITCH",
        |t, v| t.resilient_switch = v,
        0.0,
        100.0,
    );
    changed |= apply_shadow_pri_env(
        &mut tuning,
        "CHIMERA_SHADOW_PRI_RESILIENT_HARD",
        |t, v| t.resilient_hard = v,
        0.0,
        100.0,
    );

    changed |= apply_env_f32_unit(
        "CHIMERA_SHADOW_PRI_CONFIRM_LOAD_THRESHOLD",
        &mut tuning.confirm_load_threshold,
    );
    changed |= apply_env_f32_unit(
        "CHIMERA_SHADOW_PRI_CONFIRM_RELIABILITY_THRESHOLD",
        &mut tuning.confirm_reliability_threshold,
    );
    changed |= apply_env_f32_unit(
        "CHIMERA_SHADOW_PRI_CONFIRM_HEALTH_THRESHOLD",
        &mut tuning.confirm_health_threshold,
    );

    changed |= apply_env_u8_range(
        "CHIMERA_SHADOW_PRI_CONFIRM_FAST_PREPARE_N",
        &mut tuning.confirm_fast_prepare_n,
        1,
        10,
    );
    changed |= apply_env_u8_range(
        "CHIMERA_SHADOW_PRI_CONFIRM_FAST_HOT_N",
        &mut tuning.confirm_fast_hot_n,
        1,
        10,
    );
    changed |= apply_env_u8_range(
        "CHIMERA_SHADOW_PRI_CONFIRM_FAST_SWITCH_N",
        &mut tuning.confirm_fast_switch_n,
        1,
        10,
    );
    changed |= apply_env_u8_range(
        "CHIMERA_SHADOW_PRI_CONFIRM_FAST_HARD_N",
        &mut tuning.confirm_fast_hard_n,
        1,
        10,
    );
    changed |= apply_env_u8_range(
        "CHIMERA_SHADOW_PRI_CONFIRM_BALANCED_PREPARE_N",
        &mut tuning.confirm_balanced_prepare_n,
        1,
        10,
    );
    changed |= apply_env_u8_range(
        "CHIMERA_SHADOW_PRI_CONFIRM_BALANCED_HOT_N",
        &mut tuning.confirm_balanced_hot_n,
        1,
        10,
    );
    changed |= apply_env_u8_range(
        "CHIMERA_SHADOW_PRI_CONFIRM_BALANCED_SWITCH_N",
        &mut tuning.confirm_balanced_switch_n,
        1,
        10,
    );
    changed |= apply_env_u8_range(
        "CHIMERA_SHADOW_PRI_CONFIRM_BALANCED_HARD_N",
        &mut tuning.confirm_balanced_hard_n,
        1,
        10,
    );
    changed |= apply_env_u8_range(
        "CHIMERA_SHADOW_PRI_CONFIRM_RESILIENT_PREPARE_N",
        &mut tuning.confirm_resilient_prepare_n,
        1,
        10,
    );
    changed |= apply_env_u8_range(
        "CHIMERA_SHADOW_PRI_CONFIRM_RESILIENT_HOT_N",
        &mut tuning.confirm_resilient_hot_n,
        1,
        10,
    );
    changed |= apply_env_u8_range(
        "CHIMERA_SHADOW_PRI_CONFIRM_RESILIENT_SWITCH_N",
        &mut tuning.confirm_resilient_switch_n,
        1,
        10,
    );
    changed |= apply_env_u8_range(
        "CHIMERA_SHADOW_PRI_CONFIRM_RESILIENT_HARD_N",
        &mut tuning.confirm_resilient_hard_n,
        1,
        10,
    );
    changed |= apply_env_u8_range("CHIMERA_SHADOW_PRI_CONFIRM_M", &mut tuning.confirm_m, 1, 10);

    changed |= apply_env_f32_unit(
        "CHIMERA_SHADOW_PRI_MIN_SWITCH_CONF_FAST",
        &mut tuning.min_switch_conf_fast,
    );
    changed |= apply_env_f32_unit(
        "CHIMERA_SHADOW_PRI_MIN_SWITCH_CONF_BALANCED",
        &mut tuning.min_switch_conf_balanced,
    );
    changed |= apply_env_f32_unit(
        "CHIMERA_SHADOW_PRI_MIN_SWITCH_CONF_RESILIENT",
        &mut tuning.min_switch_conf_resilient,
    );

    normalize_shadow_pri_weights(&mut tuning);
    sanitize_shadow_thresholds(&mut tuning);
    sanitize_confirmation_thresholds(&mut tuning);
    sanitize_switch_conf_thresholds(&mut tuning);

    if changed {
        tuning.source = ShadowPriTuningSource::Env;
    }
    tuning
}

pub(crate) fn format_confirmation_tuning(tuning: &ShadowPriTuning) -> String {
    format!(
        "m={};thr={:.2}/{:.2}/{:.2};n_fast={}/{}/{}/{};n_bal={}/{}/{}/{};n_res={}/{}/{}/{};min_conf={:.2}/{:.2}/{:.2}",
        tuning.confirm_m,
        tuning.confirm_load_threshold,
        tuning.confirm_reliability_threshold,
        tuning.confirm_health_threshold,
        tuning.confirm_fast_prepare_n,
        tuning.confirm_fast_hot_n,
        tuning.confirm_fast_switch_n,
        tuning.confirm_fast_hard_n,
        tuning.confirm_balanced_prepare_n,
        tuning.confirm_balanced_hot_n,
        tuning.confirm_balanced_switch_n,
        tuning.confirm_balanced_hard_n,
        tuning.confirm_resilient_prepare_n,
        tuning.confirm_resilient_hot_n,
        tuning.confirm_resilient_switch_n,
        tuning.confirm_resilient_hard_n,
        tuning.min_switch_conf_fast,
        tuning.min_switch_conf_balanced,
        tuning.min_switch_conf_resilient
    )
}
