use super::super::preemptive_helpers::{explain_value, shadow_switch_guard_meta};
use super::table_consistency::setup_compact_consistency;

pub(super) fn append_decision_summaries(explain: &mut Vec<String>) {
    let switch_reason = explain_value(explain, "preemptive_shadow_switch_reason=")
        .unwrap_or("none")
        .to_string();
    let (switch_guard, switch_guard_source) = shadow_switch_guard_meta(switch_reason.as_str());
    explain.push(format!("dps_payload_switch_guard={switch_guard}"));
    explain.push(format!(
        "dps_payload_switch_guard_source={switch_guard_source}"
    ));
    explain.push(format!(
        "dps_payload_switch_guard_summary={switch_guard}|{switch_guard_source}"
    ));
    explain.push(format!(
        "dps_payload_switch_block_reason_chain=reason={};guard={};source={}",
        switch_reason, switch_guard, switch_guard_source
    ));

    let confirm_n = explain_value(explain, "preemptive_shadow_confirm_n=").unwrap_or("0");
    let confirm_hits =
        explain_value(explain, "preemptive_shadow_confirm_signal_hits=").unwrap_or("0");
    let confirm_stage =
        explain_value(explain, "preemptive_shadow_confirm_stage=").unwrap_or("none");
    let confirm_trigger =
        explain_value(explain, "preemptive_shadow_confirm_trigger=").unwrap_or("none");
    explain.push(format!(
        "dps_payload_confirm_summary=hits={confirm_hits}/need={confirm_n};stage={confirm_stage};trigger={confirm_trigger}"
    ));

    let pri = explain_value(explain, "preemptive_shadow_pri=")
        .unwrap_or("0.00")
        .to_string();
    let stage = explain_value(explain, "preemptive_shadow_stage=")
        .unwrap_or("none")
        .to_string();
    let trigger = explain_value(explain, "preemptive_shadow_trigger=")
        .unwrap_or("none")
        .to_string();
    let switch_candidate_confidence =
        explain_value(explain, "preemptive_shadow_switch_candidate_confidence=")
            .or_else(|| explain_value(explain, "preemptive_shadow_switch_confidence="))
            .unwrap_or("0.0000")
            .to_string();
    let switch_confidence_gate_min =
        explain_value(explain, "preemptive_shadow_switch_confidence_gate_min=")
            .unwrap_or("0.0000")
            .to_string();
    let switch_confidence_gate_passed =
        explain_value(explain, "preemptive_shadow_switch_confidence_gate_passed=")
            .unwrap_or("false")
            .to_string();
    let switch_candidate_sample_age_ticks = explain_value(
        explain,
        "preemptive_shadow_switch_candidate_sample_age_ticks=",
    )
    .unwrap_or("unknown")
    .to_string();
    explain.push(format!(
        "dps_payload_risk_summary=pri={pri};stage={stage};trigger={trigger}"
    ));
    explain.push(format!(
        "dps_payload_preemptive_switch_candidate_confidence={switch_candidate_confidence}"
    ));
    explain.push(format!(
        "dps_payload_preemptive_switch_confidence_gate_min={switch_confidence_gate_min}"
    ));
    explain.push(format!(
        "dps_payload_preemptive_switch_confidence_gate_passed={switch_confidence_gate_passed}"
    ));
    explain.push(format!(
        "dps_payload_preemptive_switch_candidate_sample_age_ticks={switch_candidate_sample_age_ticks}"
    ));
    explain.push(format!(
        "dps_payload_preemptive_switch_confidence_summary=conf={switch_candidate_confidence};min={switch_confidence_gate_min};passed={switch_confidence_gate_passed};sample_age_ticks={switch_candidate_sample_age_ticks};reason_chain=reason={switch_reason};guard={switch_guard};source={switch_guard_source}"
    ));
    let candidate_readiness_summary =
        explain_value(explain, "preemptive_shadow_candidate_readiness_summary=")
            .unwrap_or("eligible=0;switch_valid=false;health_blocked=0;confidence_gate_passed=false;sample_age_ticks=unknown");
    explain.push(format!(
        "dps_payload_candidate_readiness_summary={candidate_readiness_summary}"
    ));
    let selection_pressure_summary = explain_value(explain, "selection_pressure_summary=")
        .unwrap_or(
            "considered:0;selected:0;rejected:0;limit_skipped:0;utilization_pct:0;headroom:0",
        )
        .to_string();
    let selection_pressure_level = explain_value(explain, "selection_pressure_level=")
        .unwrap_or("unknown")
        .to_string();
    let selection_pressure_score = explain_value(explain, "selection_pressure_score=")
        .unwrap_or("0")
        .to_string();
    let selection_pressure_dominant = explain_value(explain, "selection_pressure_dominant=")
        .unwrap_or("none")
        .to_string();
    let selection_pressure_action_hint = explain_value(explain, "selection_pressure_action_hint=")
        .unwrap_or("none")
        .to_string();
    let selection_pressure_compact = explain_value(explain, "selection_pressure_compact=")
        .unwrap_or("level:unknown;score:0;dominant:none;action:none")
        .to_string();
    let selection_pressure_reason = explain_value(explain, "selection_pressure_reason=")
        .unwrap_or("level=unknown;dominant=none;blocked=0;health=0;region=0;reliability=0;load=0;limit_skipped=0;headroom=0")
        .to_string();
    explain.push(format!(
        "dps_payload_selection_pressure_summary={selection_pressure_summary}"
    ));
    explain.push(format!(
        "dps_payload_selection_pressure_level={selection_pressure_level}"
    ));
    explain.push(format!(
        "dps_payload_selection_pressure_score={selection_pressure_score}"
    ));
    explain.push(format!(
        "dps_payload_selection_pressure_dominant={selection_pressure_dominant}"
    ));
    explain.push(format!(
        "dps_payload_selection_pressure_action_hint={selection_pressure_action_hint}"
    ));
    explain.push(format!(
        "dps_payload_selection_pressure_compact={selection_pressure_compact}"
    ));
    explain.push(format!(
        "dps_payload_selection_pressure_reason={selection_pressure_reason}"
    ));

    let tuning_source =
        explain_value(explain, "preemptive_shadow_tuning_source=").unwrap_or("default");
    let tuning_weights =
        explain_value(explain, "preemptive_shadow_tuning_weights=").unwrap_or("none");
    let tuning_thresholds =
        explain_value(explain, "preemptive_shadow_tuning_thresholds=").unwrap_or("none");
    let tuning_confirmation =
        explain_value(explain, "preemptive_shadow_tuning_confirmation=").unwrap_or("none");
    explain.push(format!(
        "dps_payload_tuning_summary=src={tuning_source};w={tuning_weights};thr={tuning_thresholds};conf={tuning_confirmation}"
    ));
    let degraded_path = explain_value(explain, "preemptive_shadow_degraded_path=")
        .unwrap_or("false")
        .to_string();
    let degraded_reason = explain_value(explain, "preemptive_shadow_degraded_reason=")
        .unwrap_or("none")
        .to_string();
    let consistency_gate = explain_value(explain, "peer_table_runtime_consistency_gate=")
        .unwrap_or("unknown")
        .to_string();
    let consistency_all_true = explain_value(explain, "peer_table_runtime_consistency_all_true=")
        .unwrap_or("false")
        .to_string();
    let setup_compact = explain_value(explain, "plan_setup_discovery_table_compact=")
        .unwrap_or(
            "join_mode:Unknown;sources:0;entries_after:0;consistency_gate:unknown;degraded:false",
        )
        .to_string();
    let setup_compact_consistency_from_plan =
        explain_value(explain, "plan_setup_discovery_table_compact_consistency=")
            .unwrap_or("gate_match:unknown;degraded_match:unknown")
            .to_string();
    let plan_setup_compact_consistency_match = explain_value(
        explain,
        "plan_setup_discovery_table_compact_consistency_match=",
    )
    .unwrap_or("false")
    .to_string();
    explain.push(format!(
        "dps_payload_table_runtime_consistency_gate={consistency_gate}"
    ));
    explain.push(format!(
        "dps_payload_table_runtime_consistency_all_true={consistency_all_true}"
    ));
    explain.push(format!(
        "dps_payload_table_runtime_consistency_summary=gate={consistency_gate};all_true={consistency_all_true}"
    ));
    explain.push(format!(
        "dps_payload_plan_setup_discovery_table_compact={setup_compact}"
    ));
    explain.push(format!(
        "dps_payload_plan_setup_discovery_table_compact_consistency={setup_compact_consistency_from_plan}"
    ));
    explain.push(format!(
        "dps_payload_plan_setup_discovery_table_compact_consistency_match={plan_setup_compact_consistency_match}"
    ));
    explain.push(
        "dps_payload_plan_setup_discovery_table_compact_consistency_match_source=plan_setup_explain"
            .to_string(),
    );
    let (compact_consistency, setup_compact_consistency_match) =
        setup_compact_consistency(&setup_compact, &consistency_gate, degraded_path == "true");
    let setup_compact_consistency_match = setup_compact_consistency_match.to_string();
    explain.push(format!(
        "dps_payload_plan_setup_compact_consistency={compact_consistency}"
    ));
    explain.push(format!(
        "dps_payload_setup_compact_consistency_match={setup_compact_consistency_match}"
    ));
    explain.push(
        "dps_payload_setup_compact_consistency_match_source=computed_from_setup_compact"
            .to_string(),
    );
    explain.push(format!(
        "dps_payload_preemptive_degraded_path={degraded_path}"
    ));
    explain.push(format!(
        "dps_payload_preemptive_degraded_reason={degraded_reason}"
    ));
    explain.push(format!(
        "dps_payload_preemptive_degraded_summary=path={degraded_path};reason={degraded_reason};gate={consistency_gate};all_true={consistency_all_true}"
    ));
    explain.push(format!(
        "dps_payload_preemptive_shadow_compact=pri={pri};stage={stage};trigger={trigger};degraded={degraded_path};consistency_gate={consistency_gate};setup_consistency={compact_consistency};setup_match={setup_compact_consistency_match};setup_match_source=computed_from_setup_compact;plan_setup_match_source=plan_setup_explain"
    ));
    explain.push(
        "dps_payload_consistency_source_matrix=setup=computed_from_setup_compact;plan_setup=plan_setup_explain;compact_setup=computed_from_setup_compact;compact_plan_setup=plan_setup_explain"
            .to_string(),
    );
}

