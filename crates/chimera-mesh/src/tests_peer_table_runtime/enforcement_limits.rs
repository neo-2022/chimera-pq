use crate::{MeshDiscoveryRecord, MeshPeerTablePolicy, MeshRuntime};

#[test]
fn peer_table_enforces_max_entries_and_region_quota() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy(MeshPeerTablePolicy {
                max_entries: 3,
                max_entries_per_region: 1,
                stale_after_ticks: 32,
                target_distinct_regions: 3,
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
            node_id: "node-eu-1".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-eu-2".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "eu".to_string(),
            load_score: 15,
            reliability_score: 80,
        },
        MeshDiscoveryRecord {
            node_id: "node-us-1".to_string(),
            endpoint: "198.51.100.12:443".to_string(),
            region: "us".to_string(),
            load_score: 25,
            reliability_score: 85,
        },
        MeshDiscoveryRecord {
            node_id: "node-ap-1".to_string(),
            endpoint: "198.51.100.13:443".to_string(),
            region: "ap".to_string(),
            load_score: 30,
            reliability_score: 88,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    assert_eq!(runtime.peer_count(), 3);
    let distribution = runtime.region_distribution();
    assert_eq!(
        distribution,
        vec![
            ("ap".to_string(), 1),
            ("eu".to_string(), 1),
            ("us".to_string(), 1)
        ]
    );
    let report = runtime.peer_table_last_enforcement_report();
    assert_eq!(report.total_peers_before, 4);
    assert_eq!(report.total_peers_after, 3);
    assert_eq!(report.dropped_total, 1);
    assert_eq!(report.dropped_by_region_cap, 1);
    assert_eq!(report.dropped_by_global_cap, 0);
    assert_eq!(report.effective_profile, "balanced");
    assert_eq!(report.effective_target_distinct_regions, 3);
    assert_eq!(report.effective_target_source, "balanced:configured");

    let status = runtime.status_explain();
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_drop_breakdown_match=true") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_count_transition_valid=true") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_dropped_by_region_cap=1") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_dropped_by_global_cap=0") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_drop_components_sum=1") })
    );
    assert!(status.iter().any(|line| {
        line.contains("status_table_enforcement_drop_accounting=")
            && line.contains("total:1")
            && line.contains("components:1")
            && line.contains("delta:1")
    }));
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_drop_accounting_matches=true") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_total_peers_before=4") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_total_peers_after=3") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_drop_delta=1") })
    );
    assert!(
        status.iter().any(|line| {
            line.contains("status_table_enforcement_drop_delta_matches_total=true")
        })
    );
    assert!(status.iter().any(|line| {
        line.contains("status_table_enforcement_summary=")
            && line.contains("before:4")
            && line.contains("after:3")
            && line.contains("drop_total:1")
            && line.contains("drop_region:1")
            && line.contains("drop_global:0")
            && line.contains("source:balanced:configured")
    }));
}

#[test]
fn peer_table_enforcement_report_tracks_global_cap_drops() {
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
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "us".to_string(),
            load_score: 15,
            reliability_score: 88,
        },
        MeshDiscoveryRecord {
            node_id: "node-c".to_string(),
            endpoint: "198.51.100.12:443".to_string(),
            region: "ap".to_string(),
            load_score: 20,
            reliability_score: 85,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let report = runtime.peer_table_last_enforcement_report();
    assert_eq!(report.total_peers_before, 3);
    assert_eq!(report.total_peers_after, 2);
    assert_eq!(report.dropped_total, 1);
    assert_eq!(report.dropped_by_region_cap, 0);
    assert_eq!(report.dropped_by_global_cap, 1);
    assert_eq!(report.effective_profile, "balanced");
    assert_eq!(report.effective_target_distinct_regions, 1);
    assert_eq!(report.effective_target_source, "balanced:configured");

    let status = runtime.status_explain();
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_drop_breakdown_match=true") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_count_transition_valid=true") })
    );
    assert!(
        status
            .iter()
            .any(|line| line.contains("status_table_enforcement_effective_target_source="))
    );
    assert!(
        status
            .iter()
            .any(|line| line.contains("status_table_enforcement_tick="))
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_dropped_by_region_cap=0") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_dropped_by_global_cap=1") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_drop_components_sum=1") })
    );
    assert!(status.iter().any(|line| {
        line.contains("status_table_enforcement_drop_accounting=")
            && line.contains("total:1")
            && line.contains("components:1")
            && line.contains("delta:1")
    }));
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_drop_accounting_matches=true") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_protected_region_skips=0") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_total_peers_before=3") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_total_peers_after=2") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_drop_delta=1") })
    );
    assert!(
        status.iter().any(|line| {
            line.contains("status_table_enforcement_drop_delta_matches_total=true")
        })
    );
    assert!(status.iter().any(|line| {
        line.contains("status_table_enforcement_summary=")
            && line.contains("before:3")
            && line.contains("after:2")
            && line.contains("drop_total:1")
            && line.contains("drop_global:1")
    }));
}
