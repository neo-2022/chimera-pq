use super::*;
#[test]
fn runtime_status_report_with_dps_payload_sets_shadow_switch_mode_hint() {
    let runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let report = runtime.status_report_with_dps_payload("mesh_continuity_policy=allow_flow_drain");
    assert_eq!(report.preemptive_shadow_switch_mode, "flow_drain");
    assert_eq!(report.preemptive_shadow_hints_status, "ok");
    assert_eq!(report.preemptive_shadow_hints_source, "dps_payload");
    assert_eq!(report.preemptive_shadow_hints_reason, "dps_payload_parsed");
    assert!(report.preemptive_shadow_hints_present);
    assert_eq!(report.preemptive_shadow_hints_multipath_mode, "none");
    assert_eq!(
        report.preemptive_shadow_hints_continuity_policy,
        "allow_flow_drain"
    );
    assert_eq!(
        report.preemptive_shadow_hints_summary,
        "status=ok;present=true;reason=dps_payload_parsed;multipath_mode=none;continuity_policy=allow_flow_drain;source=dps_payload"
    );
    assert_eq!(report.standby_shadow_source, "dps_multipath_policy");
    assert!(
        report.standby_shadow_target_source == "switch_target"
            || report.standby_shadow_target_source == "selected_primary"
            || report.standby_shadow_target_source == "selected_secondary"
            || report.standby_shadow_target_source == "none"
    );
    assert!(report.standby_shadow_summary.contains("target_source:"));
    assert!(report.standby_shadow_summary.contains("stage:"));
    assert!(report.standby_shadow_summary.contains("trigger:"));
}

#[test]
fn runtime_status_report_with_dps_payload_marks_no_hints_reason() {
    let runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let report = runtime.status_report_with_dps_payload("mesh_max_peers=1");
    assert_eq!(report.preemptive_shadow_hints_status, "ok");
    assert_eq!(report.preemptive_shadow_hints_source, "dps_payload");
    assert_eq!(
        report.preemptive_shadow_hints_reason,
        "dps_payload_no_hints"
    );
    assert!(!report.preemptive_shadow_hints_present);
    assert_eq!(report.preemptive_shadow_hints_multipath_mode, "none");
    assert_eq!(report.preemptive_shadow_hints_continuity_policy, "none");
    assert_eq!(
        report.preemptive_shadow_hints_summary,
        "status=ok;present=false;reason=dps_payload_no_hints;multipath_mode=none;continuity_policy=none;source=dps_payload"
    );
    assert_eq!(report.standby_shadow_source, "preemptive_shadow");
    assert!(!report.standby_shadow_summary.is_empty());
}