pub(super) fn append_standby_summaries(explain: &mut Vec<String>) {
    let standby_mode = explain_value(explain, "standby_shadow_mode=")
        .unwrap_or("off")
        .to_string();
    let standby_target = explain_value(explain, "standby_shadow_target=")
        .unwrap_or("none")
        .to_string();
    let standby_target_source = explain_value(explain, "standby_shadow_target_source=")
        .unwrap_or("none")
        .to_string();
    let standby_reason = explain_value(explain, "standby_shadow_reason=")
        .unwrap_or("no_action")
        .to_string();
    let standby_source = explain_value(explain, "standby_shadow_source=")
        .unwrap_or("preemptive_shadow")
        .to_string();
    let standby_stage_source = explain_value(explain, "standby_shadow_stage_source=")
        .unwrap_or("stage:clear;trigger:none")
        .to_string();
    let standby_warm = explain_value(explain, "standby_shadow_warm_ready=")
        .unwrap_or("false")
        .to_string();
    let standby_hot = explain_value(explain, "standby_shadow_hot_ready=")
        .unwrap_or("false")
        .to_string();

    explain.push(format!("dps_payload_standby_mode={standby_mode}"));
    explain.push(format!("dps_payload_standby_target={standby_target}"));
    explain.push(format!(
        "dps_payload_standby_target_source={standby_target_source}"
    ));
    explain.push(format!("dps_payload_standby_reason={standby_reason}"));
    explain.push(format!("dps_payload_standby_source={standby_source}"));
    explain.push(format!(
        "dps_payload_standby_stage_source={standby_stage_source}"
    ));
    explain.push(format!("dps_payload_standby_warm_ready={standby_warm}"));
    explain.push(format!("dps_payload_standby_hot_ready={standby_hot}"));
    explain.push(format!(
        "dps_payload_standby_summary=mode={standby_mode};target={standby_target};target_source={standby_target_source};reason={standby_reason};source={standby_source};stage_source={standby_stage_source};warm={standby_warm};hot={standby_hot}"
    ));
}
