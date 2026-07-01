use super::*;
use std::path::Path;

fn fixture(path: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/fixtures/workflow_attestation_guard")
        .join(path)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn accepts_full_lifecycle_fixture() -> Result<(), String> {
    validate_file(&fixture("pass/full_lifecycle.json"))
}

#[test]
fn accepts_stop_without_done_fixture() -> Result<(), String> {
    validate_file(&fixture("pass/stop_no_done.json"))
}

#[test]
fn rejects_missing_stage_fixture() {
    assert_fixture_error(
        "fail/missing_stage.json",
        "full lifecycle must contain exactly 8 stages",
    );
}

#[test]
fn rejects_pass_without_council_fixture() {
    assert_fixture_error(
        "fail/pass_without_council.json",
        "pass/done requires real council and red-team review",
    );
}

#[test]
fn rejects_lab_promoted_to_real_world_fixture() {
    assert_fixture_error(
        "fail/lab_promoted_to_real_world.json",
        "lab scope promoted to real-world/prod-ready",
    );
}

#[test]
fn rejects_detailed_lifecycle_gap_fixtures() {
    for (path, expected) in [
        (
            "fail/missing_stage_reports.json",
            "missing object: stage_reports",
        ),
        ("fail/missing_council_detail.json", "missing object: detail"),
        (
            "fail/missing_interdisciplinary_research.json",
            "missing object: interdisciplinary_research",
        ),
        (
            "fail/interdisciplinary_software_only.json",
            "software_only mismatch",
        ),
        (
            "fail/interdisciplinary_too_few_disciplines.json",
            "at least three non-software disciplines",
        ),
        (
            "fail/validation_missing_stress_testing.json",
            "missing array: stress_testing",
        ),
        (
            "fail/postmortem_missing_research_debt_update.json",
            "missing array: research_debt_update",
        ),
        (
            "fail/done_without_final_done_conditions.json",
            "done/pass requires final done conditions",
        ),
        (
            "fail/missing_coverage_link.json",
            "stage report coverage mismatch",
        ),
        ("fail/public_ip_literal.json", "IP address literal found"),
        ("fail/secret_marker.json", "client_secret"),
        (
            "fail/missing_workline_id.json",
            "missing string: workline_id",
        ),
        (
            "fail/missing_source_coverage_ref.json",
            "source_coverage_ref mismatch",
        ),
        (
            "fail/missing_normative_requirement_coverage.json",
            "missing object: normative_requirement_coverage",
        ),
        (
            "fail/missing_per_role_interdisciplinary_fields.json",
            "missing array: interdisciplinary_findings",
        ),
        (
            "fail/missing_rejected_irrelevant_analogies.json",
            "missing array: rejected_irrelevant_analogies",
        ),
        (
            "fail/knowledge_transfer_without_risks_or_non_transferable_parts.json",
            "missing array: non_transferable_parts",
        ),
        (
            "fail/final_decision_research_debt_updated_false.json",
            "done/pass requires final done conditions",
        ),
        ("fail/wrong_coverage_file.json", "coverage_file mismatch"),
        ("fail/wrong_source_sha256.json", "source_sha256 mismatch"),
        (
            "fail/wrong_coverage_digest.json",
            "coverage_digest_fnv1a mismatch",
        ),
        (
            "fail/broken_stage_report_ref.json",
            "stage_report_ref mismatch",
        ),
        (
            "fail/public_ipv6_literal.json",
            "public IPv6 address literal found",
        ),
        ("fail/hostname_endpoint.json", "hostname/FQDN marker found"),
        ("fail/root_at_host.json", "principal@host marker found"),
        ("fail/url_endpoint.json", "://"),
        (
            "fail/unlabeled_token.json",
            "unlabeled high-entropy token found",
        ),
        (
            "fail/done_state_without_pass_skips_stage_reports.json",
            "final done requires status=pass and task_done=true",
        ),
        (
            "fail/pass_with_return_gate.json",
            "done/pass requires pass gate",
        ),
        (
            "fail/pass_with_blocked_consensus.json",
            "pass/done cannot use blocked or not_agreed consensus",
        ),
        (
            "fail/final_done_bool_only.json",
            "missing array: final_done_conditions",
        ),
        (
            "fail/forbidden_prod_ready_narrative.json",
            "forbidden narrative claim",
        ),
        (
            "fail/hostname_endpoint_key_value.json",
            "hostname/FQDN marker found",
        ),
        ("fail/credential_key_auth_value.json", "auth="),
        (
            "fail/final_decision_log_not_updated.json",
            "done/pass requires final done conditions",
        ),
        (
            "fail/coverage_id_wrong_stage.json",
            "coverage id assigned to wrong stage",
        ),
        (
            "fail/duplicate_stage_coverage_id.json",
            "duplicate covered_required_item_id",
        ),
        (
            "fail/nonexistent_evidence_ref.json",
            "unbounded evidence ref",
        ),
        ("fail/weak_transfer_principle.json", "is not object"),
        (
            "fail/pass_gate_with_found_problem.json",
            "expected empty array: found_problems",
        ),
        (
            "fail/single_label_user_at_host.json",
            "principal@host marker found",
        ),
        (
            "fail/single_label_host_port.json",
            "host:port endpoint marker found",
        ),
        (
            "fail/missing_source_text_coverage.json",
            "missing object: source_text_coverage",
        ),
        (
            "fail/missing_final_done_checklist.json",
            "missing array: final_done_checklist",
        ),
        (
            "fail/missing_interdisciplinary_source_lists.json",
            "missing object: interdisciplinary_source_lists",
        ),
    ] {
        assert_fixture_error(path, expected);
    }
}

#[test]
fn detects_any_ipv4_literals_without_stand_specific_hardcode() {
    let private_lan = format!("endpoint={}.{}.{}.{}", 192, 168, 10, 20);
    assert!(redaction::contains_ipv4_literal("endpoint=127.0.0.1"));
    assert!(redaction::contains_ipv4_literal(&private_lan));
    assert!(redaction::contains_ipv4_literal("endpoint=10.1.2.3"));
    assert!(redaction::contains_ipv4_literal("endpoint=172.16.0.1"));
    assert!(redaction::contains_ipv4_literal("endpoint=203.0.113.10"));
    assert!(!redaction::contains_ipv4_literal("not an ip 999.1.2.3"));
}

#[test]
fn rejects_claim_only_evidence_refs() -> Result<(), String> {
    let mut root = read_fixture_obj("pass/full_lifecycle.json")?;
    let stages = root
        .get_mut("stages")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "missing stages".to_string())?;
    let first_stage = stages[0]
        .as_object_mut()
        .ok_or_else(|| "missing first stage object".to_string())?;
    first_stage.insert(
        "evidence".to_string(),
        Value::Array(vec![Value::String("verified".to_string())]),
    );

    assert!(validate_root(&root).is_err());
    Ok(())
}

