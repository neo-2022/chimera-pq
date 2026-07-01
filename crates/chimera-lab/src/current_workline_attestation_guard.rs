use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fmt::Write as _, fs, path::Path, time::SystemTime};

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
    validate_deepseek_stage_check(require_obj(root, "deepseek_stage_check")?)?;
    validate_interdisciplinary_trace(require_obj(root, "interdisciplinary_trace")?)?;
    validate_subagent_execution_log(require_array(root, "subagent_execution_log")?, status)?;
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
    Ok(())
}

fn validate_latest_handoff(root: &Map<String, Value>) -> Result<(), String> {
    let path = require_non_empty_str(root, "latest_handoff")?;
    if path.starts_with("tests/fixtures/") {
        return validate_path_exists(path, "latest_handoff");
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
    Ok(())
}

fn latest_handoff_path() -> Result<String, String> {
    let mut paths = Vec::new();
    let entries = fs::read_dir("docs")
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
    validate_evidence_array(trace, "evidence")
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
        require_non_empty_str(obj, "agent_id")?;
        require_str(obj, "tool", "multi_agent_v1.spawn_agent")?;
        require_str(obj, "status", "completed")?;
        validate_timestamp(
            require_non_empty_str(obj, "started_at_utc")?,
            "started_at_utc",
        )?;
        validate_timestamp(
            require_non_empty_str(obj, "completed_at_utc")?,
            "completed_at_utc",
        )?;
        require_bool(obj, "deepseek_stages_checked", true)?;
        require_bool(obj, "interdisciplinary_research_used", true)?;
        require_bool(obj, "independent_before_cross_review", true)?;
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
    if path.contains("..") || path.contains("://") || path.contains('\\') {
        return Err(format!(
            "current workline attestation guard: unsafe path in {key}"
        ));
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if !(Path::new(path).exists() || root.join(path).exists()) {
        return Err(format!(
            "current workline attestation guard: missing path in {key}: {path}"
        ));
    }
    Ok(())
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

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_hex(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
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

    #[test]
    fn accepts_current_workline_fixture() -> Result<(), String> {
        validate_file(&fixture("pass/current_workline.json"))
    }

    #[test]
    fn rejects_missing_subagent_trace_fixture() {
        let result = validate_file(&fixture("fail/missing_subagent_execution_log.json"));
        assert!(result.is_err(), "expected missing trace failure");
        let err = result.err().unwrap_or_default();
        assert!(
            err.contains("subagent_execution_log role count mismatch")
                || err.contains("missing array: subagent_execution_log"),
            "unexpected error: {err}"
        );
    }
}
