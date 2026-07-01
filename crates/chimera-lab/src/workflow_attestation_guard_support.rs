use serde_json::{Map, Value};
use std::fs;

pub(crate) fn validate_evidence_array(obj: &Map<String, Value>, key: &str) -> Result<(), String> {
    for value in require_non_empty_array(obj, key)? {
        let evidence = value
            .as_str()
            .ok_or_else(|| format!("workflow attestation guard: evidence is not string: {key}"))?;
        validate_evidence_ref(evidence)?;
    }
    Ok(())
}

pub(crate) fn validate_named_array(obj: &Map<String, Value>, key: &str) -> Result<(), String> {
    for value in require_array(obj, key)? {
        validate_named_item(value, key)?;
    }
    Ok(())
}

pub(crate) fn validate_named_non_empty_array(
    obj: &Map<String, Value>,
    key: &str,
) -> Result<(), String> {
    for value in require_non_empty_array(obj, key)? {
        validate_named_item(value, key)?;
    }
    Ok(())
}

pub(crate) fn read_obj(path: &str) -> Result<Map<String, Value>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|_| format!("workflow attestation guard: missing file: {path}"))?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|_| format!("workflow attestation guard: invalid json: {path}"))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| "workflow attestation guard: root is not object".to_string())
}

pub(crate) fn read_project_obj(path: &str) -> Result<Map<String, Value>, String> {
    match read_obj(path) {
        Ok(value) => Ok(value),
        Err(_) => {
            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            let fallback = format!("{manifest_dir}/../../{path}");
            read_obj(&fallback)
        }
    }
}

pub(crate) fn require_obj<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a Map<String, Value>, String> {
    obj.get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("workflow attestation guard: missing object: {key}"))
}

pub(crate) fn require_array<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a Vec<Value>, String> {
    obj.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("workflow attestation guard: missing array: {key}"))
}

pub(crate) fn require_non_empty_array<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a Vec<Value>, String> {
    let values = require_array(obj, key)?;
    if values.is_empty() {
        return Err(format!("workflow attestation guard: empty array: {key}"));
    }
    Ok(values)
}

pub(crate) fn require_empty_array(obj: &Map<String, Value>, key: &str) -> Result<(), String> {
    if !require_array(obj, key)?.is_empty() {
        return Err(format!(
            "workflow attestation guard: expected empty array: {key}"
        ));
    }
    Ok(())
}

pub(crate) fn require_str(
    obj: &Map<String, Value>,
    key: &str,
    expected: &str,
) -> Result<(), String> {
    if obj.get(key).and_then(Value::as_str) != Some(expected) {
        return Err(format!("workflow attestation guard: {key} mismatch"));
    }
    Ok(())
}

pub(crate) fn require_non_empty_str<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, String> {
    let value = obj
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("workflow attestation guard: missing string: {key}"))?;
    if value.trim().is_empty() {
        return Err(format!("workflow attestation guard: empty string: {key}"));
    }
    Ok(value)
}

pub(crate) fn require_i64(
    obj: &Map<String, Value>,
    key: &str,
    expected: i64,
) -> Result<(), String> {
    if obj.get(key).and_then(Value::as_i64) != Some(expected) {
        return Err(format!("workflow attestation guard: {key} mismatch"));
    }
    Ok(())
}

pub(crate) fn require_i64_value(obj: &Map<String, Value>, key: &str) -> Result<i64, String> {
    obj.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("workflow attestation guard: missing integer: {key}"))
}

pub(crate) fn require_min_i64(obj: &Map<String, Value>, key: &str, min: i64) -> Result<(), String> {
    let value = require_i64_value(obj, key)?;
    if value < min {
        return Err(format!(
            "workflow attestation guard: integer below minimum: {key}"
        ));
    }
    Ok(())
}

pub(crate) fn require_min_string_array(
    obj: &Map<String, Value>,
    key: &str,
    min_len: usize,
) -> Result<(), String> {
    let values = require_non_empty_array(obj, key)?;
    if values.len() < min_len {
        return Err(format!(
            "workflow attestation guard: array below minimum length: {key}"
        ));
    }
    for value in values {
        let Some(text) = value.as_str() else {
            return Err(format!(
                "workflow attestation guard: non-string array item: {key}"
            ));
        };
        if text.trim().is_empty() {
            return Err(format!(
                "workflow attestation guard: empty string array item: {key}"
            ));
        }
    }
    Ok(())
}

pub(crate) fn require_string_array_contains_all(
    obj: &Map<String, Value>,
    key: &str,
    required: &[&str],
) -> Result<(), String> {
    let values = require_non_empty_array(obj, key)?;
    let mut found = std::collections::BTreeSet::new();
    for value in values {
        let Some(text) = value.as_str() else {
            return Err(format!(
                "workflow attestation guard: non-string array item: {key}"
            ));
        };
        if text.trim().is_empty() {
            return Err(format!(
                "workflow attestation guard: empty string array item: {key}"
            ));
        }
        found.insert(text);
    }
    for item in required {
        if !found.contains(item) {
            return Err(format!(
                "workflow attestation guard: missing required field name in {key}: {item}"
            ));
        }
    }
    Ok(())
}

pub(crate) fn require_min_array_len(
    obj: &Map<String, Value>,
    key: &str,
    min_len: usize,
) -> Result<(), String> {
    let values = require_non_empty_array(obj, key)?;
    if values.len() < min_len {
        return Err(format!(
            "workflow attestation guard: array below minimum length: {key}"
        ));
    }
    Ok(())
}

pub(crate) fn require_bool(
    obj: &Map<String, Value>,
    key: &str,
    expected: bool,
) -> Result<(), String> {
    if obj.get(key).and_then(Value::as_bool) != Some(expected) {
        return Err(format!("workflow attestation guard: {key} mismatch"));
    }
    Ok(())
}

