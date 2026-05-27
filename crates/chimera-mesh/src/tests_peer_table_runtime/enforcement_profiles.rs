use crate::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPeerHealth, MeshPeerTablePolicy,
    MeshRuntime,
};

#[test]
fn resilient_profile_protects_region_diversity_during_global_cap_drop() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy(MeshPeerTablePolicy {
                max_entries: 2,
                max_entries_per_region: 2,
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

    let prime = vec![
        MeshDiscoveryRecord {
            node_id: "eu-low".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 70,
            reliability_score: 40,
        },
        MeshDiscoveryRecord {
            node_id: "us-high-1".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "us".to_string(),
            load_score: 15,
            reliability_score: 92,
        },
        MeshDiscoveryRecord {
            node_id: "us-high-2".to_string(),
            endpoint: "198.51.100.12:443".to_string(),
            region: "us".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &prime).is_ok());

    assert!(
        runtime
            .update_health_state(&[MeshPeerHealth {
                node_id: "us-high-1".to_string(),
                healthy: false,
                cooldown_active: true,
            }])
            .is_ok()
    );

    assert!(runtime.merge_discovery("seed-c", &prime).is_ok());
    let snapshot = runtime.peer_snapshot();
    assert!(snapshot.iter().any(|p| p.node_id == "eu-low"));
    assert!(
        snapshot
            .iter()
            .any(|p| p.node_id == "us-high-1" || p.node_id == "us-high-2")
    );
    assert_eq!(runtime.peer_count(), 2);

    let report = runtime.peer_table_last_enforcement_report();
    assert_eq!(report.effective_profile, "resilient");
    assert_eq!(report.effective_target_source, "resilient:auto_raise");
    assert_eq!(report.effective_target_distinct_regions, 2);
    assert!(report.protected_region_skips >= 1);

    let status = runtime.status_explain();
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_effective_profile=resilient") })
    );
    assert!(status.iter().any(|line| {
        line.contains("status_table_enforcement_effective_target_source=resilient:auto_raise")
    }));
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_effective_target=2") })
    );
    assert!(status.iter().any(|line| {
        line.contains("status_table_enforcement_protected_region_skips=") && !line.ends_with("=0")
    }));
    assert!(status.iter().any(|line| {
        line.contains("status_table_enforcement_summary=")
            && line.contains("profile:resilient")
            && line.contains("target:2")
            && line.contains("source:resilient:auto_raise")
    }));
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_runtime_consistency_all_true=true") })
    );
}

#[test]
fn fast_profile_compacts_target_distinct_regions_for_enforcement() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy(MeshPeerTablePolicy {
                max_entries: 2,
                max_entries_per_region: 2,
                stale_after_ticks: 32,
                target_distinct_regions: 2,
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
            node_id: "node-eu".to_string(),
            endpoint: "198.51.100.20:443".to_string(),
            region: "eu".to_string(),
            load_score: 5,
            reliability_score: 98,
        },
        MeshDiscoveryRecord {
            node_id: "node-us".to_string(),
            endpoint: "198.51.100.21:443".to_string(),
            region: "us".to_string(),
            load_score: 6,
            reliability_score: 97,
        },
        MeshDiscoveryRecord {
            node_id: "node-ap".to_string(),
            endpoint: "198.51.100.22:443".to_string(),
            region: "ap".to_string(),
            load_score: 7,
            reliability_score: 96,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let report = runtime.peer_table_last_enforcement_report();
    assert_eq!(report.effective_profile, "fast");
    assert_eq!(report.effective_target_source, "fast:auto_compact");
    assert_eq!(report.effective_target_distinct_regions, 1);
    assert_eq!(report.total_peers_after, 2);

    let status = runtime.status_explain();
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_effective_profile=fast") })
    );
    assert!(status.iter().any(|line| {
        line.contains("status_table_enforcement_effective_target_source=fast:auto_compact")
    }));
    assert!(status.iter().any(|line| {
        line.contains("status_table_enforcement_summary=")
            && line.contains("profile:fast")
            && line.contains("target:1")
            && line.contains("source:fast:auto_compact")
    }));
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_runtime_consistency_all_true=true") })
    );
}

#[test]
fn resilient_score_only_applies_region_spread_bonus() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-us-1".to_string(),
            endpoint: "198.51.100.30:443".to_string(),
            region: "us".to_string(),
            load_score: 10,
            reliability_score: 60,
        },
        MeshDiscoveryRecord {
            node_id: "node-us-2".to_string(),
            endpoint: "198.51.100.31:443".to_string(),
            region: "us".to_string(),
            load_score: 11,
            reliability_score: 59,
        },
        MeshDiscoveryRecord {
            node_id: "node-eu-1".to_string(),
            endpoint: "198.51.100.32:443".to_string(),
            region: "eu".to_string(),
            load_score: 24,
            reliability_score: 62,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    assert!(
        runtime
            .update_health_state(&[MeshPeerHealth {
                node_id: "node-us-1".to_string(),
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
        allowed_regions: Vec::new(),
        blocked_node_ids: Vec::new(),
        require_min_reliability: 0,
        max_load_score: 100,
        max_peers: 1,
        prefer_region_diversity: false,
        max_selected_per_region: 1,
        min_distinct_regions: 1,
        path_profile_override: None,
        connect_fallback_ports: vec![443, 8443],
    };
    let plan = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 1);
    assert_eq!(plan.selected_peers[0].node_id, "node-eu-1");
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selection_strategy=score_only"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("resilient_region_spread_bonus_applied=true"))
    );
}
