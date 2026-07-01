use super::coverage::validate_interdisciplinary_source_lists_checked;
use super::support::*;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub(crate) const REQUIRED_ROLES: [&str; 6] = [
    "architect",
    "senior_developer",
    "tester",
    "security_engineer",
    "devops_engineer",
    "critic_skeptic",
];

pub(crate) fn validate_interdisciplinary_research(
    research: &Map<String, Value>,
    done_like: bool,
) -> Result<(), String> {
    require_bool(research, "required", true)?;
    let planned = require_bool_value(research, "planned")?;
    let performed = require_bool_value(research, "performed")?;
    let council_used = require_bool_value(research, "council_used_for_critique")?;
    let knowledge_transfer = require_bool_value(research, "knowledge_transfer_checked")?;
    require_bool(research, "software_only", false)?;
    validate_evidence_array(research, "evidence")?;

    let disciplines = require_non_empty_array(research, "disciplines")?;
    validate_any_other_disciplines(require_obj(research, "any_other_disciplines")?)?;
    validate_discipline_matrix(require_non_empty_array(research, "discipline_matrix")?)?;
    validate_interdisciplinary_source_lists_checked(research)?;
    let mut non_software_disciplines = BTreeSet::new();
    for discipline in disciplines {
        let value = discipline.as_str().ok_or_else(|| {
            "workflow attestation guard: interdisciplinary discipline is not string".to_string()
        })?;
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(
                "workflow attestation guard: empty interdisciplinary discipline".to_string(),
            );
        }
        if trimmed != "software_engineering" {
            non_software_disciplines.insert(trimmed);
        }
    }
    if non_software_disciplines.len() < 3 {
        return Err(
            "workflow attestation guard: interdisciplinary research needs at least three non-software disciplines"
                .to_string(),
        );
    }

    if done_like && !(planned && performed && council_used && knowledge_transfer) {
        return Err(
            "workflow attestation guard: pass/done requires interdisciplinary research in council critique"
                .to_string(),
        );
    }
    Ok(())
}

pub(crate) fn validate_council(
    council: &Map<String, Value>,
    done_like: bool,
) -> Result<(), String> {
    let used_subagents = require_bool_value(council, "real_subagents_used")?;
    let independent = require_bool_value(council, "independent_reports_published")?;
    let cross = require_bool_value(council, "cross_analysis_done")?;
    let red_team = require_bool_value(council, "red_team_done")?;
    let consensus = require_enum(
        council,
        "consensus",
        &[
            "accepted",
            "accepted_with_constraints",
            "blocked",
            "not_agreed",
        ],
    )?;
    if done_like && !(used_subagents && independent && cross && red_team) {
        return Err(
            "workflow attestation guard: pass/done requires real council and red-team review"
                .to_string(),
        );
    }
    if done_like {
        match consensus {
            "accepted" => {}
            "accepted_with_constraints" => validate_resolved_consensus_constraints(council)?,
            "blocked" | "not_agreed" => {
                return Err(
                    "workflow attestation guard: pass/done cannot use blocked or not_agreed consensus"
                        .to_string(),
                );
            }
            _ => unreachable!(),
        }
    }

    let participants = require_array(council, "participants")?;
    if done_like {
        let mut roles = BTreeSet::new();
        for participant in participants {
            let obj = value_obj(participant, "council participant")?;
            let role = require_enum(obj, "role", &REQUIRED_ROLES)?;
            require_non_empty_str(obj, "agent_id")?;
            require_str(obj, "status", "completed")?;
            roles.insert(role);
        }
        for role in REQUIRED_ROLES {
            if !roles.contains(role) {
                return Err(format!(
                    "workflow attestation guard: missing council role: {role}"
                ));
            }
        }
    }

    let blockers = require_array(council, "open_blockers")?;
    if done_like && !blockers.is_empty() {
        return Err("workflow attestation guard: pass/done with open council blockers".to_string());
    }

    if done_like {
        validate_council_detail(require_obj(council, "detail")?, done_like)?;
    }
    Ok(())
}

pub(crate) fn validate_council_stage_report(council: &Map<String, Value>) -> Result<(), String> {
    require_bool(council, "independent_work_done", true)?;
    require_bool(council, "publication_barrier_respected", true)?;
    require_bool(council, "cross_analysis_done", true)?;
    require_bool(council, "red_team_done", true)?;
    require_bool(council, "additional_research_loop_checked", true)?;
    require_bool(council, "completion_criteria_met", true)?;
    validate_evidence_array(council, "evidence")?;
    validate_named_non_empty_array(council, "participants")?;
    validate_named_non_empty_array(council, "accepted_recommendations")?;
    validate_named_array(council, "rejected_recommendations")?;
    validate_named_array(council, "open_questions")
}

