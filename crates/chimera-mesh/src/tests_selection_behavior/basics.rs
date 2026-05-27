use crate::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPeerTablePolicy, MeshRuntime,
};

#[test]
fn policy_region_filter_is_case_insensitive() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-eu".to_string(),
        endpoint: "198.51.100.10:443".to_string(),
        region: "EU".to_string(),
        load_score: 10,
        reliability_score: 90,
    }];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 70,
        max_load_score: 60,
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
    assert_eq!(plan.selected_peers[0].node_id, "node-eu");
}

#[test]
fn region_quota_treats_mixed_case_regions_as_same_region() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy(MeshPeerTablePolicy {
                max_entries: 4,
                max_entries_per_region: 1,
                stale_after_ticks: 32,
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
            node_id: "node-eu-upper".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "EU".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-eu-lower".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-us".to_string(),
            endpoint: "198.51.100.12:443".to_string(),
            region: "us".to_string(),
            load_score: 30,
            reliability_score: 85,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    assert_eq!(runtime.peer_count(), 2);
    let distribution = runtime.region_distribution();
    assert_eq!(
        distribution,
        vec![("eu".to_string(), 1), ("us".to_string(), 1)]
    );
}

#[test]
fn peer_snapshot_is_deterministic_by_node_id() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-c".to_string(),
            endpoint: "198.51.100.13:443".to_string(),
            region: "eu".to_string(),
            load_score: 30,
            reliability_score: 80,
        },
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.12:443".to_string(),
            region: "us".to_string(),
            load_score: 20,
            reliability_score: 85,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let snapshot = runtime.peer_snapshot();
    let ids: Vec<String> = snapshot.into_iter().map(|peer| peer.node_id).collect();
    assert_eq!(
        ids,
        vec![
            "node-a".to_string(),
            "node-b".to_string(),
            "node-c".to_string()
        ]
    );
}
