use super::tests_contract_constants::{
    BOOL_FALSE, BOOL_TRUE, SUCCESS_OPERATOR_ACTION, SUCCESS_OPERATOR_REASON, SUCCESS_PRESSURE,
    SUCCESS_SELECTED_NODE,
};
use super::tests_json_runner_utils::run_route_explain_json;
use super::tests_json_success_utils::assert_success_operator_health_block;
use super::tests_json_utils::{
    assert_health_summary_shape, assert_operator_summary_invariants,
    assert_route_explain_contract_blocks_presence, base_route_explain_args,
    expected_contract_family, expected_contract_version, expected_integrity_all_true,
    expected_kind_ok, expected_network_state, expected_operator_route_key_ok,
    expected_preemptive_shadow_degraded_summary, expected_status_ok,
    expected_table_runtime_consistency_summary, json_value, summary_field,
};

#[test]
fn mesh_route_explain_json_escapes_user_controlled_fields() {
    let parsed = run_route_explain_json(
        vec![
            "--namespace".to_string(),
            "cef\"public".to_string(),
            "--node".to_string(),
            "node\"client".to_string(),
            "--policy-payload".to_string(),
            "allow=mesh;mesh_max_peers=1".to_string(),
            "--peer".to_string(),
            "n1@198.51.100.1:443@eu@20@90".to_string(),
        ],
        0,
        "json_escape",
    );

    assert_eq!(parsed["namespace"], "cef\"public");
    assert_eq!(parsed["node"], "node\"client");
    assert_eq!(parsed["network_state"], expected_network_state());
}

#[test]
fn mesh_route_explain_json_degraded_fields_are_coherent() {
    let parsed = run_route_explain_json(
        base_route_explain_args("allow=mesh;mesh_max_peers=1"),
        0,
        "degraded_coherent",
    );
    let json = parsed.to_string();

    let is_degraded_true = json.contains(&format!(
        "\"preemptive_shadow_degraded_path\":\"{}\"",
        BOOL_TRUE
    ));
    let is_degraded_false = json.contains(&format!(
        "\"preemptive_shadow_degraded_path\":\"{}\"",
        BOOL_FALSE
    ));
    assert!(is_degraded_true || is_degraded_false);
    let gate_is_warn = json.contains("\"table_runtime_consistency_gate\":\"warn:");
    let gate_is_ok = json.contains("\"table_runtime_consistency_gate\":\"ok\"");
    assert!(gate_is_warn || gate_is_ok);
    let consistency_true = json.contains(&format!(
        "\"table_runtime_consistency_all_true\":\"{}\"",
        BOOL_TRUE
    ));
    let consistency_false = json.contains(&format!(
        "\"table_runtime_consistency_all_true\":\"{}\"",
        BOOL_FALSE
    ));
    assert!(consistency_true || consistency_false);
    if gate_is_ok {
        let expected_ok_summary =
            expected_table_runtime_consistency_summary(expected_status_ok(), BOOL_TRUE);
        let expected_degraded_summary = expected_preemptive_shadow_degraded_summary(
            BOOL_FALSE,
            SUCCESS_OPERATOR_REASON,
            expected_status_ok(),
            BOOL_TRUE,
        );
        assert!(json.contains(&format!(
            "\"table_runtime_consistency_summary\":\"{}\"",
            expected_ok_summary
        )));
        assert!(json.contains(&format!(
            "\"preemptive_shadow_degraded_summary\":\"{}\"",
            expected_degraded_summary
        )));
    } else {
        let expected_warn_summary = expected_table_runtime_consistency_summary("warn:", BOOL_FALSE);
        let expected_degraded_summary =
            expected_preemptive_shadow_degraded_summary(BOOL_TRUE, "warn:", "warn:", BOOL_FALSE);
        assert!(
            json.contains("\"table_runtime_consistency_summary\":\"gate=warn:")
                && json.contains(&format!(";all_true={}\"", BOOL_FALSE))
        );
        assert!(json.contains(&format!(
            "\"table_runtime_consistency_summary\":\"{}",
            expected_warn_summary
        )));
        assert!(
            json.contains("\"preemptive_shadow_degraded_summary\":\"path=true;reason=warn:")
                && json.contains(&format!(";all_true={}\"", BOOL_FALSE))
        );
        assert!(json.contains(&format!(
            "\"preemptive_shadow_degraded_summary\":\"{}",
            expected_degraded_summary
        )));
    }

    if is_degraded_true {
        assert!(gate_is_warn);
        assert!(consistency_false);
        assert!(json.contains("\"preemptive_shadow_degraded_reason\":\"warn:"));
    } else {
        let expected_reason = format!(
            "\"preemptive_shadow_degraded_reason\":\"{}\"",
            SUCCESS_OPERATOR_REASON
        );
        assert!(gate_is_ok);
        assert!(consistency_true);
        assert!(json.contains(&expected_reason));
    }

    let compact = json_value(&json, "plan_setup_discovery_table_compact").unwrap_or("");
    assert!(!compact.is_empty());
    assert!(compact.contains("join_mode:"));
    assert!(compact.contains("consistency_gate:"));
    assert!(compact.contains("degraded:"));
    let explain = json_value(&json, "explain").unwrap_or("");
    assert!(explain.contains(&format!("plan_setup_discovery_table_compact={compact}")));
}

