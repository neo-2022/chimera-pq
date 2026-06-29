use super::preemptive_status_lines::append_status_preemptive_shadow_lines;
use super::status_base_explain::status_base_lines;
use super::status_report_builder::build_status_report_from_snapshot;
use super::status_shadow_snapshot::build_shadow_status_snapshot;
use super::*;
use std::fmt::Write;

pub(super) fn build_status_report(
    runtime: &MeshRuntime,
    payload: Option<&str>,
) -> MeshRuntimeStatusReport {
    let snapshot = build_shadow_status_snapshot(runtime, payload);
    build_status_report_from_snapshot(runtime, snapshot)
}

pub(super) fn build_status_explain(
    runtime: &MeshRuntime,
    report: &MeshRuntimeStatusReport,
) -> Vec<String> {
    let mut lines = status_base_lines(report, runtime);
    lines.reserve(74);
    let mut preemptive_shadow_compact = String::with_capacity(
        "status_preemptive_shadow_compact=".len()
            + report.preemptive_shadow_stage.len()
            + report.preemptive_shadow_trigger.len()
            + report.table_runtime_consistency_gate.len()
            + report.preemptive_shadow_switch_reason.len()
            + report.preemptive_shadow_switch_guard.len()
            + report.preemptive_shadow_switch_guard_source.len()
            + report.plan_setup_discovery_table_compact.len()
            + report.plan_setup_discovery_table_compact_consistency.len()
            + report.setup_compact_consistency_match_source.len()
            + report.plan_setup_compact_consistency_match_source.len()
            + 192,
    );
    preemptive_shadow_compact.push_str("status_preemptive_shadow_compact=");
    let _ = write!(
        &mut preemptive_shadow_compact,
        "stage:{};trigger:{};pri={:.2};degraded={};consistency_gate={};confidence=conf={:.4};min={:.4};passed={};sample_age_ticks={};reason_chain=reason={};guard={};source={};setup_compact={};setup_consistency={};setup_match={};setup_match_source={};plan_setup_match_source={}",
        report.preemptive_shadow_stage,
        report.preemptive_shadow_trigger,
        report.preemptive_shadow_pri,
        report.preemptive_shadow_degraded_path,
        report.table_runtime_consistency_gate,
        report.preemptive_shadow_switch_candidate_confidence,
        report.preemptive_shadow_switch_confidence_gate_min,
        report.preemptive_shadow_switch_confidence_gate_passed,
        report.preemptive_shadow_switch_candidate_sample_age_ticks,
        report.preemptive_shadow_switch_reason,
        report.preemptive_shadow_switch_guard,
        report.preemptive_shadow_switch_guard_source,
        report.plan_setup_discovery_table_compact,
        report.plan_setup_discovery_table_compact_consistency,
        report.setup_compact_consistency_match,
        report.setup_compact_consistency_match_source,
        report.plan_setup_compact_consistency_match_source
    );
    lines.push(preemptive_shadow_compact);
    lines.push(format!(
        "status_consistency_source_matrix=setup={};plan_setup={};compact_setup={};compact_plan_setup={}",
        report.setup_compact_consistency_match_source,
        report.plan_setup_compact_consistency_match_source,
        report.setup_compact_consistency_match_source,
        report.plan_setup_compact_consistency_match_source
    ));
    append_status_preemptive_shadow_lines(&mut lines, report);
    lines
}
