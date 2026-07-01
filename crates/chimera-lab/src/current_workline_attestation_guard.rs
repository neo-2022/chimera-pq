use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[path = "workflow_attestation_guard_redaction.rs"]
mod redaction;
use redaction::reject_sensitive_text;

#[path = "workflow_attestation_guard_support.rs"]
#[allow(dead_code)]
mod support;
use support::*;

const REQUIRED_ROLES: [&str; 6] = [
    "architect",
    "senior_developer",
    "tester",
    "security_engineer",
    "devops_engineer",
    "critic_skeptic",
];

const REQUIRED_STAGE_ORDER: [&str; 8] = [
    "ANALYZE",
    "IMPACT_ANALYSIS",
    "RESEARCH_PLANNING",
    "RESEARCH",
    "ARCHITECTURE_SYNTHESIS",
    "IMPLEMENTATION",
    "VALIDATION",
    "POSTMORTEM",
];

const REQUIRED_INTERDISCIPLINARY_SOURCE_LISTS: [&str; 7] = [
    "council_interdisciplinary_disciplines",
    "council_fundamental_knowledge_items",
    "stage_research_disciplines",
    "stage_research_fundamental_items",
    "knowledge_transfer_checks",
    "analogical_thinking_questions",
    "found_idea_evaluation_criteria",
];

pub fn validate_file(path: &str) -> Result<(), String> {
    let root = read_obj(path)?;
    validate_root(&root)
}

fn validate_root(root: &Map<String, Value>) -> Result<(), String> {
    require_str(root, "kind", "current_workline_attestation")?;
    require_i64(root, "schema_version", 1)?;
    let status = require_enum(root, "status", &["pass", "partial", "blocked", "not_done"])?;
    if status != "pass" {
        return Err(
            "current workline attestation guard: current workline status must be pass".to_string(),
        );
    }
    require_bool(root, "canonical_only_not_sufficient", true)?;
    require_bool(root, "requires_current_workline_attestation", true)?;
    require_bool(root, "requires_real_subagent_trace", true)?;
    require_bool(root, "requires_interdisciplinary_trace", true)?;
    reject_sensitive_current_workline_text(root)?;

    let current_workline_id = require_non_empty_str(root, "current_workline_id")?;
    validate_slug(current_workline_id, "current_workline_id")?;
    validate_timestamp(
        require_non_empty_str(root, "updated_at_utc")?,
        "updated_at_utc",
    )?;

    validate_path_field(
        root,
        "canonical_lifecycle_attestation",
        "docs/WORKFLOW_ATTESTATION.json",
    )?;
    validate_workline_artifact(root, current_workline_id)?;
    validate_latest_handoff(root)?;
    validate_truth_boundary(require_obj(root, "truth_boundary")?)?;
    validate_stage_trace(require_obj(root, "stage_trace")?)?;
    validate_deepseek_stage_check(require_obj(root, "deepseek_stage_check")?)?;
    validate_interdisciplinary_trace(require_obj(root, "interdisciplinary_trace")?)?;
    let subagent_items = require_array(root, "subagent_execution_log")?;
    validate_subagent_execution_log(subagent_items, status)?;
    validate_subagent_execution_trace_artifact(root, current_workline_id, subagent_items, status)?;
    validate_review_results(require_obj(root, "review_results")?, status)?;
    validate_gap_resolution(require_obj(root, "gap_resolution")?)?;
    Ok(())
}

fn validate_workline_artifact(
    root: &Map<String, Value>,
    current_workline_id: &str,
) -> Result<(), String> {
    let path = require_non_empty_str(root, "workline_attestation")?;
    if !path.starts_with("docs/WORKFLOW_ATTESTATION_") {
        return Err(
            "current workline attestation guard: workline_attestation must be a workline artifact"
                .to_string(),
        );
    }
    validate_path_exists(path, "workline_attestation")?;
    let expected_fragment = current_workline_id.replace('-', "_").to_ascii_uppercase();
    let actual = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_uppercase();
    if !actual.contains(&expected_fragment) {
        return Err(
            "current workline attestation guard: workline artifact is not bound to current_workline_id"
                .to_string(),
        );
    }
    validate_file_sha256(
        path,
        require_non_empty_str(root, "workline_attestation_sha256")?,
    )?;
    Ok(())
}

