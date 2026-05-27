use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPeerHealth, MeshRuntime};

#[test]
fn plan_manual_override_disables_health_filtering() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-degraded".to_string(),
        endpoint: "198.51.100.10:443".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 95,
    }];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    assert!(
        runtime
            .update_health_state(&[MeshPeerHealth {
                node_id: "node-degraded".to_string(),
                healthy: false,
                cooldown_active: true,
            }])
            .is_ok()
    );
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 90,
        max_load_score: 30,
        max_peers: 1,
        prefer_region_diversity: true,
        max_selected_per_region: 1,
        min_distinct_regions: 1,
        path_profile_override: None,
        connect_fallback_ports: vec![443, 8443],
    };
    let plan = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 1);
    assert_eq!(plan.selected_peers[0].node_id, "node-degraded");
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_health_filter_source=manual_disabled"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_selected_per_region=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_health_relax_applied=false"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_health_relax_reason=manual_override_disabled"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_health_relax_stage=none"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_attempts=0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_triggered=false"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_final_result=not_triggered"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_trace=none"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_trace_steps=0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_trace_consistent=true"))
    );
}
