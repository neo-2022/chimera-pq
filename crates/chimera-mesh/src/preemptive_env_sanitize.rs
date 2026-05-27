use super::ShadowPriTuning;

pub(super) fn normalize_shadow_pri_weights(tuning: &mut ShadowPriTuning) {
    fn normalize3(a: &mut f32, b: &mut f32, c: &mut f32) {
        let sum = (*a + *b + *c).max(0.0001);
        *a = (*a / sum).clamp(0.0, 1.0);
        *b = (*b / sum).clamp(0.0, 1.0);
        *c = (*c / sum).clamp(0.0, 1.0);
    }

    normalize3(
        &mut tuning.fast_w_load,
        &mut tuning.fast_w_rel,
        &mut tuning.fast_w_health,
    );
    normalize3(
        &mut tuning.balanced_w_load,
        &mut tuning.balanced_w_rel,
        &mut tuning.balanced_w_health,
    );
    normalize3(
        &mut tuning.resilient_w_load,
        &mut tuning.resilient_w_rel,
        &mut tuning.resilient_w_health,
    );
}

pub(super) fn sanitize_shadow_thresholds(tuning: &mut ShadowPriTuning) {
    fn sanitize(prepare: &mut f32, hot: &mut f32, switch: &mut f32, hard: &mut f32) {
        *prepare = prepare.clamp(0.0, 100.0);
        *hot = hot.clamp(*prepare, 100.0);
        *switch = switch.clamp(*hot, 100.0);
        *hard = hard.clamp(*switch, 100.0);
    }

    sanitize(
        &mut tuning.fast_prepare,
        &mut tuning.fast_hot,
        &mut tuning.fast_switch,
        &mut tuning.fast_hard,
    );
    sanitize(
        &mut tuning.balanced_prepare,
        &mut tuning.balanced_hot,
        &mut tuning.balanced_switch,
        &mut tuning.balanced_hard,
    );
    sanitize(
        &mut tuning.resilient_prepare,
        &mut tuning.resilient_hot,
        &mut tuning.resilient_switch,
        &mut tuning.resilient_hard,
    );
}

pub(super) fn sanitize_confirmation_thresholds(tuning: &mut ShadowPriTuning) {
    tuning.confirm_m = tuning.confirm_m.max(1);

    for n in [
        &mut tuning.confirm_fast_prepare_n,
        &mut tuning.confirm_fast_hot_n,
        &mut tuning.confirm_fast_switch_n,
        &mut tuning.confirm_fast_hard_n,
        &mut tuning.confirm_balanced_prepare_n,
        &mut tuning.confirm_balanced_hot_n,
        &mut tuning.confirm_balanced_switch_n,
        &mut tuning.confirm_balanced_hard_n,
        &mut tuning.confirm_resilient_prepare_n,
        &mut tuning.confirm_resilient_hot_n,
        &mut tuning.confirm_resilient_switch_n,
        &mut tuning.confirm_resilient_hard_n,
    ] {
        *n = (*n).clamp(1, tuning.confirm_m);
    }
}

pub(super) fn sanitize_switch_conf_thresholds(tuning: &mut ShadowPriTuning) {
    tuning.min_switch_conf_fast = tuning.min_switch_conf_fast.clamp(0.0, 1.0);
    tuning.min_switch_conf_balanced = tuning.min_switch_conf_balanced.clamp(0.0, 1.0);
    tuning.min_switch_conf_resilient = tuning.min_switch_conf_resilient.clamp(0.0, 1.0);
}