fn validate_any_other_disciplines(obj: &Map<String, Value>) -> Result<(), String> {
    require_bool(obj, "source_clause_preserved", true)?;
    require_non_empty_str(obj, "selection_rule")?;
    require_non_empty_str(obj, "non_applicability_rule")?;
    validate_evidence_array(obj, "evidence")
}

fn validate_discipline_matrix(items: &[Value]) -> Result<(), String> {
    if items.len() < 36 {
        return Err("workflow attestation guard: discipline_matrix is too small".to_string());
    }
    for item in items {
        let obj = value_obj(item, "discipline matrix item")?;
        require_non_empty_str(obj, "discipline")?;
        require_enum(obj, "decision", &["selected", "rejected"])?;
        require_non_empty_str(obj, "result")?;
        validate_evidence_array(obj, "evidence")?;
        if obj.get("decision").and_then(Value::as_str) == Some("rejected") {
            require_non_empty_str(obj, "non_applicability_reason")?;
        }
    }
    Ok(())
}

fn validate_resolved_consensus_constraints(council: &Map<String, Value>) -> Result<(), String> {
    for item in require_non_empty_array(council, "resolved_consensus_constraints")? {
        let obj = value_obj(item, "resolved consensus constraint")?;
        require_non_empty_str(obj, "constraint")?;
        require_bool(obj, "resolved", true)?;
        validate_evidence_array(obj, "evidence")?;
    }
    Ok(())
}

fn validate_council_detail(detail: &Map<String, Value>, done_like: bool) -> Result<(), String> {
    if !done_like {
        return Ok(());
    }
    require_bool(detail, "assignments_unique", true)?;
    require_bool(detail, "minimal_context_given", true)?;
    require_bool(detail, "independent_work_before_cross_reading", true)?;
    require_bool(detail, "publication_barrier_respected", true)?;
    require_bool(detail, "each_expert_researched", true)?;
    require_bool(
        detail,
        "interdisciplinary_research_used_by_each_expert",
        true,
    )?;
    require_bool(detail, "cross_analysis_done", true)?;
    require_bool(detail, "red_team_done", true)?;
    require_bool(detail, "additional_research_loop_done", true)?;
    require_bool(detail, "review_cycles_completed", true)?;
    require_bool(detail, "council_report_formed", true)?;
    validate_evidence_array(detail, "evidence")?;

    validate_individual_reports(require_array(detail, "individual_reports")?)?;
    validate_named_non_empty_array(detail, "cross_analysis_findings")?;
    validate_named_non_empty_array(detail, "red_team_findings")?;
    validate_named_non_empty_array(detail, "additional_research_checks")?;
    for key in [
        "problem_description",
        "participant_list",
        "responsibility_areas",
        "researched_architectures",
        "engineering_alternatives",
        "found_fundamental_principles",
        "found_invariants",
        "found_constraints",
        "found_engineering_risks",
        "red_team_results",
        "additional_research_results",
        "rejected_options",
        "rejection_reasons",
        "remaining_open_questions",
        "council_recommendations",
    ] {
        validate_named_non_empty_array(detail, key)?;
    }
    validate_named_non_empty_array(detail, "completion_criteria")
}

fn validate_individual_reports(reports: &[Value]) -> Result<(), String> {
    if reports.len() < REQUIRED_ROLES.len() {
        return Err("workflow attestation guard: missing individual council reports".to_string());
    }
    let mut roles = BTreeSet::new();
    for report in reports {
        let obj = value_obj(report, "individual council report")?;
        let role = require_enum(obj, "role", &REQUIRED_ROLES)?;
        require_non_empty_str(obj, "responsibility")?;
        require_non_empty_str(obj, "context_scope")?;
        require_non_empty_str(obj, "research_task")?;
        require_non_empty_str(obj, "confidence")?;
        validate_evidence_array(obj, "evidence")?;
        validate_named_non_empty_array(obj, "sources")?;
        validate_named_non_empty_array(obj, "findings")?;
        validate_named_non_empty_array(obj, "interdisciplinary_findings")?;
        validate_named_non_empty_array(obj, "disciplines_considered")?;
        validate_named_non_empty_array(obj, "fundamental_principles")?;
        validate_named_non_empty_array(obj, "knowledge_transfer_checks")?;
        validate_named_non_empty_array(obj, "transfer_risks")?;
        validate_named_non_empty_array(obj, "tradeoffs")?;
        validate_named_non_empty_array(obj, "rejected_analogies")?;
        for key in [
            "research_results",
            "engineering_solutions",
            "found_invariants",
            "found_alternative_architectures",
            "found_risks",
            "found_constraints",
            "improvement_proposals",
            "additional_research_questions",
            "confidence_by_conclusion",
        ] {
            validate_named_non_empty_array(obj, key)?;
        }
        roles.insert(role);
    }
    for role in REQUIRED_ROLES {
        if !roles.contains(role) {
            return Err(format!(
                "workflow attestation guard: missing individual report for role: {role}"
            ));
        }
    }
    Ok(())
}
