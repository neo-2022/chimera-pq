use super::status_shadow_snapshot::ShadowStatusSnapshot;
use super::table_consistency::{
    evaluate_table_consistency, format_setup_compact, setup_compact_consistency_match,
    setup_compact_consistency_summary,
};
use super::*;
#[path = "status_report_builder_hints.rs"]
mod hints;
#[path = "status_report_builder_shadow.rs"]
mod shadow;

pub(super) fn build_status_report_from_snapshot(
    runtime: &MeshRuntime,
    snapshot: ShadowStatusSnapshot,
) -> MeshRuntimeStatusReport {
    let hints_summary = hints::build_status_hints_summary(&snapshot);
    let table_consistency = evaluate_table_consistency(
        &runtime.table_policy,
        &runtime.last_table_enforcement_report,
    );
    let table_runtime_consistency_summary = table_consistency.consistency_summary();
    let preemptive_shadow_degraded_summary = table_consistency.degraded_summary();
    let plan_setup_discovery_table_compact = format_setup_compact(
        runtime.source_count(),
        runtime.last_table_enforcement_report.total_peers_after,
        &table_consistency.runtime_consistency_gate,
        table_consistency.preemptive_degraded_path,
    );
    let plan_setup_discovery_table_compact_consistency = setup_compact_consistency_summary(
        &plan_setup_discovery_table_compact,
        &table_consistency.runtime_consistency_gate,
        table_consistency.preemptive_degraded_path,
    );
    let setup_compact_consistency_match =
        setup_compact_consistency_match(&plan_setup_discovery_table_compact_consistency);
    let risk_summary = shadow::preemptive_shadow_risk_summary(&snapshot);
    let switch_guard_summary = shadow::preemptive_shadow_switch_guard_summary(&snapshot);
    let confirm_state = shadow::preemptive_shadow_confirm_state(&snapshot);
    let confirm_summary = shadow::preemptive_shadow_confirm_summary(&snapshot);
    MeshRuntimeStatusReport {
        namespace: runtime.namespace.clone(),
        source_count: runtime.source_count(),
        peer_count: runtime.peer_count(),
        health_state_count: runtime.health_state_count(),
        active_profile: profile_label(runtime.profile_state.active_profile).to_string(),
        table_policy: runtime.table_policy.clone(),
        table_enforcement: runtime.last_table_enforcement_report.clone(),
        preemptive_shadow_pri: (snapshot.shadow.report.risk.pri * 100.0) as f32,
        preemptive_shadow_instant_risk: snapshot.shadow.report.risk.instant_risk as f32,
        preemptive_shadow_trend_risk: snapshot.shadow.report.risk.trend_risk as f32,
        preemptive_shadow_stage: snapshot.shadow.report.stage.to_string(),
        preemptive_shadow_trigger: snapshot.shadow.report.trigger.to_string(),
        preemptive_shadow_risk_summary: risk_summary,
        preemptive_shadow_switch_prepare: snapshot.shadow.switch.should_prepare,
        preemptive_shadow_switch_recommend: snapshot.shadow.switch.should_switch,
        preemptive_shadow_switch_reason: snapshot.shadow.switch.reason.clone(),
        preemptive_shadow_switch_guard: snapshot.switch_guard.clone(),
        preemptive_shadow_switch_guard_source: snapshot.switch_guard_source.clone(),
        preemptive_shadow_switch_guard_summary: switch_guard_summary,
        preemptive_shadow_switch_confidence: snapshot.shadow.switch.confidence,
        preemptive_shadow_switch_candidate_confidence: snapshot.shadow.switch.confidence,
        preemptive_shadow_switch_confidence_gate_min: snapshot.switch_confidence_gate_min,
        preemptive_shadow_switch_confidence_gate_passed: snapshot.switch_confidence_gate_passed,
        preemptive_shadow_switch_candidate_sample_age_ticks: snapshot
            .switch_candidate_sample_age_ticks
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        preemptive_shadow_switch_target: snapshot.switch_target,
        preemptive_shadow_switch_mode: snapshot.switch_mode,
        preemptive_shadow_hints_status: snapshot.hints_status,
        preemptive_shadow_hints_source: hints_summary.source,
        preemptive_shadow_hints_reason: snapshot.hints_reason,
        preemptive_shadow_hints_present: snapshot.hints_present,
        preemptive_shadow_hints_multipath_mode: snapshot.hints_multipath_mode,
        preemptive_shadow_hints_continuity_policy: snapshot.hints_continuity_policy,
        preemptive_shadow_hints_summary: hints_summary.summary,
        preemptive_shadow_action: format_shadow_action(snapshot.shadow.action).to_string(),
        preemptive_shadow_action_reason: snapshot.shadow.action_reason.to_string(),
        preemptive_shadow_action_state: format_shadow_action_state(
            snapshot.shadow.action,
            snapshot.shadow.action_reason,
            snapshot.shadow.eligible_candidates,
        ),
        preemptive_shadow_action_priority: shadow_action_priority(snapshot.shadow.action),
        preemptive_shadow_confirm_passed: snapshot.shadow.confirmation.passed,
        preemptive_shadow_confirm_n: snapshot.shadow.confirmation.confirm_n,
        preemptive_shadow_confirm_m: snapshot.shadow.confirmation.confirm_m,
        preemptive_shadow_confirm_signal_hits: snapshot.shadow.confirmation.signal_hits,
        preemptive_shadow_confirm_ratio: snapshot.confirm_ratio,
        preemptive_shadow_confirm_missing_signals: snapshot.confirm_missing_signals,
        preemptive_shadow_confirm_state: confirm_state,
        preemptive_shadow_confirm_signal_labels: snapshot
            .shadow
            .confirmation
            .signal_labels
            .to_string(),
        preemptive_shadow_confirm_stage: snapshot.shadow.confirmation.stage.to_string(),
        preemptive_shadow_confirm_trigger: snapshot.shadow.confirmation.trigger.to_string(),
        preemptive_shadow_confirm_summary: confirm_summary,
        preemptive_shadow_risk_valid: snapshot.shadow.risk_valid,
        preemptive_shadow_switch_valid: snapshot.shadow.switch_valid,
        preemptive_shadow_eligible_candidates: snapshot.shadow.eligible_candidates,
        preemptive_shadow_health_blocked_count: snapshot.health_blocked_count,
        preemptive_shadow_antiflap_blocked: snapshot.antiflap.blocked,
        preemptive_shadow_antiflap_reason: snapshot.antiflap.reason.to_string(),
        preemptive_shadow_antiflap_replacements_window: snapshot.antiflap.replacements_window,
        preemptive_shadow_antiflap_replacements_limit: snapshot.antiflap.replacements_limit,
        preemptive_shadow_tuning_source: snapshot.tuning_source,
        preemptive_shadow_tuning_confirmation: snapshot.tuning_confirmation,
        preemptive_shadow_tuning_weights: snapshot.tuning_weights,
        preemptive_shadow_tuning_thresholds: snapshot.tuning_thresholds,
        table_runtime_consistency_all_true: table_consistency.runtime_consistency_all_true,
        table_runtime_consistency_gate: table_consistency.runtime_consistency_gate.clone(),
        table_runtime_consistency_summary,
        plan_setup_discovery_table_compact,
        plan_setup_discovery_table_compact_consistency,
        setup_compact_consistency_match,
        setup_compact_consistency_match_source: "status_report".to_string(),
        plan_setup_compact_consistency_match_source: "status_report".to_string(),
        preemptive_shadow_degraded_path: table_consistency.preemptive_degraded_path,
        preemptive_shadow_degraded_reason: table_consistency.preemptive_degraded_reason,
        preemptive_shadow_degraded_summary,
        standby_shadow_mode: snapshot.standby.mode,
        standby_shadow_target: snapshot.standby.target,
        standby_shadow_target_source: snapshot.standby.target_source,
        standby_shadow_reason: snapshot.standby.reason,
        standby_shadow_source: snapshot.standby.source,
        standby_shadow_warm_ready: snapshot.standby.warm_ready,
        standby_shadow_hot_ready: snapshot.standby.hot_ready,
        standby_shadow_stage_source: snapshot.standby.stage_source,
        standby_shadow_summary: snapshot.standby.summary,
    }
}
