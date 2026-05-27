use crate::{MeshDiscoveryRecord, MeshFailoverEvent, MeshJoinRequest, MeshRuntime};

#[test]
fn runtime_failover_plan_from_dps_payload_builds_replacement() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-eu".to_string(),
            endpoint: "198.51.100.60:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-us".to_string(),
            endpoint: "198.51.100.61:443".to_string(),
            region: "us".to_string(),
            load_score: 11,
            reliability_score: 94,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let event = MeshFailoverEvent {
        failed_node_id: "node-eu".to_string(),
        reason: "probe_timeout".to_string(),
    };
    let payload = "mesh_max_peers=1;mesh_max_selected_per_region=1;mesh_min_distinct_regions=1";
    let plan = runtime
        .failover_plan_from_dps_payload(&req, payload, &event)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 1);
    assert_eq!(plan.selected_peers[0].node_id, "node-us");
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("failover_triggered node=node-eu reason=probe_timeout"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_origin=failover"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_hints_status=ok"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_hints_status=ok"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_hints_present=false"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_reason=dps_payload_no_hints") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_source=dps_payload") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_source=dps_payload") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_multipath_mode=none") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_continuity_policy=none") })
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("preemptive_shadow_hints_summary=")
            && line.contains("status=ok")
            && line.contains("present=false")
            && line.contains("reason=dps_payload_no_hints")
            && line.contains("multipath_mode=none")
            && line.contains("continuity_policy=none")
            && line.contains("source=dps_payload")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_switch_mode=unknown") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_hints_present=false"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_hints_reason=dps_payload_no_hints"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_hints_source=dps_payload"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_hints_source=dps_payload"))
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_hints_summary=")
            && line.contains("multipath_mode=none")
            && line.contains("continuity_policy=none")
            && line.contains("source=dps_payload")
    }));
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
    assert!(plan.explain.iter().any(|line| {
        line.contains(
            "dps_payload_preemptive_degraded_summary=path=false;reason=none;gate=ok;all_true=true",
        )
    }));
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
                || line.contains("switch_target")
                || line.contains("none"))
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

#[test]
fn runtime_failover_plan_from_dps_payload_rejects_invalid_policy() {
    let runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let event = MeshFailoverEvent {
        failed_node_id: "node-a".to_string(),
        reason: "probe_timeout".to_string(),
    };
    assert!(
        runtime
            .failover_plan_from_dps_payload(&req, "mesh_max_peers=0", &event)
            .is_err()
    );
    assert!(
        runtime
            .failover_plan_from_dps_payload(&req, "allow=mesh", &event)
            .is_err()
    );
}
