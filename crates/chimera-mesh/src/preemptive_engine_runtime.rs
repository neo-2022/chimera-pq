use std::collections::{BTreeMap, BTreeSet};

use crate::model::MeshPeerState;
use crate::policy::MeshPathProfile;

use super::super::preemptive_types::{ShadowPriTuning, ShadowRuntimeDecision};
use super::risk::{apply_shadow_confirmation_gate, compute_shadow_pri};
use super::switch::{determine_shadow_action, evaluate_shadow_switch_decision};

pub(crate) fn evaluate_shadow_runtime_decision(
    profile: MeshPathProfile,
    avg_load_score: u8,
    avg_reliability_score: u8,
    health_blocked_count: usize,
    peers: &BTreeMap<String, MeshPeerState>,
    unhealthy: &BTreeSet<String>,
    tuning: &ShadowPriTuning,
) -> ShadowRuntimeDecision {
    let report = compute_shadow_pri(
        profile,
        avg_load_score,
        avg_reliability_score,
        health_blocked_count,
        tuning,
    );

    let mut confirmation = apply_shadow_confirmation_gate(
        report.stage,
        report.trigger,
        report.load_risk,
        report.reliability_risk,
        report.health_risk,
        profile,
        tuning,
    );

    let (mut switch, eligible_candidates) = evaluate_shadow_switch_decision(
        report.stage,
        report.trigger,
        report.risk.pri,
        peers,
        unhealthy,
        profile,
        tuning,
    );

    if matches!(report.stage, "switch" | "hard")
        && !confirmation.passed
        && !matches!(
            switch.reason.as_str(),
            "no_candidate" | "no_candidate_for_switch"
        )
    {
        switch.should_prepare = false;
        switch.should_switch = false;
        switch.reason = "confirmation_gate_blocked".to_string();
        confirmation.trigger = "confirmation_gate_blocked";
        confirmation.stage = "clear";
    }
    if switch.reason == "no_candidate_for_switch" {
        switch.reason = "no_candidate".to_string();
    }

    let (action, action_reason) = determine_shadow_action(&switch);

    let risk_valid = report.risk.validate().is_ok();
    let switch_valid = switch.validate().is_ok();

    ShadowRuntimeDecision {
        report,
        confirmation,
        switch,
        action,
        action_reason,
        risk_valid,
        switch_valid,
        eligible_candidates,
    }
}
