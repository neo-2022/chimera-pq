use crate::{
    MeshDiscoveryRecord, MeshFailoverEvent, MeshJoinRequest, MeshPathPolicy, MeshPeerHealth,
    MeshRuntime,
};

#[test]
fn reselection_respects_health_cooldown() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());

    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 80,
        max_load_score: 60,
        max_peers: 1,
        prefer_region_diversity: true,
        max_selected_per_region: 1,
        min_distinct_regions: 1,
        path_profile_override: None,
        connect_fallback_ports: vec![443, 8443],
    };
    let health = vec![MeshPeerHealth {
        node_id: "node-a".to_string(),
        healthy: true,
        cooldown_active: true,
    }];
    let plan = runtime
        .reselection_plan_with_health(&req, &policy, &health)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers[0].node_id, "node-b");
}

#[test]
fn update_health_state_rejects_invalid_node_id() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let bad = [MeshPeerHealth {
        node_id: "node,bad".to_string(),
        healthy: true,
        cooldown_active: false,
    }];
    assert!(runtime.update_health_state(&bad).is_err());
}

#[test]
fn failover_plan_rejects_invalid_failed_node_id() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.10:443".to_string(),
        region: "eu".to_string(),
        load_score: 10,
        reliability_score: 95,
    }];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 80,
        max_load_score: 60,
        max_peers: 1,
        prefer_region_diversity: true,
        max_selected_per_region: 1,
        min_distinct_regions: 1,
        path_profile_override: None,
        connect_fallback_ports: vec![443, 8443],
    };
    let bad_event = MeshFailoverEvent {
        failed_node_id: "node,\nbad".to_string(),
        reason: "health_probe_timeout".to_string(),
    };
    assert!(runtime.failover_plan(&req, &policy, &bad_event).is_err());
}
