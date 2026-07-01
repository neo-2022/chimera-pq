use super::support::*;
use serde_json::{Map, Value};

const REQUIRED_FINAL_DONE_CONDITIONS: [&str; 10] = [
    "all_stages_reported",
    "all_gates_passed",
    "council_consensus_resolved",
    "red_team_processed",
    "interdisciplinary_research_proven",
    "accepted_decisions_recorded",
    "rejected_decisions_recorded",
    "open_questions_recorded",
    "research_debt_updated",
    "truth_boundary_preserved",
];

pub(crate) fn validate_final_decision(
    final_decision: &Map<String, Value>,
    status: &str,
    task_done: bool,
    state: &str,
) -> Result<(), String> {
    let decision_log_updated = require_bool_value(final_decision, "decision_log_updated")?;
    let postmortem_done = require_bool_value(final_decision, "postmortem_done")?;
    let research_debt_updated = require_bool_value(final_decision, "research_debt_updated")?;
    let final_done_conditions_met = final_decision
        .get("final_done_conditions_met")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if status == "pass" || task_done || state == "done" {
        validate_named_non_empty_array(final_decision, "final_done_evidence")?;
        validate_final_done_conditions(final_decision)?;
        validate_final_done_checklist(final_decision)?;
    }
    if (status == "pass" || task_done || state == "done")
        && (state != "done"
            || !postmortem_done
            || !research_debt_updated
            || !decision_log_updated
            || !final_done_conditions_met)
    {
        return Err(
            "workflow attestation guard: done/pass requires final done conditions, postmortem and research debt update"
                .to_string(),
        );
    }
    if status == "stop" && state == "done" {
        return Err(
            "workflow attestation guard: stop status cannot have done final state".to_string(),
        );
    }
    Ok(())
}

fn validate_final_done_checklist(final_decision: &Map<String, Value>) -> Result<(), String> {
    const REQUIRED: [&str; 10] = [
        "council_review_completed",
        "red_team_mode_completed",
        "council_report_formed",
        "universal_gate_passed",
        "final_postmortem_report_formed",
        "confirmed_engineering_knowledge_preserved",
        "confirmed_fundamental_principles_preserved",
        "new_architectural_invariants_preserved",
        "updated_research_debt_preserved",
        "next_lifecycle_recommendations_formed",
    ];
    let items = require_array(final_decision, "final_done_checklist")?;
    if items.len() != REQUIRED.len() {
        return Err("workflow attestation guard: final_done_checklist count mismatch".to_string());
    }
    for (idx, expected) in REQUIRED.iter().enumerate() {
        let item = value_obj(&items[idx], "final done checklist item")?;
        require_str(item, "id", expected)?;
        require_bool(item, "met", true)?;
        validate_evidence_array(item, "evidence")?;
    }
    Ok(())
}

fn validate_final_done_conditions(final_decision: &Map<String, Value>) -> Result<(), String> {
    let conditions = require_array(final_decision, "final_done_conditions")?;
    if conditions.len() != REQUIRED_FINAL_DONE_CONDITIONS.len() {
        return Err("workflow attestation guard: final done condition count mismatch".to_string());
    }
    for (idx, expected) in REQUIRED_FINAL_DONE_CONDITIONS.iter().enumerate() {
        let condition = value_obj(&conditions[idx], "final done condition")?;
        require_str(condition, "id", expected)?;
        require_bool(condition, "met", true)?;
        validate_evidence_array(condition, "evidence")?;
    }
    Ok(())
}
