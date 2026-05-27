#![forbid(unsafe_code)]

use std::fs;

use super::tests_json_runner_utils::run_mesh_subcommand_json;
use super::tests_json_utils::temp_out_file;

fn write_json(path: &std::path::Path, content: &str) {
    fs::write(path, content)
        .unwrap_or_else(|e| unreachable!("write json fixture should succeed: {e}"));
}

fn run_verify(
    vps: &std::path::Path,
    laptop: &std::path::Path,
    expected_rc: i32,
    label: &str,
) -> serde_json::Value {
    run_mesh_subcommand_json(
        "launch-preflight-verify",
        vec![
            "--vps-report".to_string(),
            vps.to_string_lossy().to_string(),
            "--laptop-report".to_string(),
            laptop.to_string_lossy().to_string(),
        ],
        expected_rc,
        label,
    )
}

#[test]
fn launch_preflight_verify_ready_when_both_ready() {
    let vps = temp_out_file("vps_ready");
    let laptop = temp_out_file("laptop_ready");
    write_json(
        &vps,
        r#"{"status":"ready","namespace":"cef-public","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}"#,
    );
    write_json(
        &laptop,
        r#"{"status":"ready","namespace":"cef-public","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}"#,
    );
    let parsed = run_verify(&vps, &laptop, 0, "verify_ready");
    let _ = fs::remove_file(&vps);
    let _ = fs::remove_file(&laptop);
    assert_eq!(parsed["status"].as_str().unwrap_or(""), "ready");
    assert_eq!(parsed["all_ready"].as_bool(), Some(true));
    assert_eq!(parsed["vps_ready"].as_bool(), Some(true));
    assert_eq!(parsed["laptop_ready"].as_bool(), Some(true));
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
    let vps = temp_out_file("vps_blocked");
    let laptop = temp_out_file("laptop_blocked");
    write_json(
        &vps,
        r#"{"status":"blocked","namespace":"cef-public","ready_for_real_launch":false,"connect_probe_success":false,"network_state":"not_modified","blockers":["connectivity_probe_failed"]}"#,
    );
    write_json(
        &laptop,
        r#"{"status":"ready","namespace":"cef-public","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}"#,
    );
    let parsed = run_verify(&vps, &laptop, 1, "verify_blocked");
    let _ = fs::remove_file(&vps);
    let _ = fs::remove_file(&laptop);
    assert_eq!(parsed["status"].as_str().unwrap_or(""), "blocked");
    assert_eq!(parsed["all_ready"].as_bool(), Some(false));
    assert_eq!(parsed["vps_ready"].as_bool(), Some(false));
    assert_eq!(parsed["laptop_ready"].as_bool(), Some(true));
    let blockers = parsed["blockers"]
        .as_array()
        .unwrap_or_else(|| unreachable!("blockers should be array"));
    assert!(
        blockers
            .iter()
            .any(|v| v.as_str() == Some("vps_report_not_ready"))
    );
}

#[test]
fn launch_preflight_verify_blocked_on_namespace_mismatch() {
    let vps = temp_out_file("vps_ns_mismatch");
    let laptop = temp_out_file("laptop_ns_mismatch");
    write_json(
        &vps,
        r#"{"status":"ready","namespace":"cef-public-a","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}"#,
    );
    write_json(
        &laptop,
        r#"{"status":"ready","namespace":"cef-public-b","ready_for_real_launch":true,"connect_probe_success":true,"network_state":"not_modified","blockers":[]}"#,
    );
    let parsed = run_verify(&vps, &laptop, 1, "verify_ns_mismatch");
    let _ = fs::remove_file(&vps);
    let _ = fs::remove_file(&laptop);
    assert_eq!(parsed["status"].as_str().unwrap_or(""), "blocked");
    assert_eq!(parsed["all_ready"].as_bool(), Some(false));
    assert_eq!(parsed["vps_ready"].as_bool(), Some(true));
    assert_eq!(parsed["laptop_ready"].as_bool(), Some(true));
    let blockers = parsed["blockers"]
        .as_array()
        .unwrap_or_else(|| unreachable!("blockers should be array"));
    assert!(
        blockers
            .iter()
            .any(|v| v.as_str() == Some("namespace_mismatch"))
    );
}