fn validate_latest_handoff(root: &Map<String, Value>) -> Result<(), String> {
    let path = require_non_empty_str(root, "latest_handoff")?;
    if path.starts_with("tests/fixtures/") {
        validate_path_exists(path, "latest_handoff")?;
        return validate_file_sha256(path, require_non_empty_str(root, "latest_handoff_sha256")?);
    }
    if !path.starts_with("docs/MESH_SESSION_HANDOFF_") {
        return Err(
            "current workline attestation guard: latest_handoff must reference mesh handoff"
                .to_string(),
        );
    }
    validate_path_exists(path, "latest_handoff")?;
    let latest = latest_handoff_path()?;
    if path != latest {
        return Err(
            "current workline attestation guard: latest_handoff is not the newest handoff"
                .to_string(),
        );
    }
    validate_file_sha256(path, require_non_empty_str(root, "latest_handoff_sha256")?)
}

fn latest_handoff_path() -> Result<String, String> {
    let mut paths = Vec::new();
    let docs_dir = project_root().join("docs");
    let entries = fs::read_dir(&docs_dir)
        .map_err(|err| format!("current workline attestation guard: cannot read docs: {err}"))?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            format!("current workline attestation guard: cannot read dir entry: {err}")
        })?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with("MESH_SESSION_HANDOFF_") && name.ends_with(".md") {
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            paths.push((modified, format!("docs/{name}")));
        }
    }
    paths.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    paths
        .pop()
        .map(|(_, path)| path)
        .ok_or_else(|| "current workline attestation guard: no mesh handoff found".to_string())
}

fn validate_truth_boundary(truth: &Map<String, Value>) -> Result<(), String> {
    require_bool(truth, "local_runtime_started", false)?;
    require_str(truth, "network_state", "not_modified")?;
    require_bool(truth, "secrets_redacted", true)?;
    require_bool(truth, "stand_literals_absent", true)
}

fn validate_stage_trace(trace: &Map<String, Value>) -> Result<(), String> {
    require_bool(trace, "required", true)?;
    require_i64(trace, "stage_count", REQUIRED_STAGE_ORDER.len() as i64)?;
    let stages = require_array(trace, "stage_order")?;
    if stages.len() != REQUIRED_STAGE_ORDER.len() {
        return Err("current workline attestation guard: stage_order length mismatch".to_string());
    }
    for (index, expected) in REQUIRED_STAGE_ORDER.iter().enumerate() {
        let actual = stages.get(index).and_then(Value::as_str).ok_or_else(|| {
            "current workline attestation guard: stage_order item is not string".to_string()
        })?;
        if actual != *expected {
            return Err(format!(
                "current workline attestation guard: stage_order mismatch at index {index}"
            ));
        }
    }
    require_bool(trace, "gate_decisions_recorded", true)?;
    require_bool(trace, "rollback_rules_checked", true)?;
    validate_stage_reports(require_array(trace, "stage_reports")?)?;
    validate_evidence_array(trace, "evidence")
}

fn validate_stage_reports(reports: &[Value]) -> Result<(), String> {
    if reports.len() != REQUIRED_STAGE_ORDER.len() {
        return Err(
            "current workline attestation guard: stage_reports length mismatch".to_string(),
        );
    }
    for (index, expected_stage) in REQUIRED_STAGE_ORDER.iter().enumerate() {
        let report = value_obj(&reports[index], "stage report")?;
        require_str(report, "stage", expected_stage)?;
        require_non_empty_str(report, "goal")?;
        require_non_empty_str(report, "outcome")?;
        require_enum(report, "gate_decision", &["pass", "return", "stop"])?;
        require_bool(report, "council_review_recorded", true)?;
        require_bool(report, "red_team_review_recorded", true)?;
        validate_evidence_array(report, "evidence")?;
    }
    Ok(())
}

fn validate_deepseek_stage_check(check: &Map<String, Value>) -> Result<(), String> {
    require_i64(check, "stage_count", 8)?;
    require_i64(check, "covered_required_item_count", 227)?;
    require_bool(check, "all_stage_gates_accounted", true)?;
    require_bool(check, "canonical_workflow_guard_passed", true)?;
    validate_evidence_array(check, "evidence")
}

