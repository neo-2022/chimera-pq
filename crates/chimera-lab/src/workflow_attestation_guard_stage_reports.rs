use super::support::*;
use serde_json::{Map, Value};

pub(crate) fn validate_stage_specific_report(
    report: &Map<String, Value>,
    stage: &str,
) -> Result<(), String> {
    match stage {
        "analyze" => validate_analyze_report(report),
        "impact_analysis" => validate_impact_report(report),
        "research_planning" => validate_research_plan(report),
        "research" => validate_research_report(report),
        "architecture_synthesis" => validate_architecture_report(report),
        "implementation" => validate_implementation_report(report),
        "validation" => validate_validation_report(report),
        "postmortem" => validate_postmortem_report(report),
        _ => Err(format!(
            "workflow attestation guard: unknown stage: {stage}"
        )),
    }
}

fn validate_analyze_report(report: &Map<String, Value>) -> Result<(), String> {
    for key in [
        "original_task",
        "fundamental_problem",
        "goals",
        "dependencies",
        "contracts",
        "technical_debt",
        "open_questions",
        "missing_information",
        "research_questions",
        "user_clarification_questions",
        "confidence_by_conclusion",
    ] {
        validate_named_non_empty_array(report, key)?;
    }
    Ok(())
}

fn validate_impact_report(report: &Map<String, Value>) -> Result<(), String> {
    for key in [
        "proposed_changes",
        "impact_scope",
        "affected_components",
        "interfaces",
        "contracts",
        "dependencies",
        "invariants",
        "backward_compatibility",
        "cascading_consequences",
        "tradeoffs",
        "risk_register",
        "risk_minimization",
        "impact_minimization",
        "open_questions",
        "next_stage_recommendations",
    ] {
        validate_named_non_empty_array(report, key)?;
    }
    Ok(())
}

fn validate_research_plan(report: &Map<String, Value>) -> Result<(), String> {
    for key in [
        "research_goals",
        "research_questions",
        "research_directions",
        "council_composition",
        "source_classes",
        "expected_disciplines",
        "hypotheses",
        "assumptions_to_verify",
        "success_criteria",
        "completion_criteria",
        "repeat_cycle_policy",
        "evaluation_criteria",
        "inputs_from_analyze",
        "inputs_from_impact",
        "known_unknowns",
    ] {
        validate_named_non_empty_array(report, key)?;
    }
    require_bool(report, "research_may_start_only_after_plan", true)?;
    require_min_array_len(report, "source_classes", 2)?;
    require_min_array_len(report, "expected_disciplines", 3)?;
    Ok(())
}

fn validate_research_report(report: &Map<String, Value>) -> Result<(), String> {
    for key in [
        "researched_directions",
        "solution_families",
        "researched_disciplines",
        "researched_sources",
        "fundamental_principles",
        "knowledge_transfer",
        "invariants",
        "opposite_architectures",
        "simplification_results",
        "analogical_thinking",
        "rejected_solutions",
        "rejected_irrelevant_analogies",
        "analogy_rejection_reasons",
        "council_interdisciplinary_critique",
        "open_questions",
        "additional_research_directions",
        "confidence_levels",
    ] {
        validate_named_non_empty_array(report, key)?;
    }
    for key in [
        "found_theories",
        "found_models",
        "found_patterns",
        "found_architectural_alternatives",
        "comparative_analysis",
        "rejection_reasons_per_solution",
    ] {
        validate_named_non_empty_array(report, key)?;
    }
    require_min_array_len(report, "solution_families", 2)?;
    require_min_array_len(report, "researched_sources", 2)?;
    require_min_array_len(report, "researched_disciplines", 3)?;
    validate_transfer_principles(report)?;
    Ok(())
}

fn validate_transfer_principles(report: &Map<String, Value>) -> Result<(), String> {
    let mut transfer_disciplines = std::collections::BTreeSet::new();
    for value in require_non_empty_array(report, "transfer_principles")? {
        let obj = value_obj(value, "transfer principle")?;
        let discipline = require_non_empty_str(obj, "discipline")?;
        transfer_disciplines.insert(discipline.to_string());
        require_non_empty_str(obj, "principle")?;
        for key in [
            "properties",
            "invariants",
            "constraints",
            "transferable_parts",
            "non_transferable_parts",
            "benefits",
            "risks",
            "tradeoffs",
        ] {
            validate_structured_claim_array(obj, key)?;
        }
        validate_evidence_array(obj, "evidence")?;
    }
    for discipline in require_non_empty_array(report, "researched_disciplines")? {
        let Some(text) = discipline.as_str() else {
            return Err(
                "workflow attestation guard: researched_disciplines item is not string".to_string(),
            );
        };
        if text == "software_engineering" {
            continue;
        }
        if !transfer_disciplines.contains(text) {
            return Err(format!(
                "workflow attestation guard: missing transfer principle for discipline: {text}"
            ));
        }
    }
    Ok(())
}

