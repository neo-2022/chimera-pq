use super::*;
#[test]
fn runtime_status_explain_with_dps_payload_sets_shadow_switch_mode_hint() {
    let runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let lines = runtime.status_explain_with_dps_payload("mesh_multipath_mode=standby_only");
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_switch_mode=transport_only"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_standby_shadow_target_source="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_status=ok"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_reason=dps_payload_parsed"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_present=true"))
    );
    assert!(lines.iter().any(|line| {
        line.contains("status_preemptive_shadow_hints_multipath_mode=standby_only")
    }));
    assert!(
        lines
            .iter()
            .any(|line| { line.contains("status_preemptive_shadow_hints_continuity_policy=none") })
    );
    assert!(lines.iter().any(|line| {
        line.contains("status_preemptive_shadow_hints_summary=")
            && line.contains("status=ok")
            && line.contains("present=true")
            && line.contains("reason=dps_payload_parsed")
            && line.contains("multipath_mode=standby_only")
            && line.contains("continuity_policy=none")
    }));
    assert!(lines.iter().any(|line| {
        line.contains("status_standby_shadow_summary=")
            && line.contains("target_source:")
            && line.contains("stage:")
            && line.contains("trigger:")
    }));
}

#[test]
fn runtime_status_explain_with_dps_payload_marks_invalid_hints() {
    let runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let lines = runtime.status_explain_with_dps_payload("mesh_multipath_mode=broken");
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_switch_mode=unknown"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_status=invalid"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_source=invalid_payload"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_reason=dps_payload_invalid"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_present=false"))
    );
    assert!(
        lines
            .iter()
            .any(|line| { line.contains("status_preemptive_shadow_hints_multipath_mode=invalid") })
    );
    assert!(
        lines.iter().any(|line| {
            line.contains("status_preemptive_shadow_hints_continuity_policy=invalid")
        })
    );
    assert!(lines.iter().any(|line| {
        line.contains("status_preemptive_shadow_hints_summary=")
            && line.contains("status=invalid")
            && line.contains("present=false")
            && line.contains("reason=dps_payload_invalid")
            && line.contains("multipath_mode=invalid")
            && line.contains("continuity_policy=invalid")
    }));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_standby_shadow_mode="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_standby_shadow_target_source="))
    );
    assert!(lines.iter().any(|line| {
        line.contains("status_standby_shadow_summary=")
            && line.contains("target_source:")
            && line.contains("stage:")
            && line.contains("trigger:")
    }));
}
