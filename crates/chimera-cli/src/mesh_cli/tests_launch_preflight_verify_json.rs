#![forbid(unsafe_code)]

use std::fs;

use super::tests_json_runner_utils::run_mesh_subcommand_json;
use super::tests_json_utils::temp_out_file;

fn write_json(path: &std::path::Path, content: &str) {
    fs::write(path, content)
        .unwrap_or_else(|e| unreachable!("write json fixture should succeed: {e}"));
}

fn run_verify(
    side_a: &std::path::Path,
    side_b: &std::path::Path,
    expected_rc: i32,
    label: &str,
) -> serde_json::Value {
    run_mesh_subcommand_json(
        "launch-preflight-verify",
        vec![
            "--side-a-report".to_string(),
            side_a.to_string_lossy().to_string(),
            "--side-b-report".to_string(),
            side_b.to_string_lossy().to_string(),
        ],
        expected_rc,
        label,
    )
}

#[test]
fn launch_preflight_verify_ready_when_both_ready() {
    let side_a = temp_out_file("side_a_ready");
    let side_b = temp_out_file("side_b_ready");
    write_json(
        &side_a,
        r#"{"status":"ready","namespace":"cef-public","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}"#,
    );
    write_json(
        &side_b,
        r#"{"status":"ready","namespace":"cef-public","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}"#,
    );
    let parsed = run_verify(&side_a, &side_b, 0, "verify_ready");
    let _ = fs::remove_file(&side_a);
    let _ = fs::remove_file(&side_b);
    assert_eq!(parsed["status"].as_str().unwrap_or(""), "ready");
    assert_eq!(parsed["all_ready"].as_bool(), Some(true));
    assert_eq!(parsed["side_a_ready"].as_bool(), Some(true));
    assert_eq!(parsed["side_b_ready"].as_bool(), Some(true));
    assert_eq!(parsed["namespace"].as_str().unwrap_or(""), "cef-public");
    assert_eq!(
        parsed["blockers"]
            .as_array()
            .map(|v| v.len())
            .unwrap_or(999),
        0
    );
}

#[test]
fn launch_preflight_verify_blocked_when_one_blocked() {
    let side_a = temp_out_file("side_a_blocked");
    let side_b = temp_out_file("side_b_blocked");
    write_json(
        &side_a,
        r#"{"status":"blocked","namespace":"cef-public","ready_for_real_launch":false,"connect_probe_success":false,"network_state":"not_modified","blockers":["connectivity_probe_failed"]}"#,
    );
    write_json(
        &side_b,
        r#"{"status":"ready","namespace":"cef-public","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}"#,
    );
    let parsed = run_verify(&side_a, &side_b, 1, "verify_blocked");
    let _ = fs::remove_file(&side_a);
    let _ = fs::remove_file(&side_b);
    assert_eq!(parsed["status"].as_str().unwrap_or(""), "blocked");
    assert_eq!(parsed["all_ready"].as_bool(), Some(false));
    assert_eq!(parsed["side_a_ready"].as_bool(), Some(false));
    assert_eq!(parsed["side_b_ready"].as_bool(), Some(true));
    let blockers = parsed["blockers"]
        .as_array()
        .unwrap_or_else(|| unreachable!("blockers should be array"));
    assert!(
        blockers
            .iter()
            .any(|v| v.as_str() == Some("side_a_report_not_ready"))
    );
}

#[test]
fn launch_preflight_verify_blocked_on_namespace_mismatch() {
    let side_a = temp_out_file("side_a_ns_mismatch");
    let side_b = temp_out_file("side_b_ns_mismatch");
    write_json(
        &side_a,
        r#"{"status":"ready","namespace":"cef-public-a","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}"#,
    );
    write_json(
        &side_b,
        r#"{"status":"ready","namespace":"cef-public-b","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}"#,
    );
    let parsed = run_verify(&side_a, &side_b, 1, "verify_ns_mismatch");
    let _ = fs::remove_file(&side_a);
    let _ = fs::remove_file(&side_b);
    assert_eq!(parsed["status"].as_str().unwrap_or(""), "blocked");
    assert_eq!(parsed["all_ready"].as_bool(), Some(false));
    assert_eq!(parsed["side_a_ready"].as_bool(), Some(true));
    assert_eq!(parsed["side_b_ready"].as_bool(), Some(true));
    let blockers = parsed["blockers"]
        .as_array()
        .unwrap_or_else(|| unreachable!("blockers should be array"));
    assert!(
        blockers
            .iter()
            .any(|v| v.as_str() == Some("namespace_mismatch"))
    );
}
