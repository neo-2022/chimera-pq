use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPeerHealth, MeshRuntime};

#[test]
fn plan_auto_excludes_degraded_peers_from_candidates() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-degraded".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 100,
            reliability_score: 0,
        },
        MeshDiscoveryRecord {
            node_id: "node-healthy".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "us".to_string(),
            load_score: 25,
            reliability_score: 90,
        },
    ];
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
    let plan = runtime
        .plan_path(&req, &MeshPathPolicy::default_auto())
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 2);
    assert!(
        plan.selected_peers
            .iter()
            .any(|peer| peer.node_id == "node-healthy")
    );
    assert!(
        plan.selected_peers
            .iter()
            .any(|peer| peer.node_id == "node-degraded")
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_relaxed_filters=health"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_health_blocked_candidates=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_health_blocked_node_ids=node-degraded"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_health_relax_applied=true"))
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("effective_health_relax_reason=resilient_health_relax")
            || line.contains("effective_health_relax_reason=last_chance_health_relax")
    }));
    assert!(plan.explain.iter().any(|line| {
        line.contains("effective_health_relax_stage=primary")
            || line.contains("effective_health_relax_stage=last_chance")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_attempts=1")
                || line.contains("auto_recovery_attempts=2")
                || line.contains("auto_recovery_attempts=3"))
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("auto_recovery_final_result=selected_from_relaxed_health")
            || line.contains("auto_recovery_final_result=selected_from_last_chance_health")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_health_filter_source=auto"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_result=selected_from_relaxed_health"))
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("candidate_summary=accepted:2,rejected_blocked:0,rejected_health:0")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("peer=node-degraded rejected=health"))
    );
}

#[test]

fn plan_auto_resilient_can_fill_capacity_with_health_relax() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-degraded".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-healthy".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "us".to_string(),
            load_score: 25,
            reliability_score: 90,
        },
    ];
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
    let plan = runtime
        .plan_path(&req, &MeshPathPolicy::default_auto())
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 2);
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_relaxed_filters=health"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_result=selected_from_relaxed_health"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_health_relax_applied=true"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_health_relax_reason=resilient_health_relax"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_health_relax_stage=primary"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_attempts=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_triggered=true"))
    );
    assert!(
        plan.explain.iter().any(|line| {
            line.contains("auto_recovery_final_result=selected_from_relaxed_health")
        })
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("auto_recovery_trace=primary:health->primary:selected_from_relaxed_health")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_trace_steps=2"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("auto_recovery_trace_consistent=true"))
    );
}
