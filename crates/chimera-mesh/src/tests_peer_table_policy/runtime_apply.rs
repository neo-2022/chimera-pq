use crate::{MeshDiscoveryRecord, MeshRuntime};

#[test]
fn runtime_applies_peer_table_policy_from_dps_payload() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let payload = "mesh_table_max_entries=2;mesh_table_max_entries_per_region=2;mesh_table_target_distinct_regions=1;mesh_table_stale_after_ticks=32;mesh_table_replacement_min_score_delta=1;mesh_table_degraded_replacement_min_score_delta=1;mesh_table_max_replacements_per_window=8;mesh_table_stability_window_ticks=8;mesh_table_profile_hysteresis_ticks=4;mesh_table_resilient_region_spread_bonus_weight=7";
    assert!(
        runtime
            .set_peer_table_policy_from_dps_payload(payload)
            .is_ok()
    );

    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.40:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.41:443".to_string(),
            region: "us".to_string(),
            load_score: 11,
            reliability_score: 94,
        },
        MeshDiscoveryRecord {
            node_id: "node-c".to_string(),
            endpoint: "198.51.100.42:443".to_string(),
            region: "ap".to_string(),
            load_score: 12,
            reliability_score: 93,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    assert_eq!(runtime.peer_count(), 2);
}

#[test]
fn runtime_rejects_invalid_peer_table_policy_payload() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy_from_dps_payload(
                "mesh_table_resilient_region_spread_bonus_weight=0"
            )
            .is_err()
    );
}

#[test]
fn runtime_peer_table_policy_snapshot_returns_applied_policy() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let payload = "mesh_table_max_entries=9;mesh_table_max_entries_per_region=3;mesh_table_target_distinct_regions=2;mesh_table_stale_after_ticks=32;mesh_table_replacement_min_score_delta=2;mesh_table_degraded_replacement_min_score_delta=1;mesh_table_max_replacements_per_window=7;mesh_table_stability_window_ticks=6;mesh_table_profile_hysteresis_ticks=5;mesh_table_resilient_region_spread_bonus_weight=11";
    assert!(
        runtime
            .set_peer_table_policy_from_dps_payload(payload)
            .is_ok()
    );
    let policy = runtime.peer_table_policy_snapshot();
    assert_eq!(policy.max_entries, 9);
    assert_eq!(policy.max_entries_per_region, 3);
    assert_eq!(policy.target_distinct_regions, 2);
    assert_eq!(policy.resilient_region_spread_bonus_weight, 11);
}
