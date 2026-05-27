use crate::{MeshDiscoveryRecord, MeshFailoverEvent, MeshJoinRequest, MeshPeerHealth, MeshRuntime};

#[test]
fn runtime_evaluate_dps_policy_payload_is_dry_run_with_marker() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-eu".to_string(),
            endpoint: "198.51.100.80:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-us".to_string(),
            endpoint: "198.51.100.81:443".to_string(),
            region: "us".to_string(),
            load_score: 12,
            reliability_score: 92,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let before_peers = runtime.peer_count();
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let payload = "mesh_allowed_regions=eu;mesh_max_peers=1;mesh_max_selected_per_region=1;mesh_min_distinct_regions=1";
    let plan = runtime
        .evaluate_dps_policy_payload(&req, payload)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 1);
    assert_eq!(plan.selected_peers[0].node_id, "node-eu");
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_policy_evaluation=true"))
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
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_standby_stage_source=stage:"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_hints_source=dps_payload"))
    );
    assert_eq!(runtime.peer_count(), before_peers);
}

#[test]
fn runtime_evaluate_dps_policy_payload_with_health_is_dry_run_with_marker() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.90:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.91:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let before_peers = runtime.peer_count();
    let before_health = runtime.health_state_count();
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
        .evaluate_dps_policy_payload_with_health(&req, payload, &health)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 1);
    assert_eq!(plan.selected_peers[0].node_id, "node-b");
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_policy_evaluation_with_health=true"))
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
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_standby_stage_source=stage:"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_hints_source=dps_payload"))
    );
    assert_eq!(runtime.peer_count(), before_peers);
    assert_eq!(runtime.health_state_count(), before_health);
}

#[test]
fn runtime_evaluate_dps_failover_payload_is_dry_run_with_marker() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-eu".to_string(),
            endpoint: "198.51.100.100:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-us".to_string(),
            endpoint: "198.51.100.101:443".to_string(),
            region: "us".to_string(),
            load_score: 12,
            reliability_score: 92,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let before_peers = runtime.peer_count();
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
        .evaluate_dps_failover_payload(&req, payload, &event)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 1);
    assert_eq!(plan.selected_peers[0].node_id, "node-us");
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_policy_evaluation_failover=true"))
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
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_standby_stage_source=stage:"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_hints_source=dps_payload"))
    );
    assert_eq!(runtime.peer_count(), before_peers);
}
