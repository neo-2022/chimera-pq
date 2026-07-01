use super::council::validate_council_stage_report;
use super::coverage::{
    CoverageCatalog, collect_and_validate_stage_coverage, validate_stage_source_line_coverage,
};
use super::stage_reports::validate_stage_specific_report;
use super::support::*;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub(crate) const REQUIRED_STAGES: [&str; 8] = [
    "analyze",
    "impact_analysis",
    "research_planning",
    "research",
    "architecture_synthesis",
    "implementation",
    "validation",
    "postmortem",
];

const CONCLUSION_CLASSES: [&str; 5] = [
    "proven",
    "confirmed_by_independent_sources",
    "assumption",
    "hypothesis",
    "opinion",
];

pub(crate) fn validate_full_stage_sequence(stages: &[Value]) -> Result<(), String> {
    if stages.len() != REQUIRED_STAGES.len() {
        return Err(
            "workflow attestation guard: full lifecycle must contain exactly 8 stages".to_string(),
        );
    }
    for (idx, expected) in REQUIRED_STAGES.iter().enumerate() {
        let obj = value_obj(&stages[idx], "stage")?;
        let actual = require_non_empty_str(obj, "id")?;
        if actual != *expected {
            return Err(format!(
                "workflow attestation guard: stage order mismatch: expected {expected}, got {actual}"
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_stages(stages: &[Value], done_like: bool) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for stage in stages {
        let obj = value_obj(stage, "stage")?;
        let id = require_enum(obj, "id", &REQUIRED_STAGES)?;
        if !seen.insert(id) {
            return Err(format!("workflow attestation guard: duplicate stage: {id}"));
        }

        require_bool(obj, "stage_report", true)?;
        require_bool(obj, "council_review", true)?;
        require_bool(obj, "red_team", true)?;
        require_bool(obj, "council_report", true)?;
        validate_evidence_array(obj, "evidence")?;

        let gate = require_obj(obj, "gate")?;
        validate_gate(gate, id)?;
        if done_like && gate.get("decision").and_then(Value::as_str) != Some("pass") {
            return Err(format!(
                "workflow attestation guard: done/pass requires pass gate: {id}"
            ));
        }
        if gate.get("decision").and_then(Value::as_str) == Some("pass") {
            require_empty_array(obj, "critical_risks")?;
            require_empty_array(obj, "blocking_issues")?;
        }
    }
    Ok(())
}

pub(crate) fn validate_detailed_stage_reports(
    root: &Map<String, Value>,
    stages: &[Value],
    done_like: bool,
    coverage_items: &CoverageCatalog,
) -> Result<(), String> {
    if !done_like {
        return Ok(());
    }
    let reports = require_obj(root, "stage_reports")?;
    let mut covered = BTreeSet::new();
    for stage in REQUIRED_STAGES {
        let report = require_obj(reports, stage)?;
        validate_stage_report_common(report, stage)?;
        validate_stage_source_line_coverage(report, stage)?;
        validate_stage_specific_report(report, stage)?;
        collect_and_validate_stage_coverage(stage, report, coverage_items, &mut covered)?;
    }
    for stage in stages {
        let obj = value_obj(stage, "stage")?;
        let id = require_enum(obj, "id", &REQUIRED_STAGES)?;
        let expected_ref = format!("stage_reports.{id}");
        require_str(obj, "stage_report_ref", &expected_ref)?;
    }
    if covered != coverage_items.ids {
        let missing: Vec<_> = coverage_items
            .ids
            .difference(&covered)
            .take(5)
            .cloned()
            .collect();
        let extra: Vec<_> = covered
            .difference(&coverage_items.ids)
            .take(5)
            .cloned()
            .collect();
        return Err(format!(
            "workflow attestation guard: stage report coverage mismatch; missing={missing:?}; extra={extra:?}"
        ));
    }
    Ok(())
}

pub(crate) fn validate_rollback_events(root: &Map<String, Value>) -> Result<(), String> {
    let events = require_array(root, "rollback_events")?;
    for event in events {
        let obj = value_obj(event, "rollback event")?;
        require_enum(obj, "from_stage", &REQUIRED_STAGES)?;
        require_enum(obj, "to_stage", &REQUIRED_STAGES)?;
        require_non_empty_str(obj, "reason")?;
        require_non_empty_str(obj, "root_cause_stage")?;
    }
    Ok(())
}

fn validate_stage_report_common(report: &Map<String, Value>, stage: &str) -> Result<(), String> {
    require_str(report, "stage_id", stage)?;
    require_non_empty_str(report, "goal")?;
    validate_evidence_array(report, "evidence")?;
    validate_named_non_empty_array(report, "inputs")?;
    validate_named_non_empty_array(report, "actions")?;
    validate_named_non_empty_array(report, "constraints")?;
    validate_named_non_empty_array(report, "invariants")?;
    validate_named_non_empty_array(report, "assumptions")?;
    validate_named_array(report, "unknowns")?;
    validate_named_array(report, "risks")?;
    validate_named_array(report, "blockers")?;
    validate_named_non_empty_array(report, "outcome")?;

    let conclusions = require_non_empty_array(report, "conclusions")?;
    for conclusion in conclusions {
        validate_conclusion(value_obj(conclusion, "stage conclusion")?)?;
    }

    validate_council_stage_report(require_obj(report, "council_stage_report")?)?;
    validate_gate_report(require_obj(report, "gate_report")?, stage)
}

fn validate_conclusion(conclusion: &Map<String, Value>) -> Result<(), String> {
    require_non_empty_str(conclusion, "claim")?;
    require_enum(conclusion, "classification", &CONCLUSION_CLASSES)?;
    require_non_empty_str(conclusion, "confidence")?;
    validate_evidence_array(conclusion, "evidence")
}

fn validate_gate_report(gate: &Map<String, Value>, stage: &str) -> Result<(), String> {
    require_str(gate, "stage_id", stage)?;
    let decision = require_enum(gate, "decision", &["pass", "return", "stop"])?;
    if decision != "pass" {
        return Err(format!(
            "workflow attestation guard: done/pass requires pass gate report: {stage}"
        ));
    }
    validate_evidence_array(gate, "evidence")?;
    validate_named_non_empty_array(gate, "check_result")?;
    validate_named_array(gate, "found_problems")?;
    validate_named_array(gate, "found_risks")?;
    validate_named_array(gate, "found_violations")?;
    validate_named_array(gate, "unmet_requirements")?;
    validate_named_non_empty_array(gate, "recommendations")?;
    require_non_empty_str(gate, "justification")?;
    require_non_empty_str(gate, "transfer_of_control")?;
    require_bool(gate, "does_not_fix_errors", true)?;
    require_bool(gate, "does_not_replace_council", true)?;
    require_bool(gate, "does_not_make_architecture_decisions", true)?;

    if decision == "pass" {
        require_empty_array(gate, "found_problems")?;
        require_empty_array(gate, "found_violations")?;
        require_empty_array(gate, "unmet_requirements")?;
    }

    if decision == "return" {
        let rollback = require_obj(gate, "earliest_rollback_analysis")?;
        for key in [
            "error_first_occurred_stage",
            "wrong_decision_stage",
            "missed_check_stage",
            "research_should_have_been_stage",
            "earliest_capable_stage",
            "rejected_later_stages",
        ] {
            require_non_empty_str(rollback, key)?;
        }
    }
    Ok(())
}

fn validate_gate(gate: &Map<String, Value>, stage_id: &str) -> Result<(), String> {
    let decision = require_enum(gate, "decision", &["pass", "return", "stop"])?;
    validate_evidence_array(gate, "evidence")?;
    let checks = require_obj(gate, "checks")?;
    for key in ["completeness", "quality", "risk", "research", "red_team"] {
        require_bool(checks, key, true)?;
    }
    let readiness = require_bool_value(checks, "readiness")?;

    match decision {
        "pass" => {
            if !readiness {
                return Err(format!(
                    "workflow attestation guard: pass gate without readiness: {stage_id}"
                ));
            }
            if gate.get("rollback_stage").is_some_and(|v| !v.is_null()) {
                return Err(
                    "workflow attestation guard: pass gate must not declare rollback_stage"
                        .to_string(),
                );
            }
            require_empty_array(gate, "stop_reasons")?;
        }
        "return" => {
            let rollback = require_non_empty_str(gate, "rollback_stage")?;
            require_enum_value(rollback, &REQUIRED_STAGES, "rollback_stage")?;
        }
        "stop" => {
            require_non_empty_array(gate, "stop_reasons")?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
