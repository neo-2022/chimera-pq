use super::ShadowPriTuning;

pub(super) fn apply_shadow_pri_value(
    tuning: &mut ShadowPriTuning,
    raw: Option<&str>,
    apply: fn(&mut ShadowPriTuning, f32),
    min: f32,
    max: f32,
) -> bool {
    let Some(raw) = raw else {
        return false;
    };
    let Ok(parsed) = raw.parse::<f32>() else {
        return false;
    };
    if !parsed.is_finite() {
        return false;
    }
    apply(tuning, parsed.clamp(min, max));
    true
}

pub(super) fn apply_f32_unit_value(raw: Option<&str>, target: &mut f32) -> bool {
    let Some(raw) = raw else {
        return false;
    };
    let Ok(parsed) = raw.parse::<f32>() else {
        return false;
    };
    if !parsed.is_finite() {
        return false;
    }
    *target = parsed.clamp(0.0, 1.0);
    true
}

pub(super) fn apply_u8_range_value(raw: Option<&str>, target: &mut u8, min: u8, max: u8) -> bool {
    let Some(raw) = raw else {
        return false;
    };
    let Ok(parsed) = raw.parse::<u8>() else {
        return false;
    };
    *target = parsed.clamp(min, max);
    true
}
