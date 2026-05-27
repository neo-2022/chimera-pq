use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshRuntime};

#[test]
fn runtime_dps_explain_shadow_switch_mode_prefers_continuity_policy() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-eu".to_string(),
        endpoint: "198.51.100.80:443".to_string(),
        region: "eu".to_string(),
        load_score: 10,
        reliability_score: 95,
    }];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let payload = "mesh_allowed_regions=eu;mesh_max_peers=1;mesh_max_selected_per_region=1;mesh_continuity_policy=allow_flow_drain;mesh_multipath_mode=standby_only";
    let plan = runtime
        .plan_path_from_dps_payload(&req, payload)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_shadow_switch_mode=flow_drain") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_hints_status=ok") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_status=ok") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_present=true") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_reason=dps_payload_parsed") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_source=dps_payload") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_multipath_mode=standby_only") })
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("preemptive_shadow_hints_continuity_policy=allow_flow_drain")
    }));
    assert!(plan.explain.iter().any(|line| {
        line.contains("preemptive_shadow_hints_summary=")
            && line.contains("status=ok")
            && line.contains("present=true")
            && line.contains("reason=dps_payload_parsed")
            && line.contains("multipath_mode=standby_only")
            && line.contains("continuity_policy=allow_flow_drain")
            && line.contains("source=dps_payload")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_switch_mode=flow_drain") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_hints_present=true") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_hints_reason=dps_payload_parsed") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_switch_guard="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_switch_guard_source="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_switch_guard_summary="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_confirm_summary="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_risk_summary="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_switch_confidence_summary=") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_tuning_summary="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_table_runtime_consistency_gate=ok"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_table_runtime_consistency_all_true=true"))
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_table_runtime_consistency_summary=gate=ok;all_true=true")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_preemptive_degraded_path=false"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_preemptive_degraded_reason=none"))
    );
    let dps_consistency_summary = plan
        .explain
        .iter()
        .find_map(|line| line.strip_prefix("dps_payload_table_runtime_consistency_summary="))
        .unwrap_or("");
    let dps_degraded_summary = plan
        .explain
        .iter()
        .find_map(|line| line.strip_prefix("dps_payload_preemptive_degraded_summary="))
        .unwrap_or("");
    assert!(!dps_consistency_summary.is_empty());
    assert!(dps_degraded_summary.ends_with(dps_consistency_summary));
    assert!(plan.explain.iter().any(|line| {
        line.contains(
            "dps_payload_preemptive_degraded_summary=path=false;reason=none;gate=ok;all_true=true",
        )
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_standby_summary=mode="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_standby_mode="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_standby_target_source="))
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_standby_target_source=")
            && (line.contains("selected_primary")
                || line.contains("selected_secondary")
                || line.contains("switch_target"))
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_standby_stage_source=stage:") })
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_standby_stage_source=") && line.contains("trigger:")
    }));
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_standby_summary=") && line.contains("stage_source=")
    }));
}
