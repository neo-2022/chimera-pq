use super::validate_probe_contract;
use serde_json::{Map, Value, json};

const CI_SNAPSHOT_HOST: &str = "chimera-ci-snapshot.local";

fn probe_with(url: &str) -> Map<String, Value> {
    let mut probe = Map::new();
    probe.insert("status".to_string(), json!("ok"));
    probe.insert("kind".to_string(), json!("probe_access"));
    probe.insert("network_state".to_string(), json!("not_modified"));
    probe.insert(
        "totals".to_string(),
        json!({
            "all": 1,
            "direct_ok": 1,
            "unreachable": 0,
            "policy_apply_failed": 0,
            "failed_total": 0,
            "fail_threshold": 0,
            "threshold_exceeded": false
        }),
    );
    probe.insert(
        "targets".to_string(),
        json!([{
            "url": url,
            "direct_ok": true,
            "recommended_route": "direct",
            "policy_hint": "domain_exact=example.org outbound=direct",
            "policy_apply_result": "not_requested",
            "policy_rule_id": "",
            "policy_verify_ok": false,
            "policy_verify_outbound": "",
            "target_error": ""
        }]),
    );
    probe
}

#[test]
fn accepts_live_contract() {
    let probe = probe_with("https://example.org");
    assert!(validate_probe_contract(&probe, CI_SNAPSHOT_HOST).is_ok());
}

#[test]
fn rejects_probe_contract_total_mismatch() {
    let mut probe = probe_with("https://example.org");
    probe.insert(
        "totals".to_string(),
        json!({
            "all": 1,
            "direct_ok": 1,
            "unreachable": 1,
            "policy_apply_failed": 0,
            "failed_total": 0,
            "fail_threshold": 0,
            "threshold_exceeded": false
        }),
    );
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(res.err().is_some_and(|e| e.contains("totals mismatch")));
}

#[test]
fn rejects_probe_contract_target_row_mismatch() {
    let mut probe = probe_with("https://example.org");
    probe.insert(
        "targets".to_string(),
        json!([{
            "url": "https://example.org",
            "direct_ok": false,
            "recommended_route": "transit",
            "policy_hint": "domain_exact=example.org outbound=transit",
            "policy_apply_result": "not_requested",
            "policy_rule_id": "",
            "policy_verify_ok": false,
            "policy_verify_outbound": "",
            "target_error": ""
        }]),
    );
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(
        res.err()
            .is_some_and(|e| e.contains("target direct_ok total mismatch"))
    );
}

#[test]
fn rejects_probe_contract_missing_target_schema() {
    let mut probe = probe_with("https://example.org");
    probe.insert(
        "targets".to_string(),
        json!([{ "url": "https://example.org" }]),
    );
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(
        res.err()
            .is_some_and(|e| e.contains("target direct_ok missing"))
    );
}

#[test]
fn rejects_probe_contract_non_object_target() {
    let mut probe = probe_with("https://example.org");
    probe.insert("targets".to_string(), json!(["https://example.org"]));
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(
        res.err()
            .is_some_and(|e| e.contains("target row is not object"))
    );
}

#[test]
fn rejects_probe_contract_non_http_target_url() {
    let mut probe = probe_with("https://example.org");
    probe.insert(
        "targets".to_string(),
        json!([{
            "url": "ftp://example.org",
            "direct_ok": true,
            "recommended_route": "direct",
            "policy_hint": "domain_exact=example.org outbound=direct",
            "policy_apply_result": "not_requested",
            "policy_rule_id": "",
            "policy_verify_ok": false,
            "policy_verify_outbound": "",
            "target_error": ""
        }]),
    );
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(
        res.err()
            .is_some_and(|e| e.contains("target url scheme mismatch"))
    );
}

#[test]
fn rejects_probe_contract_empty_authority_url() {
    let mut probe = probe_with("https://example.org");
    probe.insert(
        "targets".to_string(),
        json!([{
            "url": "https:///missing-authority",
            "direct_ok": true,
            "recommended_route": "direct",
            "policy_hint": "domain_exact=example.org outbound=direct",
            "policy_apply_result": "not_requested",
            "policy_rule_id": "",
            "policy_verify_ok": false,
            "policy_verify_outbound": "",
            "target_error": ""
        }]),
    );
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(
        res.err()
            .is_some_and(|e| e.contains("target url scheme mismatch"))
    );
}