pub(crate) fn require_bool_value(obj: &Map<String, Value>, key: &str) -> Result<bool, String> {
    obj.get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("workflow attestation guard: missing bool: {key}"))
}

pub(crate) fn require_enum<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
    allowed: &[&str],
) -> Result<&'a str, String> {
    let value = require_non_empty_str(obj, key)?;
    require_enum_value(value, allowed, key)?;
    Ok(value)
}

pub(crate) fn require_enum_value(value: &str, allowed: &[&str], label: &str) -> Result<(), String> {
    if !allowed.contains(&value) {
        return Err(format!(
            "workflow attestation guard: invalid {label}: {value}"
        ));
    }
    Ok(())
}

pub(crate) fn value_obj<'a>(
    value: &'a Value,
    label: &str,
) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("workflow attestation guard: {label} is not object"))
}

fn validate_evidence_ref(evidence: &str) -> Result<(), String> {
    let trimmed = evidence.trim();
    if trimmed.is_empty() {
        return Err("workflow attestation guard: empty evidence ref".to_string());
    }
    if trimmed.contains("..") || trimmed.contains("://") || trimmed.contains('\\') {
        return Err(format!(
            "workflow attestation guard: unsafe evidence ref: {trimmed}"
        ));
    }
    if is_allowed_command_evidence(trimmed) {
        return Ok(());
    }
    if is_allowed_file_evidence(trimmed) {
        return Ok(());
    }
    Err(format!(
        "workflow attestation guard: unbounded evidence ref: {trimmed}"
    ))
}

fn is_allowed_file_evidence(trimmed: &str) -> bool {
    let file_part = trimmed.split_once(':').map_or(trimmed, |(path, _)| path);
    if file_part.is_empty() || file_part.contains(' ') {
        return false;
    }
    let allowed_exact = [
        "AGENTS.md",
        "Agent.md",
        "CHIMERA-PQ_MVP_SPEC.md",
        "README.md",
        "justfile",
    ];
    let allowed_prefixes = ["docs/", "crates/", "scripts/", "tests/"];
    if !(allowed_exact.contains(&file_part)
        || allowed_prefixes
            .iter()
            .any(|prefix| file_part.starts_with(prefix)))
    {
        return false;
    }
    std::path::Path::new(file_part).exists()
        || std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(file_part)
            .exists()
        || std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .join(file_part)
            .exists()
}

fn is_allowed_command_evidence(trimmed: &str) -> bool {
    const ALLOWED_COMMANDS: [&str; 25] = [
        "cargo fmt --all -- --check",
        "cargo check -q --workspace --all-targets",
        "cargo clippy -q --workspace --all-targets -- -D warnings",
        "cargo clippy -q -p chimera-lab --bin current_workline_attestation_guard -- -D warnings",
        "cargo test -q -p chimera-lab --bin workflow_attestation_guard",
        "cargo test -q -p chimera-lab --bin current_workline_attestation_guard",
        "cargo run -q -p chimera-lab --bin workflow_attestation_guard -- docs/WORKFLOW_ATTESTATION.json",
        "cargo run -q -p chimera-lab --bin current_workline_attestation_guard -- docs/CURRENT_WORKLINE_ATTESTATION.json",
        "cargo run -q -p chimera-lab --bin ai_architect_artifact_guard -- docs/AI_ARCHITECT_LIFECYCLE_GUARD.md docs/AI_ARCHITECT_ALGORITHM_COVERAGE.json docs/WORKFLOW_ATTESTATION.json docs/RESEARCH_DEBT.md",
        "just workflow-attestation-guard-selfcheck",
        "just workflow-attestation-guard",
        "just current-workline-attestation-guard-selfcheck",
        "just current-workline-attestation-guard",
        "just ai-architect-artifact-guard-selfcheck",
        "just ai-architect-artifact-guard",
        "just session-process-guard",
        "just handoff-process-check",
        "just rust-no-hardcode-guard-selfcheck",
        "just rust-no-hardcode-guard",
        "just json-no-dupe-guard-selfcheck",
        "just metadata-perf-smoke-selfcheck",
        "bash scripts/workflow_attestation_guard.sh docs/WORKFLOW_ATTESTATION.json",
        "bash scripts/ai_architect_artifact_guard.sh docs/AI_ARCHITECT_LIFECYCLE_GUARD.md docs/AI_ARCHITECT_ALGORITHM_COVERAGE.json docs/WORKFLOW_ATTESTATION.json docs/RESEARCH_DEBT.md",
        "rg source_text_coverage docs/WORKFLOW_ATTESTATION.json",
        "git diff --check",
    ];
    ALLOWED_COMMANDS.contains(&trimmed)
}

fn validate_named_item(value: &Value, key: &str) -> Result<(), String> {
    if let Some(text) = value.as_str() {
        if text.trim().is_empty() {
            return Err(format!(
                "workflow attestation guard: empty array item: {key}"
            ));
        }
        return Ok(());
    }
    let obj = value.as_object().ok_or_else(|| {
        format!("workflow attestation guard: array item must be string or object: {key}")
    })?;
    if let Some(text) = obj.get("text").and_then(Value::as_str)
        && !text.trim().is_empty()
    {
        return Ok(());
    }
    if let Some(name) = obj.get("name").and_then(Value::as_str)
        && !name.trim().is_empty()
    {
        return Ok(());
    }
    if let Some(id) = obj.get("id").and_then(Value::as_str)
        && !id.trim().is_empty()
    {
        return Ok(());
    }
    Err(format!(
        "workflow attestation guard: object array item needs text/name/id: {key}"
    ))
}