fn validate_interdisciplinary_trace(trace: &Map<String, Value>) -> Result<(), String> {
    require_bool(trace, "required", true)?;
    require_bool(trace, "used_by_each_subagent", true)?;
    require_bool(trace, "software_only", false)?;
    require_i64(trace, "source_list_count", 7)?;
    require_bool(trace, "knowledge_transfer_checked", true)?;
    require_bool(trace, "rejected_analogies_recorded", true)?;
    validate_interdisciplinary_source_lists(trace)?;
    validate_interdisciplinary_role_checks(require_array(trace, "per_role_checks")?)?;
    validate_evidence_array(trace, "evidence")
}

fn validate_interdisciplinary_source_lists(trace: &Map<String, Value>) -> Result<(), String> {
    let values = require_array(trace, "source_lists_checked")?;
    if values.len() != REQUIRED_INTERDISCIPLINARY_SOURCE_LISTS.len() {
        return Err(
            "current workline attestation guard: source_lists_checked length mismatch".to_string(),
        );
    }
    require_string_array_contains_all(
        trace,
        "source_lists_checked",
        &REQUIRED_INTERDISCIPLINARY_SOURCE_LISTS,
    )
}

fn validate_interdisciplinary_role_checks(items: &[Value]) -> Result<(), String> {
    if items.len() != REQUIRED_ROLES.len() {
        return Err(
            "current workline attestation guard: per_role_checks role count mismatch".to_string(),
        );
    }
    let mut roles = BTreeSet::new();
    for item in items {
        let obj = value_obj(item, "per-role interdisciplinary check")?;
        let role = require_enum(obj, "role", &REQUIRED_ROLES)?;
        if !roles.insert(role.to_string()) {
            return Err(format!(
                "current workline attestation guard: duplicate interdisciplinary role: {role}"
            ));
        }
        require_bool(obj, "interdisciplinary_research_used", true)?;
        require_min_string_array(obj, "disciplines_considered", 3)?;
        require_min_string_array(obj, "fundamental_principles_checked", 2)?;
        require_bool(obj, "knowledge_transfer_checked", true)?;
        require_non_empty_str(obj, "transfer_principle")?;
        require_non_empty_str(obj, "rejected_analogy_reason")?;
        validate_evidence_array(obj, "evidence")?;
    }
    for role in REQUIRED_ROLES {
        if !roles.contains(role) {
            return Err(format!(
                "current workline attestation guard: missing interdisciplinary role: {role}"
            ));
        }
    }
    Ok(())
}

fn validate_subagent_execution_trace_artifact(
    root: &Map<String, Value>,
    current_workline_id: &str,
    root_items: &[Value],
    status: &str,
) -> Result<(), String> {
    let path = require_non_empty_str(root, "subagent_execution_trace")?;
    if !(path.starts_with("docs/SUBAGENT_EXECUTION_TRACE_")
        || path.starts_with("tests/fixtures/current_workline_attestation_guard/"))
    {
        return Err(
            "current workline attestation guard: subagent_execution_trace must be a trace artifact"
                .to_string(),
        );
    }
    validate_file_sha256(
        path,
        require_non_empty_str(root, "subagent_execution_trace_sha256")?,
    )?;
    let trace = read_resolved_obj(path, "subagent_execution_trace")?;
    require_str(&trace, "kind", "subagent_execution_trace")?;
    require_i64(&trace, "schema_version", 1)?;
    let trace_workline_id = require_non_empty_str(&trace, "current_workline_id")?;
    if trace_workline_id != current_workline_id {
        return Err(
            "current workline attestation guard: subagent trace workline mismatch".to_string(),
        );
    }
    require_bool(&trace, "external_signed_receipt_available", false)?;
    require_bool(&trace, "truth_boundary_recorded", true)?;
    validate_evidence_array(&trace, "evidence")?;
    let trace_items = require_array(&trace, "entries")?;
    if root_items != trace_items {
        return Err(
            "current workline attestation guard: subagent trace entries mismatch".to_string(),
        );
    }
    validate_subagent_execution_log(trace_items, status)
}

