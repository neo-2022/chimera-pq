use crate::{MeshDiscoveryRecord, MeshPeerTablePolicy, MeshRuntime};

#[test]
fn peer_table_evicts_stale_entries() {
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

    assert!(
        runtime
            .merge_discovery(
                "seed-a",
                &[MeshDiscoveryRecord {
                    node_id: "node-old".to_string(),
                    endpoint: "198.51.100.1:443".to_string(),
                    region: "eu".to_string(),
                    load_score: 10,
                    reliability_score: 90,
                }],
            )
            .is_ok()
    );
    assert_eq!(runtime.peer_count(), 1);

    assert!(
        runtime
            .merge_discovery(
                "seed-b",
                &[MeshDiscoveryRecord {
                    node_id: "node-new".to_string(),
                    endpoint: "198.51.100.2:443".to_string(),
                    region: "us".to_string(),
                    load_score: 10,
                    reliability_score: 90,
                }],
            )
            .is_ok()
    );
    assert_eq!(runtime.peer_count(), 2);

    assert!(runtime.merge_discovery("seed-c", &[]).is_ok());
    assert_eq!(runtime.peer_count(), 1);
}
