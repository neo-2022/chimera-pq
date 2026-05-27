use std::collections::{BTreeMap, BTreeSet};

use crate::model::{MeshPeerState, SwitchDecision};
use crate::policy::MeshPathProfile;

use super::super::preemptive_types::{ShadowAction, ShadowPriTuning, profile_min_switch_conf};

pub(crate) fn evaluate_shadow_switch_decision(
    stage: &'static str,
    trigger: &'static str,
    pri: f64,
    peers: &BTreeMap<String, MeshPeerState>,
    unhealthy: &BTreeSet<String>,
    profile: MeshPathProfile,
    tuning: &ShadowPriTuning,
) -> (SwitchDecision, usize) {
    let mut eligible: Vec<&MeshPeerState> = peers
        .values()
        .filter(|peer| !unhealthy.contains(peer.node_id.as_str()))
        .collect();
    eligible.sort_by(|a, b| {
        b.reliability_score
            .cmp(&a.reliability_score)
            .then_with(|| a.load_score.cmp(&b.load_score))
            .then_with(|| a.node_id.cmp(&b.node_id))
    });

    let eligible_count = eligible.len();
    if eligible_count == 0 {
        return (
            SwitchDecision {
                should_prepare: false,
                should_switch: false,
                target_peer: None,
                reason: "no_candidate_for_switch".to_string(),
                confidence: 0.0,
            },
            0,
        );
    }

    let target = eligible[0];
    let confidence = (f64::from(target.reliability_score) / 100.0).clamp(0.0, 1.0);
    let min_switch_conf = profile_min_switch_conf(tuning, profile);

    let should_prepare = matches!(stage, "prepare" | "hot_standby" | "switch" | "hard");
    let should_switch = matches!(stage, "switch" | "hard") && confidence >= min_switch_conf;

    let reason = if matches!(stage, "switch" | "hard") && confidence < min_switch_conf {
        "candidate_low_confidence"
    } else if should_switch {
        trigger
    } else if should_prepare {
        "standby_prepared"
    } else {
        "no_action"
    };

    let _ = pri;
    (
        SwitchDecision {
            should_prepare,
            should_switch,
            target_peer: Some(target.node_id.clone()),
            reason: reason.to_string(),
            confidence,
        },
        eligible_count,
    )
}

pub(super) fn determine_shadow_action(switch: &SwitchDecision) -> (ShadowAction, &'static str) {
    if switch.reason == "candidate_low_confidence" {
        return (ShadowAction::Hold, "no_action");
    }
    if switch.should_switch {
        (ShadowAction::RecommendSwitch, "switch_recommended")
    } else if switch.should_prepare {
        if switch.reason == "switch_budget_exceeded" {
            (ShadowAction::KeepHotStandby, "switch_budget_exceeded")
        } else {
            (ShadowAction::PrepareStandby, "standby_prepared")
        }
    } else {
        (ShadowAction::Hold, "no_action")
    }
}