fn validate_structured_claim_array(obj: &Map<String, Value>, key: &str) -> Result<(), String> {
    for value in require_non_empty_array(obj, key)? {
        let item = value_obj(value, key)?;
        let claim = require_non_empty_str(item, "claim")?;
        if claim.len() < 12 {
            return Err(format!(
                "workflow attestation guard: weak structured claim in {key}"
            ));
        }
        validate_evidence_array(item, "evidence")?;
    }
    Ok(())
}

fn validate_architecture_report(report: &Map<String, Value>) -> Result<(), String> {
    for key in [
        "solution_space",
        "synthesized_variants",
        "invariant_checks",
        "criteria_matrix",
        "opposite_architecture",
        "simplification_review",
        "tradeoff_analysis",
        "self_verification",
        "readiness_criterion",
        "architecture_report",
        "architect_decision",
        "council_architecture_verification",
        "fundamental_principles",
        "found_risks",
    ] {
        validate_named_non_empty_array(report, key)?;
    }
    for key in [
        "final_architecture_description",
        "architectural_invariants",
        "architectural_contracts",
        "accepted_engineering_decisions",
        "considered_alternatives",
        "choice_reasons",
        "rejected_solution_reasons",
        "accepted_tradeoffs",
        "implementation_recommendations",
    ] {
        validate_named_non_empty_array(report, key)?;
    }
    require_min_array_len(report, "synthesized_variants", 2)?;
    Ok(())
}

fn validate_implementation_report(report: &Map<String, Value>) -> Result<(), String> {
    for key in [
        "implementation_plan",
        "minimal_impact_review",
        "invariant_monitoring",
        "deviation_log",
        "new_knowledge_log",
        "rollback_policy",
        "phase_reports",
        "change_checks",
        "implementation_report",
        "architect_decision",
        "council_engineering_verification",
    ] {
        validate_named_non_empty_array(report, key)?;
    }
    for key in [
        "implemented_changes",
        "changed_components",
        "changed_interfaces",
        "changed_contracts",
        "discovered_deviations",
        "new_engineering_knowledge",
        "identified_risks",
        "unresolved_questions",
        "technical_debts",
        "recommendations",
    ] {
        validate_named_non_empty_array(report, key)?;
    }
    require_bool(report, "direct_architecture_change_forbidden", true)?;
    Ok(())
}

fn validate_validation_report(report: &Map<String, Value>) -> Result<(), String> {
    for key in [
        "requirements_verification",
        "architecture_verification",
        "functional_verification",
        "non_functional_verification",
        "stress_testing",
        "reproducibility_check",
        "undiscovered_problem_search",
        "defects",
        "risks",
        "remaining_constraints",
        "known_technical_debt",
        "recommendations",
    ] {
        validate_named_non_empty_array(report, key)?;
    }
    require_bool(report, "independent_assessment_ready", true)?;
    require_bool(report, "does_not_hide_limitations", true)?;
    require_bool(report, "does_not_fix_errors", true)?;
    Ok(())
}

fn validate_postmortem_report(report: &Map<String, Value>) -> Result<(), String> {
    for key in [
        "decision_analysis",
        "error_analysis",
        "invariant_analysis",
        "research_analysis",
        "process_analysis",
        "engineering_knowledge",
        "research_debt_update",
        "architect_self_assessment",
        "knowledge_reuse",
        "postmortem_council_review",
        "postmortem_red_team",
        "next_lifecycle_recommendations",
    ] {
        validate_named_non_empty_array(report, key)?;
    }
    for key in [
        "final_project_assessment",
        "confirmed_engineering_decisions",
        "erroneous_decisions",
        "successful_decision_reasons",
        "erroneous_decision_reasons",
        "confirmed_invariants",
        "new_invariants",
        "confirmed_fundamental_principles",
        "confirmed_architectural_patterns",
        "identified_engineering_lessons",
        "future_recommendations",
        "open_questions",
        "updated_research_debt",
    ] {
        validate_named_non_empty_array(report, key)?;
    }
    require_bool(report, "knowledge_preserved", true)?;
    require_bool(report, "research_debt_preserved", true)?;
    Ok(())
}