#[test]
fn mesh_route_explain_json_success_snapshot_core() {
    let parsed = run_route_explain_json(
        base_route_explain_args("allow=mesh;mesh_max_peers=1"),
        0,
        "success_snapshot_core",
    );

    let snapshot = format!(
        "status={};kind={};family={};contract={};health={};operator={};integrity={};state={}",
        parsed["status"].as_str().unwrap_or(""),
        parsed["kind"].as_str().unwrap_or(""),
        parsed["route_explain_contract_family"]
            .as_str()
            .unwrap_or(""),
        parsed["explain_contract_version"].as_str().unwrap_or(""),
        parsed["route_explain_health_gate"].as_str().unwrap_or(""),
        parsed["route_explain_operator_summary"]
            .as_str()
            .unwrap_or(""),
        parsed["route_explain_contract_integrity"]
            .as_str()
            .unwrap_or(""),
        parsed["network_state"].as_str().unwrap_or(""),
    );

    let expected_snapshot = format!(
        "status={};kind={};family={};contract={};health={};operator=health:{};selected:{};pressure:{};action:{};reason:{};integrity={};state={}",
        expected_status_ok(),
        expected_kind_ok(),
        expected_contract_family(),
        expected_contract_version(),
        expected_status_ok(),
        expected_status_ok(),
        SUCCESS_SELECTED_NODE,
        SUCCESS_PRESSURE,
        SUCCESS_OPERATOR_ACTION,
        SUCCESS_OPERATOR_REASON,
        expected_integrity_all_true(),
        expected_network_state(),
    );
    assert_eq!(snapshot, expected_snapshot);
}

#[test]
fn mesh_route_explain_json_operator_summary_contract_ok() {
    let parsed = run_route_explain_json(
        base_route_explain_args("allow=mesh;mesh_max_peers=1"),
        0,
        "operator_summary_ok",
    );

    let summary = parsed["route_explain_operator_summary"]
        .as_str()
        .unwrap_or("");
    let signature = parsed["route_explain_operator_signature"]
        .as_str()
        .unwrap_or("");
    let route_key = parsed["route_explain_operator_route_key"]
        .as_str()
        .unwrap_or("");
    assert_eq!(summary_field(summary, "health"), Some(expected_status_ok()));
    assert_eq!(
        summary_field(summary, "selected"),
        Some(SUCCESS_SELECTED_NODE)
    );
    assert_eq!(
        summary_field(summary, "action"),
        Some(SUCCESS_OPERATOR_ACTION)
    );
    assert_eq!(
        summary_field(summary, "reason"),
        Some(SUCCESS_OPERATOR_REASON)
    );
    assert!(summary_field(summary, "pressure").is_some());
    assert_success_operator_health_block(
        &parsed,
        SUCCESS_SELECTED_NODE,
        SUCCESS_PRESSURE,
        SUCCESS_OPERATOR_ACTION,
        SUCCESS_OPERATOR_REASON,
    );
    assert_eq!(summary, signature);
    assert_eq!(
        route_key,
        expected_operator_route_key_ok(SUCCESS_OPERATOR_ACTION)
    );
    assert_health_summary_shape(
        parsed["route_explain_health_summary"]
            .as_str()
            .unwrap_or_default(),
    );
    assert_operator_summary_invariants(&parsed);
}

#[test]
fn mesh_route_explain_json_contract_blocks_are_present_and_non_empty() {
    let parsed = run_route_explain_json(
        base_route_explain_args("allow=mesh;mesh_max_peers=1"),
        0,
        "contract_blocks_presence",
    );

    assert_route_explain_contract_blocks_presence(&parsed);
}

#[test]
fn mesh_route_explain_json_connect_retry_plan_respects_policy_fallback_ports() {
    let parsed = run_route_explain_json(
        vec![
            "--namespace".to_string(),
            "cef-public".to_string(),
            "--node".to_string(),
            "node-client".to_string(),
            "--policy-payload".to_string(),
            "allow=mesh;mesh_max_peers=1;mesh_connect_fallback_ports=7443,443".to_string(),
            "--peer".to_string(),
            "n1@198.51.100.1:9443@eu@20@90".to_string(),
        ],
        0,
        "connect_retry_plan_policy_ports",
    );
    let explain = parsed["explain"].as_str().unwrap_or_default();
    let retry_plan = parsed["selected_peer_connect_retry_plan"]
        .as_str()
        .unwrap_or_default();
    let backoff_profile = parsed["selected_peer_connect_backoff_profile"]
        .as_str()
        .unwrap_or_default();
    assert!(
        explain.contains(
            "selected_peer_connect_retry_plan=n1@198.51.100.1:9443:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=9443|7443|443"
        ),
        "explain missing policy-driven retry port chain: {explain}"
    );
    assert_eq!(
        retry_plan,
        "n1@198.51.100.1:9443:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=9443|7443|443"
    );
    assert_eq!(
        backoff_profile,
        "initial=0ms;retry1=250ms;retry2=1000ms;jitter_step=50ms;fanout=1"
    );
}
