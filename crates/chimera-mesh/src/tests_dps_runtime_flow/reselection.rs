use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshPeerHealth, MeshRuntime};

#[test]
fn runtime_reselection_plan_with_health_from_dps_payload_builds_route() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.70:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.71:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let payload = "mesh_allowed_regions=eu;mesh_max_peers=1;mesh_max_selected_per_region=1;mesh_min_distinct_regions=1";
    let health = [MeshPeerHealth {
        node_id: "node-a".to_string(),
        healthy: false,
        cooldown_active: true,
    }];
    let plan = runtime
        .reselection_plan_with_health_from_dps_payload(&req, payload, &health)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 1);
    assert_eq!(plan.selected_peers[0].node_id, "node-b");
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("health_reselection_applied=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_origin=reselection"))
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
}

#[test]
fn runtime_reselection_plan_with_health_from_dps_payload_rejects_invalid_policy() {
    let runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    assert!(
        runtime
            .reselection_plan_with_health_from_dps_payload(&req, "mesh_max_peers=0", &[])
            .is_err()
    );
    assert!(
        runtime
            .reselection_plan_with_health_from_dps_payload(&req, "allow=mesh", &[])
            .is_err()
    );
}