#[test]
fn rejects_case_insensitive_secret_markers() -> Result<(), String> {
    let mut root = read_fixture_obj("pass/full_lifecycle.json")?;
    root.insert(
        "leaky_note".to_string(),
        Value::String("Authorization: Bearer redacted".to_string()),
    );

    assert!(validate_root(&root).is_err());
    Ok(())
}

#[test]
fn rejects_local_paths_and_local_address_markers() -> Result<(), String> {
    let mut root = read_fixture_obj("pass/full_lifecycle.json")?;
    root.insert(
        "leaky_path".to_string(),
        Value::String("/home/example/chimera".to_string()),
    );
    assert!(validate_root(&root).is_err());

    let mut root = read_fixture_obj("pass/full_lifecycle.json")?;
    root.insert(
        "leaky_host".to_string(),
        Value::String("localhost".to_string()),
    );
    assert!(validate_root(&root).is_err());
    Ok(())
}

#[test]
fn rejects_missing_interdisciplinary_research_for_done() -> Result<(), String> {
    let mut root = read_fixture_obj("pass/full_lifecycle.json")?;
    root.remove("interdisciplinary_research");

    assert!(validate_root(&root).is_err());
    Ok(())
}

#[test]
fn rejects_software_only_interdisciplinary_research() -> Result<(), String> {
    let mut root = read_fixture_obj("pass/full_lifecycle.json")?;
    let research = root
        .get_mut("interdisciplinary_research")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "missing interdisciplinary_research".to_string())?;
    research.insert("software_only".to_string(), Value::Bool(true));

    assert!(validate_root(&root).is_err());
    Ok(())
}

fn read_fixture_obj(path: &str) -> Result<Map<String, Value>, String> {
    support::read_obj(&fixture(path))
}

fn assert_fixture_error(path: &str, expected: &str) {
    let result = validate_file(&fixture(path));
    assert!(result.is_err(), "{path}: expected validation failure");
    let err = result.err().unwrap_or_default();
    assert!(
        err.contains(expected),
        "{path}: expected error containing {expected:?}, got {err:?}"
    );
}
