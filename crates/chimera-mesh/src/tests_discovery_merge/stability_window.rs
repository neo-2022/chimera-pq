use crate::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPeerTablePolicy, MeshRuntime,
};

#[test]
fn stability_counters_reset_after_window_gap() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy(MeshPeerTablePolicy {
                max_entries: 16,
                max_entries_per_region: 16,
                stale_after_ticks: 16,
                target_distinct_regions: 1,
                replacement_min_score_delta: 5,
                degraded_replacement_min_score_delta: 1,
                max_replacements_per_window: 8,
                stability_window_ticks: 1,
                profile_hysteresis_ticks: 4,
                resilient_region_spread_bonus_weight: 10,
            })
            .is_ok()
    );

    let first = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.10:443".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 80,
    }];
    assert!(runtime.merge_discovery("seed-b", &first).is_ok());
    assert!(runtime.merge_discovery("seed-c", &[]).is_ok());
    assert!(runtime.merge_discovery("seed-d", &[]).is_ok());

    let later = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.10:443".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 80,
    }];
    assert!(runtime.merge_discovery("seed-e", &later).is_ok());

    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let policy = MeshPathPolicy {
        allowed_regions: Vec::new(),
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
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_peer_stability=node-a:u1:r0:h1:d0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_effective_replacement_thresholds=node-a:5"))
    );
}
