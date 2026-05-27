use crate::MeshRuntime;

#[test]
fn runtime_dps_explain_marks_invalid_hints_status() {
    let runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let report = runtime.status_report_with_dps_payload("mesh_multipath_mode=broken");
    assert_eq!(report.preemptive_shadow_switch_mode, "unknown");
    assert_eq!(report.preemptive_shadow_hints_status, "invalid");
    assert_eq!(report.preemptive_shadow_hints_source, "invalid_payload");
    assert_eq!(report.preemptive_shadow_hints_reason, "dps_payload_invalid");
    assert!(!report.preemptive_shadow_hints_present);
    assert_eq!(report.preemptive_shadow_hints_multipath_mode, "invalid");
    assert_eq!(report.preemptive_shadow_hints_continuity_policy, "invalid");
    assert_eq!(
        report.preemptive_shadow_hints_summary,
        "status=invalid;present=false;reason=dps_payload_invalid;multipath_mode=invalid;continuity_policy=invalid;source=invalid_payload"
    );
}
