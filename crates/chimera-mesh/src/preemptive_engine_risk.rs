use crate::model::PreemptiveRisk;
use crate::policy::MeshPathProfile;

use super::super::preemptive_types::{
    ShadowConfirmation, ShadowPriReport, ShadowPriTuning, profile_thresholds, profile_weights,
};

pub(crate) fn compute_shadow_pri(
    profile: MeshPathProfile,
    avg_load_score: u8,
    avg_reliability_score: u8,
    health_blocked_count: usize,
    tuning: &ShadowPriTuning,
) -> ShadowPriReport {
    let load_risk = (f32::from(avg_load_score) / 100.0).clamp(0.0, 1.0);
    let reliability_risk = (1.0 - (f32::from(avg_reliability_score) / 100.0)).clamp(0.0, 1.0);
    let health_risk = if health_blocked_count == 0 {
        0.0
    } else {
        (health_blocked_count as f32 / 3.0).clamp(0.0, 1.0)
    };

    let (w_load, w_rel, w_health) = profile_weights(tuning, profile);
    let instant =
        (load_risk * w_load + reliability_risk * w_rel + health_risk * w_health).clamp(0.0, 1.0);

    let trend = (0.6 * load_risk + 0.3 * reliability_risk + 0.1 * health_risk).clamp(0.0, 1.0);
    let health_component = health_risk.min(0.5);
    let pri_pct =
        (100.0 * (0.50 * instant + 0.25 * trend + 0.15 * health_component)).clamp(0.0, 100.0);

    let (prepare, hot, switch, hard) = profile_thresholds(tuning, profile);
    let (mut stage, mut trigger) = if pri_pct >= hard {
        ("hard", "pri_hard_threshold")
    } else if pri_pct >= switch {
        ("switch", "pri_switch_threshold")
    } else if pri_pct >= hot {
        ("hot_standby", "pri_hot_threshold")
    } else if pri_pct >= prepare {
        ("prepare", "pri_prepare_threshold")
    } else {
        ("monitor", "pri_below_prepare")
    };
    if stage == "hot_standby" && health_blocked_count >= 8 {
        stage = "switch";
        trigger = "pri_switch_threshold";
    }

    ShadowPriReport {
        risk: PreemptiveRisk {
            instant_risk: f64::from(instant),
            trend_risk: f64::from(trend),
            pri: f64::from((pri_pct / 100.0).clamp(0.0, 1.0)),
            trend_lat: f64::from(load_risk),
            trend_jit: f64::from(reliability_risk),
            trend_loss: f64::from(health_risk),
        },
        stage,
        trigger,
        load_risk,
        reliability_risk,
        health_risk,
    }
}

pub(crate) fn apply_shadow_confirmation_gate(
    stage: &'static str,
    trigger: &'static str,
    load_risk: f32,
    reliability_risk: f32,
    health_risk: f32,
    profile: MeshPathProfile,
    tuning: &ShadowPriTuning,
) -> ShadowConfirmation {
    let load_hit = load_risk >= tuning.confirm_load_threshold;
    let reliability_hit = reliability_risk >= tuning.confirm_reliability_threshold;
    let health_hit = health_risk >= tuning.confirm_health_threshold;

    let mut labels: Vec<&'static str> = Vec::new();
    if load_hit {
        labels.push("load");
    }
    if reliability_hit {
        labels.push("reliability");
    }
    if health_hit {
        labels.push("health");
    }

    let signal_hits = labels.len() as u8;
    let signal_labels = match labels.as_slice() {
        [] => "none",
        ["load"] => "load",
        ["reliability"] => "reliability",
        ["health"] => "health",
        ["load", "reliability"] => "load,reliability",
        ["load", "health"] => "load,health",
        ["reliability", "health"] => "reliability,health",
        _ => "load,reliability,health",
    };

    let confirm_n = match (profile, stage) {
        (MeshPathProfile::Fast, "prepare") => tuning.confirm_fast_prepare_n,
        (MeshPathProfile::Fast, "hot_standby") => tuning.confirm_fast_hot_n,
        (MeshPathProfile::Fast, "switch") => tuning.confirm_fast_switch_n,
        (MeshPathProfile::Fast, "hard") => tuning.confirm_fast_hard_n,
        (MeshPathProfile::Balanced, "prepare") => tuning.confirm_balanced_prepare_n,
        (MeshPathProfile::Balanced, "hot_standby") => tuning.confirm_balanced_hot_n,
        (MeshPathProfile::Balanced, "switch") => tuning.confirm_balanced_switch_n,
        (MeshPathProfile::Balanced, "hard") => tuning.confirm_balanced_hard_n,
        (MeshPathProfile::Resilient, "prepare") => tuning.confirm_resilient_prepare_n,
        (MeshPathProfile::Resilient, "hot_standby") => tuning.confirm_resilient_hot_n,
        (MeshPathProfile::Resilient, "switch") => tuning.confirm_resilient_switch_n,
        (MeshPathProfile::Resilient, "hard") => tuning.confirm_resilient_hard_n,
        (_, _) => 1,
    };
    let confirm_m = tuning.confirm_m.max(confirm_n).max(1);
    let passed = signal_hits >= confirm_n;

    ShadowConfirmation {
        passed,
        confirm_n,
        confirm_m,
        signal_hits,
        signal_labels,
        stage,
        trigger,
    }
}
