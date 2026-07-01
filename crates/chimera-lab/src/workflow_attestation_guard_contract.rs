use super::source_lists::SOURCE_SHA256;
use super::support::*;
use serde_json::{Map, Value};

const REQUIRED_NORMATIVE_SPANS: [(&str, i64, i64); 9] = [
    ("council_interdisciplinary_research", 779, 936),
    ("council_knowledge_transfer", 937, 962),
    ("council_analogical_thinking", 1025, 1062),
    ("research_planning_interdisciplinary_search", 2829, 2854),
    ("stage_research_interdisciplinary_research", 3119, 3158),
    ("stage_research_mandatory_other_disciplines", 3159, 3230),
    ("stage_research_fundamental_principles", 3231, 3304),
    ("stage_research_knowledge_transfer", 3305, 3328),
    (
        "stage_research_analogical_thinking_and_evaluation",
        3405,
        3508,
    ),
];

pub(crate) fn validate_workline_binding(
    root: &Map<String, Value>,
    task_id: &str,
) -> Result<(), String> {
    let workline_id = require_non_empty_str(root, "workline_id")?;
    if workline_id != task_id {
        return Err("workflow attestation guard: workline_id must match task_id".to_string());
    }
    let updated_at = require_non_empty_str(root, "updated_at_utc")?;
    if !(updated_at.len() == 20
        && updated_at.ends_with('Z')
        && updated_at.as_bytes()[4] == b'-'
        && updated_at.as_bytes()[7] == b'-'
        && updated_at.as_bytes()[10] == b'T'
        && updated_at.as_bytes()[13] == b':'
        && updated_at.as_bytes()[16] == b':')
    {
        return Err(
            "workflow attestation guard: updated_at_utc must use YYYY-MM-DDTHH:MM:SSZ".to_string(),
        );
    }
    let handoff_id = require_non_empty_str(root, "handoff_id")?;
    if !handoff_id.starts_with("MESH_SESSION_HANDOFF_") {
        return Err(
            "workflow attestation guard: handoff_id must reference mesh handoff".to_string(),
        );
    }
    require_str(
        root,
        "source_coverage_ref",
        "docs/AI_ARCHITECT_ALGORITHM_COVERAGE.json#abdbad3e02da3a7a",
    )
}

pub(crate) fn validate_normative_requirement_coverage(
    coverage: &Map<String, Value>,
) -> Result<(), String> {
    require_str(coverage, "source_sha256", SOURCE_SHA256)?;
    require_bool(
        coverage,
        "line_spans_cover_required_interdisciplinary_content",
        true,
    )?;
    require_bool(coverage, "stage_reports_must_prove_each_span", true)?;
    validate_evidence_array(coverage, "evidence")?;

    let spans = require_array(coverage, "spans")?;
    if spans.len() != REQUIRED_NORMATIVE_SPANS.len() {
        return Err("workflow attestation guard: normative span count mismatch".to_string());
    }
    for (idx, (expected_id, expected_start, expected_end)) in
        REQUIRED_NORMATIVE_SPANS.iter().enumerate()
    {
        let span = value_obj(&spans[idx], "normative span")?;
        require_str(span, "id", expected_id)?;
        require_i64(span, "source_line_start", *expected_start)?;
        require_i64(span, "source_line_end", *expected_end)?;
        require_non_empty_str(span, "required_proof")?;
        validate_evidence_array(span, "evidence")?;
    }
    Ok(())
}

pub(crate) fn validate_detailed_algorithm_contract(
    contract: &Map<String, Value>,
) -> Result<(), String> {
    require_enum(
        contract,
        "current_state",
        &[
            "planning",
            "researching",
            "reviewing",
            "implementing",
            "validating",
            "done",
        ],
    )?;
    require_bool(contract, "conclusion_classification_required", true)?;
    validate_evidence_array(contract, "evidence")?;

    let stage_contract = require_obj(contract, "stage_report_contract")?;
    for key in [
        "analyze_required_fields",
        "impact_analysis_required_fields",
        "research_planning_required_fields",
        "research_required_fields",
        "architecture_synthesis_required_fields",
        "implementation_required_fields",
        "validation_required_fields",
        "postmortem_required_fields",
    ] {
        require_min_string_array(stage_contract, key, 8)?;
    }
    require_string_array_contains_all(
        stage_contract,
        "research_required_fields",
        &[
            "found_theories",
            "found_models",
            "found_patterns",
            "found_architectural_alternatives",
            "comparative_analysis",
            "rejection_reasons_per_solution",
        ],
    )?;
    require_string_array_contains_all(
        stage_contract,
        "architecture_synthesis_required_fields",
        &[
            "final_architecture_description",
            "architectural_invariants",
            "architectural_contracts",
            "accepted_engineering_decisions",
            "considered_alternatives",
            "choice_reasons",
            "rejected_solution_reasons",
            "accepted_tradeoffs",
            "implementation_recommendations",
        ],
    )?;
    require_string_array_contains_all(
        stage_contract,
        "implementation_required_fields",
        &[
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
        ],
    )?;
    require_string_array_contains_all(
        stage_contract,
        "postmortem_required_fields",
        &[
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
        ],
    )?;

    validate_research_contract(require_obj(contract, "research_contract")?)?;
    validate_council_contract(require_obj(contract, "council_contract")?)?;
    validate_gate_contract(require_obj(contract, "gate_contract")?)?;

    require_min_string_array(contract, "implementation_preconditions", 7)?;
    require_min_string_array(contract, "validation_done_conditions", 4)?;
    require_min_string_array(contract, "final_done_conditions", 10)?;

    let research_debt = require_obj(contract, "research_debt_snapshot")?;
    require_array(research_debt, "items")?;
    require_non_empty_str(research_debt, "no_open_research_debt_reason")?;
    Ok(())
}

fn validate_research_contract(research: &Map<String, Value>) -> Result<(), String> {
    require_min_i64(research, "known_solution_families_min", 2)?;
    require_min_i64(research, "source_classes_min", 2)?;
    for key in [
        "requires_interdisciplinary_search",
        "requires_knowledge_transfer",
        "requires_opposite_architecture",
        "requires_simplification_search",
        "requires_confidence_levels",
        "requires_rejected_irrelevant_analogies",
        "requires_transfer_principle_properties",
        "requires_per_role_interdisciplinary_proof",
        "repeat_cycle_policy_required",
    ] {
        require_bool(research, key, true)?;
    }
    Ok(())
}

fn validate_council_contract(council: &Map<String, Value>) -> Result<(), String> {
    for key in [
        "individual_reports_required",
        "publication_barrier_required",
        "independent_work_before_cross_reading_required",
        "cross_analysis_required",
        "red_team_required",
        "additional_research_loop_required",
        "review_cycles_required",
    ] {
        require_bool(council, key, true)?;
    }
    require_min_string_array(council, "council_report_required_fields", 8)?;
    require_string_array_contains_all(
        council,
        "council_report_required_fields",
        &[
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
        ],
    )
}

fn validate_gate_contract(gate: &Map<String, Value>) -> Result<(), String> {
    require_min_string_array(gate, "gate_report_required_fields", 8)?;
    require_min_string_array(gate, "research_check_required_fields", 8)?;
    require_bool(gate, "return_requires_earliest_rollback_analysis", true)?;
    require_bool(gate, "gate_does_not_fix_or_replace_council", true)
}
