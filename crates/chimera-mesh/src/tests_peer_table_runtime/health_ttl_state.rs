use crate::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPeerHealth, MeshPeerTablePolicy,
    MeshRuntime,
};

#[test]
fn persisted_health_state_expires_after_stale_window() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy(MeshPeerTablePolicy {
                max_entries: 16,
                max_entries_per_region: 16,
                stale_after_ticks: 1,
                target_distinct_regions: 1,
                replacement_min_score_delta: 1,
                degraded_replacement_min_score_delta: 1,
                max_replacements_per_window: 8,
                stability_window_ticks: 8,
                profile_hysteresis_ticks: 4,
                resilient_region_spread_bonus_weight: 10,
            })
            .is_ok()
    );
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
    assert!(
        runtime
            .update_health_state(&[MeshPeerHealth {
                node_id: "node-a".to_string(),
                healthy: false,
                cooldown_active: true,
            }])
            .is_ok()
    );
    assert_eq!(runtime.health_state_count(), 1);

    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let policy = MeshPathPolicy {
        allowed_regions: Vec::new(),
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

    let blocked_plan = runtime
        .reselection_plan_with_health(&req, &policy, &[])
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(blocked_plan.selected_peers[0].node_id, "node-b");

    assert!(runtime.merge_discovery("seed-c", &records).is_ok());
    assert_eq!(runtime.health_state_count(), 1);
    assert!(runtime.merge_discovery("seed-d", &records).is_ok());
    assert_eq!(runtime.health_state_count(), 0);

    let recovered_plan = runtime
        .reselection_plan_with_health(&req, &policy, &[])
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(recovered_plan.selected_peers[0].node_id, "node-a");
}

#[test]
fn health_snapshot_is_deterministic_and_reflects_ttl_eviction() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy(MeshPeerTablePolicy {
                max_entries: 16,
                max_entries_per_region: 16,
                stale_after_ticks: 1,
                target_distinct_regions: 1,
                replacement_min_score_delta: 1,
                degraded_replacement_min_score_delta: 1,
                max_replacements_per_window: 8,
                stability_window_ticks: 8,
                profile_hysteresis_ticks: 4,
                resilient_region_spread_bonus_weight: 10,
            })
            .is_ok()
    );
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    assert!(
        runtime
            .update_health_state(&[
                MeshPeerHealth {
                    node_id: "node-b".to_string(),
                    healthy: false,
                    cooldown_active: true,
                },
                MeshPeerHealth {
                    node_id: "node-a".to_string(),
                    healthy: true,
                    cooldown_active: true,
                },
            ])
            .is_ok()
    );

    let snapshot = runtime.health_snapshot();
    let ids: Vec<String> = snapshot.into_iter().map(|h| h.node_id).collect();
    assert_eq!(ids, vec!["node-a".to_string(), "node-b".to_string()]);

    assert!(runtime.merge_discovery("seed-c", &records).is_ok());
    assert_eq!(runtime.health_state_count(), 2);
    let snapshot_mid = runtime.health_snapshot();
    assert_eq!(snapshot_mid.len(), 2);

    assert!(runtime.merge_discovery("seed-d", &records).is_ok());
    assert_eq!(runtime.health_state_count(), 0);
    let snapshot_end = runtime.health_snapshot();
    assert!(snapshot_end.is_empty());
}
