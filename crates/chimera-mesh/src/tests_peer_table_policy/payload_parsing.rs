use crate::MeshPeerTablePolicy;

#[test]
fn peer_table_policy_from_dps_payload_parses_and_validates() {
    let payload = "mesh_table_max_entries=64;mesh_table_max_entries_per_region=8;mesh_table_stale_after_ticks=20;mesh_table_target_distinct_regions=2;mesh_table_replacement_min_score_delta=3;mesh_table_degraded_replacement_min_score_delta=2;mesh_table_max_replacements_per_window=12;mesh_table_stability_window_ticks=10;mesh_table_profile_hysteresis_ticks=6;mesh_table_resilient_region_spread_bonus_weight=7";
    let policy =
        MeshPeerTablePolicy::from_dps_payload(payload).unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(policy.max_entries, 64);
    assert_eq!(policy.max_entries_per_region, 8);
    assert_eq!(policy.stale_after_ticks, 20);
    assert_eq!(policy.target_distinct_regions, 2);
    assert_eq!(policy.replacement_min_score_delta, 3);
    assert_eq!(policy.degraded_replacement_min_score_delta, 2);
    assert_eq!(policy.max_replacements_per_window, 12);
    assert_eq!(policy.stability_window_ticks, 10);
    assert_eq!(policy.profile_hysteresis_ticks, 6);
    assert_eq!(policy.resilient_region_spread_bonus_weight, 7);
}

#[test]
fn peer_table_policy_from_dps_payload_rejects_invalid_values() {
    assert!(MeshPeerTablePolicy::from_dps_payload("").is_err());
    assert!(MeshPeerTablePolicy::from_dps_payload("mesh_table_max_entries=0").is_err());
    assert!(
        MeshPeerTablePolicy::from_dps_payload(
            "mesh_table_max_entries=4;mesh_table_target_distinct_regions=5"
        )
        .is_err()
    );
    assert!(
        MeshPeerTablePolicy::from_dps_payload(
            "mesh_table_replacement_min_score_delta=1;mesh_table_degraded_replacement_min_score_delta=2"
        )
        .is_err()
    );
    assert!(
        MeshPeerTablePolicy::from_dps_payload("mesh_table_resilient_region_spread_bonus_weight=0")
            .is_err()
    );
    assert!(MeshPeerTablePolicy::from_dps_payload("mesh_table_unknown_key=1").is_err());
    assert!(
        MeshPeerTablePolicy::from_dps_payload("mesh_table_max_entries=8;MESH_TABLE_MAX_ENTRIES=9")
            .is_err()
    );
}
