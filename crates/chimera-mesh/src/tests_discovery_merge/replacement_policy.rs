use crate::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPeerHealth, MeshPeerTablePolicy,
    MeshRuntime,
};

#[test]
fn merge_discovery_respects_replacement_score_delta_policy() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy(MeshPeerTablePolicy {
                max_entries: 16,
                max_entries_per_region: 16,
                stale_after_ticks: 8,
                target_distinct_regions: 1,
                replacement_min_score_delta: 5,
                degraded_replacement_min_score_delta: 1,
                max_replacements_per_window: 8,
                stability_window_ticks: 2,
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

    let small_gain = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.99:443".to_string(),
        region: "us".to_string(),
        load_score: 19,
        reliability_score: 80,
    }];
    assert!(runtime.merge_discovery("seed-c", &small_gain).is_ok());
    let after_small = runtime.peer_snapshot();
    assert_eq!(after_small[0].endpoint, "198.51.100.10:443");
    assert_eq!(after_small[0].region, "eu");

    let large_gain = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.99:443".to_string(),
        region: "us".to_string(),
        load_score: 15,
        reliability_score: 82,
    }];
    assert!(runtime.merge_discovery("seed-d", &large_gain).is_ok());
    let after_large = runtime.peer_snapshot();
    assert_eq!(after_large[0].endpoint, "198.51.100.99:443");
    assert_eq!(after_large[0].region, "us");

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
            .any(|line| line.contains("selected_peer_stability=node-a:u3:r1:h1:d0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_effective_replacement_thresholds=node-a:5"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_replacement_threshold_min=5"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_replacement_threshold_max=5"))
    );
    assert!(plan.explain.iter().any(|line| line.contains(
        "selected_replacement_decisions=node-a:replace1:hold1:churn_block0:threshold_block1"
    )));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_replacement_budget_remaining=node-a:7"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("stability_updates_total=3"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("stability_replacements_total=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("stability_holds_total=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("stability_degraded_total=0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("stability_churn_blocks_total=0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("stability_threshold_blocks_total=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("replacement_hold_ratio_pct=33"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("replacement_budget_remaining_total=7"))
    );
}

#[test]
fn merge_discovery_uses_degraded_replacement_threshold() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy(MeshPeerTablePolicy {
                max_entries: 16,
                max_entries_per_region: 16,
                stale_after_ticks: 8,
                target_distinct_regions: 1,
                replacement_min_score_delta: 5,
                degraded_replacement_min_score_delta: 1,
                max_replacements_per_window: 8,
                stability_window_ticks: 8,
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
    assert!(
        runtime
            .update_health_state(&[MeshPeerHealth {
                node_id: "node-a".to_string(),
                healthy: false,
                cooldown_active: true,
            }])
            .is_ok()
    );
    let small_gain = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.77:443".to_string(),
        region: "us".to_string(),
        load_score: 19,
        reliability_score: 80,
    }];
    assert!(runtime.merge_discovery("seed-c", &small_gain).is_ok());
    let snapshot = runtime.peer_snapshot();
    assert_eq!(snapshot[0].endpoint, "198.51.100.77:443");
    assert_eq!(snapshot[0].region, "us");

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
            .any(|line| line.contains("selected_peer_stability=node-a:u2:r1:h0:d1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_effective_replacement_thresholds=node-a:1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_replacement_threshold_min=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_replacement_threshold_max=1"))
    );
    assert!(plan.explain.iter().any(|line| line.contains(
        "selected_replacement_decisions=node-a:replace1:hold0:churn_block0:threshold_block0"
    )));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_replacement_budget_remaining=node-a:7"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("stability_degraded_total=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("stability_churn_blocks_total=0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("stability_threshold_blocks_total=0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("replacement_hold_ratio_pct=50"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("replacement_budget_remaining_total=7"))
    );
}

#[test]
fn merge_discovery_respects_max_replacements_per_window() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy(MeshPeerTablePolicy {
                max_entries: 16,
                max_entries_per_region: 16,
                stale_after_ticks: 8,
                target_distinct_regions: 1,
                replacement_min_score_delta: 1,
                degraded_replacement_min_score_delta: 1,
                max_replacements_per_window: 1,
                stability_window_ticks: 8,
                profile_hysteresis_ticks: 4,
                resilient_region_spread_bonus_weight: 10,
            })
            .is_ok()
    );
    assert!(
        runtime
            .merge_discovery(
                "seed-a",
                &[MeshDiscoveryRecord {
                    node_id: "node-a".to_string(),
                    endpoint: "198.51.100.10:443".to_string(),
                    region: "eu".to_string(),
                    load_score: 20,
                    reliability_score: 80,
                }],
            )
            .is_ok()
    );
    assert!(
        runtime
            .merge_discovery(
                "seed-b",
                &[MeshDiscoveryRecord {
                    node_id: "node-a".to_string(),
                    endpoint: "198.51.100.11:443".to_string(),
                    region: "us".to_string(),
                    load_score: 19,
                    reliability_score: 80,
                }],
            )
            .is_ok()
    );
    assert!(
        runtime
            .merge_discovery(
                "seed-c",
                &[MeshDiscoveryRecord {
                    node_id: "node-a".to_string(),
                    endpoint: "198.51.100.12:443".to_string(),
                    region: "ap".to_string(),
                    load_score: 18,
                    reliability_score: 80,
                }],
            )
            .is_ok()
    );
    let snapshot = runtime.peer_snapshot();
    assert_eq!(snapshot[0].endpoint, "198.51.100.11:443");
    assert_eq!(snapshot[0].region, "us");

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
    assert!(plan.explain.iter().any(|line| line.contains(
        "selected_replacement_decisions=node-a:replace1:hold1:churn_block1:threshold_block0"
    )));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_replacement_budget_remaining=node-a:0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("stability_churn_blocks_total=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("stability_threshold_blocks_total=0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("replacement_budget_remaining_total=0"))
    );
}
