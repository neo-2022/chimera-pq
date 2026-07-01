use serde_json::{Map, Value};

#[path = "workflow_attestation_guard_source_lists.rs"]
mod source_lists;
#[path = "workflow_attestation_guard_support.rs"]
mod support;
use support::*;
#[path = "workflow_attestation_guard_redaction.rs"]
mod redaction;
use redaction::reject_sensitive_text;

#[path = "workflow_attestation_guard_contract.rs"]
mod contract;
use contract::{
    validate_detailed_algorithm_contract, validate_normative_requirement_coverage,
    validate_workline_binding,
};

#[path = "workflow_attestation_guard_coverage.rs"]
mod coverage;
use coverage::{
    validate_algorithm_coverage, validate_interdisciplinary_source_lists_catalog,
    validate_source_text_coverage,
};

#[path = "workflow_attestation_guard_council.rs"]
mod council;
use council::{validate_council, validate_interdisciplinary_research};

#[path = "workflow_attestation_guard_final.rs"]
mod final_guard;
use final_guard::validate_final_decision;

#[path = "workflow_attestation_guard_stage_reports.rs"]
mod stage_reports;

#[path = "workflow_attestation_guard_stage_flow.rs"]
mod stage_flow;
use stage_flow::{
    validate_detailed_stage_reports, validate_full_stage_sequence, validate_rollback_events,
    validate_stages,
};

pub fn validate_file(path: &str) -> Result<(), String> {
    let root = read_obj(path)?;
    validate_root(&root)
}

fn validate_root(root: &Map<String, Value>) -> Result<(), String> {
    require_str(root, "kind", "workflow_attestation")?;
    require_i64(root, "schema_version", 1)?;
    let status = require_enum(
        root,
        "status",
        &["pass", "return", "stop", "partial", "blocked"],
    )?;
    let task_id = require_non_empty_str(root, "task_id")?;
    require_non_empty_str(root, "scope")?;
    require_str(
        root,
        "algorithm_source",
        "docs/AI_ARCHITECT_LIFECYCLE_GUARD.md",
    )?;
    validate_workline_binding(root, task_id)?;
    reject_sensitive_text(root)?;
    validate_forbidden_narrative_claims(root)?;

    let truth = require_obj(root, "truth_boundary")?;
    require_bool(truth, "local_runtime_started", false)?;
    require_str(truth, "network_state", "not_modified")?;
    let lab_scope_only = require_bool_value(truth, "lab_scope_only")?;
    let real_world_closed = require_bool_value(truth, "real_world_datapath_closed")?;

    let claims = require_obj(root, "claims")?;
    let task_done = require_bool_value(claims, "task_done")?;
    let prod_ready = require_bool_value(claims, "prod_ready")?;
    let real_world_pass = require_bool_value(claims, "real_world_pass")?;
    if lab_scope_only && (real_world_pass || prod_ready) {
        return Err(
            "workflow attestation guard: lab scope promoted to real-world/prod-ready".to_string(),
        );
    }
    if real_world_pass && !real_world_closed {
        return Err(
            "workflow attestation guard: real_world_pass without real_world_datapath_closed"
                .to_string(),
        );
    }

    let final_decision = require_obj(root, "final_decision")?;
    let final_state = require_enum(
        final_decision,
        "state",
        &[
            "done", "partial", "blocked", "stopped", "returned", "not_done",
        ],
    )?;
    let done_like = status == "pass" || task_done || final_state == "done";
    if final_state == "done" && (status != "pass" || !task_done) {
        return Err(
            "workflow attestation guard: final done requires status=pass and task_done=true"
                .to_string(),
        );
    }

    let coverage_items = validate_algorithm_coverage(require_obj(root, "algorithm_coverage")?)?;
    validate_source_text_coverage(root)?;
    validate_interdisciplinary_source_lists_catalog(root)?;
    validate_normative_requirement_coverage(require_obj(root, "normative_requirement_coverage")?)?;
    validate_detailed_algorithm_contract(require_obj(root, "detailed_algorithm_contract")?)?;
    validate_interdisciplinary_research(
        require_obj(root, "interdisciplinary_research")?,
        done_like,
    )?;
    validate_council(require_obj(root, "council")?, done_like)?;

    let stages = require_array(root, "stages")?;
    if done_like {
        validate_full_stage_sequence(stages)?;
    } else if stages.is_empty() {
        return Err(
            "workflow attestation guard: non-pass attestation still needs at least one stage"
                .to_string(),
        );
    }
    validate_stages(stages, done_like)?;
    validate_detailed_stage_reports(root, stages, done_like, &coverage_items)?;

    validate_final_decision(final_decision, status, task_done, final_state)?;
    validate_rollback_events(root)
}

fn validate_forbidden_narrative_claims(root: &Map<String, Value>) -> Result<(), String> {
    validate_forbidden_narrative_value(&Value::Object(root.clone()), "")
}

fn validate_forbidden_narrative_value(value: &Value, path: &str) -> Result<(), String> {
    match value {
        Value::String(text) => {
            if !is_claim_enum_path(path) {
                let lower = text.to_ascii_lowercase();
                for banned in [
                    "prod-ready",
                    "prod ready",
                    "production ready",
                    "real-world pass",
                    "real world pass",
                    "runtime pass",
                    "ship pass",
                    "stage closed",
                    "milestone closed",
                ] {
                    if lower.contains(banned) {
                        return Err(format!(
                            "workflow attestation guard: forbidden narrative claim at {path}: {banned}"
                        ));
                    }
                }
            }
            Ok(())
        }
        Value::Array(items) => {
            for (idx, item) in items.iter().enumerate() {
                validate_forbidden_narrative_value(item, &format!("{path}[{idx}]"))?;
            }
            Ok(())
        }
        Value::Object(obj) => {
            for (key, item) in obj {
                let child = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                validate_forbidden_narrative_value(item, &child)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn is_claim_enum_path(path: &str) -> bool {
    path == "status"
        || path == "final_decision.state"
        || path == "council.consensus"
        || path.ends_with(".gate.decision")
        || path.ends_with(".gate_report.decision")
}

#[cfg(test)]
#[path = "workflow_attestation_guard_tests.rs"]
mod tests;