fn validate_subagent_execution_log(items: &[Value], status: &str) -> Result<(), String> {
    if status != "pass" {
        return Ok(());
    }
    if items.len() != REQUIRED_ROLES.len() {
        return Err(
            "current workline attestation guard: subagent_execution_log role count mismatch"
                .to_string(),
        );
    }
    let mut roles = BTreeSet::new();
    let mut prompt_hashes = BTreeSet::new();
    let mut output_hashes = BTreeSet::new();
    for item in items {
        let obj = value_obj(item, "subagent execution log item")?;
        let role = require_enum(obj, "role", &REQUIRED_ROLES)?;
        if !roles.insert(role.to_string()) {
            return Err(format!(
                "current workline attestation guard: duplicate subagent role: {role}"
            ));
        }
        validate_agent_id(require_non_empty_str(obj, "agent_id")?)?;
        require_str(obj, "tool", "multi_agent_v1.spawn_agent")?;
        require_str(obj, "status", "completed")?;
        require_enum(obj, "verdict", &["PASS", "PARTIAL", "FAIL"])?;
        let started_at = require_non_empty_str(obj, "started_at_utc")?;
        let completed_at = require_non_empty_str(obj, "completed_at_utc")?;
        validate_timestamp(started_at, "started_at_utc")?;
        validate_timestamp(completed_at, "completed_at_utc")?;
        if completed_at < started_at {
            return Err(
                "current workline attestation guard: completed_at_utc before started_at_utc"
                    .to_string(),
            );
        }
        require_bool(obj, "deepseek_stages_checked", true)?;
        require_bool(obj, "interdisciplinary_research_used", true)?;
        require_bool(obj, "independent_before_cross_review", true)?;
        require_bool(obj, "truth_boundary_recorded", true)?;
        validate_evidence_array(obj, "evidence")?;

        validate_hashed_summary(obj, "prompt_summary", "prompt_sha256", &mut prompt_hashes)?;
        validate_hashed_summary(obj, "output_summary", "output_sha256", &mut output_hashes)?;
    }
    for role in REQUIRED_ROLES {
        if !roles.contains(role) {
            return Err(format!(
                "current workline attestation guard: missing subagent role: {role}"
            ));
        }
    }
    Ok(())
}

fn validate_hashed_summary(
    obj: &Map<String, Value>,
    text_key: &str,
    hash_key: &str,
    seen_hashes: &mut BTreeSet<String>,
) -> Result<(), String> {
    let text = require_non_empty_str(obj, text_key)?;
    if text.len() < 48 {
        return Err(format!(
            "current workline attestation guard: {text_key} is too short"
        ));
    }
    let expected = require_non_empty_str(obj, hash_key)?;
    if !is_sha256_hex(expected) {
        return Err(format!(
            "current workline attestation guard: {hash_key} is not sha256 hex"
        ));
    }
    let actual = sha256_hex(text);
    if actual != expected {
        return Err(format!(
            "current workline attestation guard: {hash_key} does not match {text_key}"
        ));
    }
    if !seen_hashes.insert(expected.to_string()) {
        return Err(format!(
            "current workline attestation guard: duplicate {hash_key}"
        ));
    }
    Ok(())
}

fn validate_review_results(review: &Map<String, Value>, status: &str) -> Result<(), String> {
    validate_named_non_empty_array(review, "risks_found")?;
    validate_named_non_empty_array(review, "accepted_fixes")?;
    validate_named_array(review, "rejected_items")?;
    let blockers = require_array(review, "open_blockers")?;
    if status == "pass" && !blockers.is_empty() {
        return Err("current workline attestation guard: pass with open blockers".to_string());
    }
    Ok(())
}

fn validate_gap_resolution(gap: &Map<String, Value>) -> Result<(), String> {
    require_bool(gap, "old_canonical_pass_bypass_blocked", true)?;
    require_bool(gap, "subagent_trace_required", true)?;
    require_bool(gap, "interdisciplinary_trace_required", true)?;
    require_bool(gap, "stale_handoff_binding_blocked", true)?;
    validate_evidence_array(gap, "evidence")
}

fn reject_sensitive_current_workline_text(root: &Map<String, Value>) -> Result<(), String> {
    let mut scrubbed = Value::Object(root.clone());
    scrub_sha256_values("", &mut scrubbed);
    let obj = scrubbed.as_object().ok_or_else(|| {
        "current workline attestation guard: scrubbed root is not object".to_string()
    })?;
    reject_sensitive_text(obj)
}

fn scrub_sha256_values(key: &str, value: &mut Value) {
    match value {
        Value::Object(obj) => {
            for (child_key, child_value) in obj {
                scrub_sha256_values(child_key, child_value);
            }
        }
        Value::Array(items) => {
            for item in items {
                scrub_sha256_values(key, item);
            }
        }
        Value::String(text) if key.ends_with("_sha256") => {
            *text = "redacted_sha256".to_string();
        }
        _ => {}
    }
}

