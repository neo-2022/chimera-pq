use super::validate_probe_contract;
use serde_json::{Map, Value, json};

const CI_SNAPSHOT_HOST: &str = "chimera-ci-snapshot.local";

fn row(index: usize, direct_ok: bool) -> Value {
    let route = if direct_ok { "direct" } else { "transit" };
    json!({
        "url": format!("target#{index}"),
        "target_ref": format!("target#{index}"),
        "direct_ok": direct_ok,
        "recommended_route": route,
        "policy_hint": format!("target_kind=domain_exact_present outbound={route}"),
        "policy_apply_result": "not_requested",
        "policy_rule_ref": "",
        "policy_verify_ok": false,
        "policy_verify_outbound": "",
        "target_error": ""
    })
}

fn probe_with_targets(targets: Vec<Value>) -> Map<String, Value> {
    let all = targets.len() as i64;
    let direct_ok = targets
        .iter()
        .filter(|target| {
            target
                .get("direct_ok")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count() as i64;
    let unreachable = all - direct_ok;
    let mut probe = Map::new();
    probe.insert("status".to_string(), json!("ok"));
    probe.insert("kind".to_string(), json!("probe_access"));
    probe.insert("redaction".to_string(), json!("raw_targets_redacted"));
    probe.insert("target_profile".to_string(), json!("live"));
    probe.insert("network_state".to_string(), json!("not_modified"));
    probe.insert(
        "totals".to_string(),
        json!({
            "all": all,
            "direct_ok": direct_ok,
            "unreachable": unreachable,
            "policy_apply_failed": 0,
            "failed_total": unreachable,
            "fail_threshold": unreachable,
            "threshold_exceeded": false
        }),
    );
    probe.insert("targets".to_string(), Value::Array(targets));
    probe
}

fn probe_one() -> Map<String, Value> {
    probe_with_targets(vec![row(1, true)])
}

#[test]
fn accepts_redacted_live_contract() {
    let probe = probe_one();
    assert!(validate_probe_contract(&probe, CI_SNAPSHOT_HOST).is_ok());
}

#[test]
fn accepts_applied_policy_with_redacted_rule_ref() {
    let mut probe = probe_one();
    probe.insert(
        "targets".to_string(),
        json!([{
            "url": "target#1",
            "target_ref": "target#1",
            "direct_ok": true,
            "recommended_route": "direct",
            "policy_hint": "target_kind=domain_exact_present outbound=direct",
            "policy_apply_result": "applied",
            "policy_rule_ref": "rule#1",
            "policy_verify_ok": true,
            "policy_verify_outbound": "direct",
            "target_error": ""
        }]),
    );
    assert!(validate_probe_contract(&probe, CI_SNAPSHOT_HOST).is_ok());
}

#[test]
fn rejects_missing_redaction_marker() {
    let mut probe = probe_one();
    probe.remove("redaction");
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(
        res.err()
            .is_some_and(|e| e.contains("redaction marker missing"))
    );
}

#[test]
fn rejects_invalid_target_profile() {
    let mut probe = probe_one();
    probe.insert("target_profile".to_string(), json!("raw"));
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(res.err().is_some_and(|e| e.contains("target_profile")));
}

#[test]
fn rejects_raw_target_url() {
    let mut probe = probe_one();
    let mut targets = vec![row(1, true)];
    targets[0]["url"] = json!("https://example.org/path?token=leak");
    probe.insert("targets".to_string(), Value::Array(targets));
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(
        res.err()
            .is_some_and(|e| e.contains("url must be redacted ref"))
    );
}

#[test]
fn rejects_target_ref_mismatch() {
    let mut probe = probe_one();
    let mut targets = vec![row(1, true)];
    targets[0]["target_ref"] = json!("target#2");
    probe.insert("targets".to_string(), Value::Array(targets));
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(res.err().is_some_and(|e| e.contains("target_ref")));
}

#[test]
fn rejects_duplicate_or_skipped_target_refs() {
    let probe = probe_with_targets(vec![row(1, true), row(1, true)]);
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(res.err().is_some_and(|e| e.contains("redacted ref")));
}

#[test]
fn rejects_raw_domain_policy_hint() {
    let mut probe = probe_one();
    let mut targets = vec![row(1, true)];
    targets[0]["policy_hint"] = json!("domain_exact=example.org outbound=direct");
    probe.insert("targets".to_string(), Value::Array(targets));
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(res.err().is_some_and(|e| e.contains("leaks raw data")));
}

#[test]
fn rejects_legacy_policy_rule_id() {
    let mut probe = probe_one();
    let mut targets = vec![row(1, true)];
    targets[0]["policy_rule_id"] = json!("probe-example-org");
    probe.insert("targets".to_string(), Value::Array(targets));
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(res.err().is_some_and(|e| e.contains("policy_rule_id")));
}

#[test]
fn rejects_sensitive_target_error_text() {
    for leak in [
        "host=example.org",
        "remote=203.0.113.10",
        "remote=2001:db8::1",
        "user@example.org",
        "/home/operator/chimera",
        "/tmp/chimera/raw",
        "token=secret",
        "payload=48656c6c6f",
        "hexdump deadbeef",
    ] {
        let mut probe = probe_one();
        let mut target = row(1, true);
        target["policy_apply_result"] = json!("failed");
        target["policy_verify_ok"] = json!(false);
        target["target_error"] = json!(leak);
        probe.insert("targets".to_string(), json!([target]));
        probe.insert(
            "totals".to_string(),
            json!({
                "all": 1,
                "direct_ok": 1,
                "unreachable": 0,
                "policy_apply_failed": 1,
                "failed_total": 1,
                "fail_threshold": 1,
                "threshold_exceeded": false
            }),
        );
        let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
        assert!(res.is_err(), "leak unexpectedly accepted: {leak}");
        assert!(res.err().is_some_and(|e| e.contains("leaks raw data")));
    }
}

#[test]
fn rejects_applied_policy_without_redacted_rule_ref() {
    let mut probe = probe_one();
    let mut target = row(1, true);
    target["policy_apply_result"] = json!("applied");
    target["policy_verify_ok"] = json!(true);
    target["policy_verify_outbound"] = json!("direct");
    probe.insert("targets".to_string(), json!([target]));
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(
        res.err()
            .is_some_and(|e| e.contains("policy verification mismatch"))
    );
}