#[test]
fn rejects_applied_policy_without_verify_ok() {
    let mut probe = probe_with("https://example.org");
    probe.insert(
        "targets".to_string(),
        json!([{
            "url": "https://example.org",
            "direct_ok": true,
            "recommended_route": "direct",
            "policy_hint": "domain_exact=example.org outbound=direct",
            "policy_apply_result": "applied",
            "policy_rule_id": "probe-example-org",
            "policy_verify_ok": false,
            "policy_verify_outbound": "direct",
            "target_error": ""
        }]),
    );
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(
        res.err()
            .is_some_and(|e| e.contains("applied policy must verify ok"))
    );
}

#[test]
fn rejects_failed_policy_without_error() {
    let mut probe = probe_with("https://example.org");
    probe.insert(
        "targets".to_string(),
        json!([{
            "url": "https://example.org",
            "direct_ok": true,
            "recommended_route": "direct",
            "policy_hint": "domain_exact=example.org outbound=direct",
            "policy_apply_result": "failed",
            "policy_rule_id": "",
            "policy_verify_ok": false,
            "policy_verify_outbound": "",
            "target_error": ""
        }]),
    );
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(
        res.err()
            .is_some_and(|e| e.contains("failed policy must carry error"))
    );
}

#[test]
fn rejects_target_error_without_failed_policy() {
    let mut probe = probe_with("https://example.org");
    probe.insert(
        "targets".to_string(),
        json!([{
            "url": "https://example.org",
            "direct_ok": true,
            "recommended_route": "direct",
            "policy_hint": "domain_exact=example.org outbound=direct",
            "policy_apply_result": "not_requested",
            "policy_rule_id": "",
            "policy_verify_ok": false,
            "policy_verify_outbound": "",
            "target_error": "failover_override_update_error: boom"
        }]),
    );
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(
        res.err()
            .is_some_and(|e| e.contains("target_error requires failed policy"))
    );
}

#[test]
fn accepts_applied_policy_with_verified_route() {
    let mut probe = probe_with("https://example.org");
    probe.insert(
        "targets".to_string(),
        json!([{
            "url": "https://example.org",
            "direct_ok": true,
            "recommended_route": "direct",
            "policy_hint": "domain_exact=example.org outbound=direct",
            "policy_apply_result": "applied",
            "policy_rule_id": "probe-example-org",
            "policy_verify_ok": true,
            "policy_verify_outbound": "direct",
            "target_error": ""
        }]),
    );
    assert!(validate_probe_contract(&probe, CI_SNAPSHOT_HOST).is_ok());
}

#[test]
fn rejects_probe_contract_threshold_exceeded() {
    let mut probe = probe_with("https://example.org");
    probe.insert(
        "totals".to_string(),
        json!({
            "all": 1,
            "direct_ok": 0,
            "unreachable": 1,
            "policy_apply_failed": 0,
            "failed_total": 1,
            "fail_threshold": 0,
            "threshold_exceeded": true
        }),
    );
    probe.insert(
        "targets".to_string(),
        json!([{
            "url": "https://example.org",
            "direct_ok": false,
            "recommended_route": "transit",
            "policy_hint": "domain_exact=example.org outbound=transit",
            "policy_apply_result": "not_requested",
            "policy_rule_id": "",
            "policy_verify_ok": false,
            "policy_verify_outbound": "",
            "target_error": ""
        }]),
    );
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(
        res.err()
            .is_some_and(|e| e.contains("threshold_exceeded cannot ship"))
    );
}

#[test]
fn rejects_mixed_ci_snapshot_and_live_targets() {
    let mut probe = probe_with("https://chimera-ci-snapshot.local/ok");
    probe.insert(
        "totals".to_string(),
        json!({
            "all": 2,
            "direct_ok": 2,
            "unreachable": 0,
            "policy_apply_failed": 0,
            "failed_total": 0,
            "fail_threshold": 0,
            "threshold_exceeded": false
        }),
    );
    probe.insert(
        "targets".to_string(),
        json!([
            {
                "url": "https://chimera-ci-snapshot.local/ok",
                "direct_ok": true,
                "recommended_route": "direct",
                "policy_hint": "domain_exact=chimera-ci-snapshot.local outbound=direct",
                "policy_apply_result": "not_requested",
                "policy_rule_id": "",
                "policy_verify_ok": false,
                "policy_verify_outbound": "",
                "target_error": ""
            },
            {
                "url": "https://example.org",
                "direct_ok": true,
                "recommended_route": "direct",
                "policy_hint": "domain_exact=example.org outbound=direct",
                "policy_apply_result": "not_requested",
                "policy_rule_id": "",
                "policy_verify_ok": false,
                "policy_verify_outbound": "",
                "target_error": ""
            }
        ]),
    );
    let res = validate_probe_contract(&probe, CI_SNAPSHOT_HOST);
    assert!(res.is_err());
    assert!(
        res.err()
            .is_some_and(|e| e.contains("mixed ci_snapshot and live"))
    );
}