fn validate_path_field(root: &Map<String, Value>, key: &str, expected: &str) -> Result<(), String> {
    require_str(root, key, expected)?;
    validate_path_exists(expected, key)
}

fn validate_path_exists(path: &str, key: &str) -> Result<(), String> {
    let _ = resolve_project_path(path, key)?;
    Ok(())
}

fn resolve_project_path(path: &str, key: &str) -> Result<PathBuf, String> {
    if path.contains("..") || path.contains("://") || path.contains('\\') {
        return Err(format!(
            "current workline attestation guard: unsafe path in {key}"
        ));
    }
    let root = project_root();
    let path_ref = Path::new(path);
    if path_ref.exists() {
        return Ok(path_ref.to_path_buf());
    }
    let root_path = root.join(path);
    if root_path.exists() {
        return Ok(root_path);
    }
    Err(format!(
        "current workline attestation guard: missing path in {key}: {path}"
    ))
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn validate_file_sha256(path: &str, expected: &str) -> Result<(), String> {
    if !is_sha256_hex(expected) {
        return Err(
            "current workline attestation guard: artifact_sha256 is not sha256 hex".to_string(),
        );
    }
    let resolved = resolve_project_path(path, "workline_attestation")?;
    let bytes = fs::read(&resolved).map_err(|err| {
        format!("current workline attestation guard: cannot read workline_attestation: {err}")
    })?;
    let actual = sha256_bytes(&bytes);
    if actual != expected {
        return Err("current workline attestation guard: artifact_sha256 mismatch".to_string());
    }
    Ok(())
}

fn read_resolved_obj(path: &str, key: &str) -> Result<Map<String, Value>, String> {
    let resolved = resolve_project_path(path, key)?;
    let raw = fs::read_to_string(&resolved)
        .map_err(|err| format!("current workline attestation guard: cannot read {key}: {err}"))?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|err| format!("current workline attestation guard: invalid {key} json: {err}"))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("current workline attestation guard: {key} root is not object"))
}

fn validate_slug(value: &str, key: &str) -> Result<(), String> {
    if value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'.'
    }) {
        Ok(())
    } else {
        Err(format!(
            "current workline attestation guard: invalid slug field: {key}"
        ))
    }
}

fn validate_timestamp(value: &str, key: &str) -> Result<(), String> {
    if value.len() == 20
        && value.ends_with('Z')
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value.as_bytes()[10] == b'T'
        && value.as_bytes()[13] == b':'
        && value.as_bytes()[16] == b':'
    {
        Ok(())
    } else {
        Err(format!(
            "current workline attestation guard: {key} must use YYYY-MM-DDTHH:MM:SSZ"
        ))
    }
}

