use crate::model::MeshPeerState;
use crate::preemptive::{ShadowPriTuning, evaluate_shadow_switch_decision};
use crate::{MeshPathProfile, PreemptiveRisk, SwitchDecision};

#[test]
fn preemptive_risk_validation_accepts_valid_unit_interval_values() {
    let risk = PreemptiveRisk {
        instant_risk: 0.7,
        trend_risk: 0.4,
        pri: 0.65,
        trend_lat: 0.05,
        trend_jit: 0.02,
        trend_loss: 0.01,
    };
    assert!(risk.validate().is_ok());
}

#[test]
fn preemptive_risk_validation_rejects_out_of_range_values() {
    let risk = PreemptiveRisk {
        instant_risk: 1.1,
        trend_risk: 0.4,
        pri: 0.65,
        trend_lat: 0.05,
        trend_jit: 0.02,
        trend_loss: 0.01,
    };
    assert!(risk.validate().is_err());
}

#[test]
fn switch_decision_validation_checks_reason_and_confidence() {
    let ok = SwitchDecision {
        should_prepare: true,
        should_switch: false,
        target_peer: Some("node-a".to_string()),
        reason: "pri_prepare_threshold".to_string(),
        confidence: 0.9,
    };
    assert!(ok.validate().is_ok());

    let bad_reason = SwitchDecision {
        should_prepare: false,
        should_switch: false,
        target_peer: None,
        reason: "   ".to_string(),
        confidence: 0.5,
    };
    assert!(bad_reason.validate().is_err());

    let bad_conf = SwitchDecision {
        should_prepare: false,
        should_switch: false,
        target_peer: None,
        reason: "no_candidate".to_string(),
        confidence: 1.5,
    };
    assert!(bad_conf.validate().is_err());
}

#[test]
fn switch_stage_without_candidates_has_specific_reason() {
    use std::collections::{BTreeMap, BTreeSet};

    let peers: BTreeMap<String, MeshPeerState> = BTreeMap::new();
    let unhealthy: BTreeSet<String> = BTreeSet::new();
    let tuning = ShadowPriTuning::default();

    let (decision, eligible) = evaluate_shadow_switch_decision(
        "switch",
        "pri_switch_threshold",
        0.8,
        &peers,
        &unhealthy,
        MeshPathProfile::Balanced,
        &tuning,
    );

    assert_eq!(eligible, 0);
    assert!(!decision.should_prepare);
    assert!(!decision.should_switch);
    assert!(decision.target_peer.is_none());
    assert_eq!(decision.reason, "no_candidate_for_switch");
    assert!((decision.confidence - 0.0).abs() < f64::EPSILON);
}