fn validate_agent_id(value: &str) -> Result<(), String> {
    let bytes = value.as_bytes();
    let dash_positions = [8, 13, 18, 23];
    if bytes.len() != 36 {
        return Err("current workline attestation guard: invalid agent_id format".to_string());
    }
    for (index, byte) in bytes.iter().enumerate() {
        if dash_positions.contains(&index) {
            if *byte != b'-' {
                return Err(
                    "current workline attestation guard: invalid agent_id format".to_string(),
                );
            }
        } else if !byte.is_ascii_hexdigit() {
            return Err("current workline attestation guard: invalid agent_id format".to_string());
        }
    }
    Ok(())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_hex(value: &str) -> String {
    sha256_bytes(value.as_bytes())
}

fn sha256_bytes(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut out = String::with_capacity(64);
    for byte in digest {
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture(path: &str) -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("tests/fixtures/current_workline_attestation_guard")
            .join(path)
            .to_string_lossy()
            .into_owned()
    }

    fn current_artifact() -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("docs/CURRENT_WORKLINE_ATTESTATION.json")
            .to_string_lossy()
            .into_owned()
    }

    fn valid_root() -> Result<Map<String, Value>, String> {
        read_obj(&current_artifact())
    }

    #[test]
    fn accepts_current_workline_artifact() -> Result<(), String> {
        validate_file(&current_artifact())
    }

    #[test]
    fn rejects_empty_subagent_trace() -> Result<(), String> {
        let mut root = valid_root()?;
        root.insert(
            "subagent_execution_log".to_string(),
            Value::Array(Vec::new()),
        );
        let result = validate_root(&root);
        assert!(result.is_err(), "expected missing trace failure");
        let err = result.err().unwrap_or_default();
        assert!(
            err.contains("subagent_execution_log role count mismatch"),
            "unexpected error: {err}"
        );
        Ok(())
    }

    #[test]
    fn rejects_old_canonical_workflow_attestation() {
        let canonical = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("docs/WORKFLOW_ATTESTATION.json")
            .to_string_lossy()
            .into_owned();
        let result = validate_file(&canonical);
        assert!(result.is_err(), "expected canonical attestation rejection");
        let err = result.err().unwrap_or_default();
        assert!(err.contains("kind mismatch"), "unexpected error: {err}");
    }

    #[test]
    fn rejects_missing_subagent_trace_field() -> Result<(), String> {
        let mut root = valid_root()?;
        root.remove("subagent_execution_log");
        let result = validate_root(&root);
        assert!(result.is_err(), "expected missing trace field failure");
        let err = result.err().unwrap_or_default();
        assert!(
            err.contains("missing array: subagent_execution_log"),
            "unexpected error: {err}"
        );
        Ok(())
    }

    #[test]
    fn rejects_kind_mismatch_fixture() {
        let result = validate_file(&fixture("fail/canonical_kind_mismatch.json"));
        assert!(result.is_err(), "expected kind mismatch failure");
        let err = result.err().unwrap_or_default();
        assert!(err.contains("kind mismatch"), "unexpected error: {err}");
    }

    #[test]
    fn rejects_tampered_workline_attestation_hash() -> Result<(), String> {
        let mut root = valid_root()?;
        root.insert(
            "workline_attestation_sha256".to_string(),
            Value::String("0".repeat(64)),
        );
        let result = validate_root(&root);
        assert!(result.is_err(), "expected hash mismatch failure");
        let err = result.err().unwrap_or_default();
        assert!(
            err.contains("artifact_sha256 mismatch"),
            "unexpected error: {err}"
        );
        Ok(())
    }

    #[test]
    fn rejects_tampered_handoff_hash() -> Result<(), String> {
        let mut root = valid_root()?;
        root.insert(
            "latest_handoff_sha256".to_string(),
            Value::String("0".repeat(64)),
        );
        let result = validate_root(&root);
        assert!(result.is_err(), "expected handoff hash mismatch failure");
        let err = result.err().unwrap_or_default();
        assert!(
            err.contains("artifact_sha256 mismatch"),
            "unexpected error: {err}"
        );
        Ok(())
    }

    #[test]
    fn rejects_missing_stage_reports() -> Result<(), String> {
        let mut root = valid_root()?;
        let stage_trace = root
            .get_mut("stage_trace")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "missing stage_trace".to_string())?;
        stage_trace.remove("stage_reports");
        let result = validate_root(&root);
        assert!(result.is_err(), "expected missing stage_reports failure");
        let err = result.err().unwrap_or_default();
        assert!(
            err.contains("missing array: stage_reports"),
            "unexpected error: {err}"
        );
        Ok(())
    }

    #[test]
    fn rejects_weak_interdisciplinary_trace() -> Result<(), String> {
        let mut root = valid_root()?;
        let trace = root
            .get_mut("interdisciplinary_trace")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "missing interdisciplinary_trace".to_string())?;
        trace.remove("per_role_checks");
        let result = validate_root(&root);
        assert!(result.is_err(), "expected weak interdisciplinary failure");
        let err = result.err().unwrap_or_default();
        assert!(
            err.contains("missing array: per_role_checks"),
            "unexpected error: {err}"
        );
        Ok(())
    }

    #[test]
    fn rejects_fake_agent_id_format() -> Result<(), String> {
        let mut root = valid_root()?;
        let first_agent = root
            .get_mut("subagent_execution_log")
            .and_then(Value::as_array_mut)
            .and_then(|items| items.first_mut())
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "missing first subagent".to_string())?;
        first_agent.insert(
            "agent_id".to_string(),
            Value::String("fixture-architect".to_string()),
        );
        let result = validate_root(&root);
        assert!(result.is_err(), "expected fake agent id failure");
        let err = result.err().unwrap_or_default();
        assert!(
            err.contains("invalid agent_id format"),
            "unexpected error: {err}"
        );
        Ok(())
    }
}
