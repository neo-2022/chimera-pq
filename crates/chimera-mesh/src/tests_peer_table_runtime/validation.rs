use crate::MeshPeerTablePolicy;

#[test]
fn peer_table_policy_rejects_invalid_target_distinct_regions() {
    assert!(
        MeshPeerTablePolicy {
            max_entries: 4,
            max_entries_per_region: 2,
            stale_after_ticks: 8,
            target_distinct_regions: 0,
            replacement_min_score_delta: 1,
            degraded_replacement_min_score_delta: 1,
            max_replacements_per_window: 8,
            stability_window_ticks: 8,
            profile_hysteresis_ticks: 4,
            resilient_region_spread_bonus_weight: 10,
        }
        .validate()
        .is_err()
    );
    assert!(
        MeshPeerTablePolicy {
            max_entries: 2,
            max_entries_per_region: 1,
            stale_after_ticks: 8,
            target_distinct_regions: 3,
            replacement_min_score_delta: 1,
            degraded_replacement_min_score_delta: 1,
            max_replacements_per_window: 8,
            stability_window_ticks: 8,
            profile_hysteresis_ticks: 4,
            resilient_region_spread_bonus_weight: 10,
        }
        .validate()
        .is_err()
    );
    assert!(
        MeshPeerTablePolicy {
            max_entries: 2,
            max_entries_per_region: 1,
            stale_after_ticks: 8,
            target_distinct_regions: 1,
            replacement_min_score_delta: 0,
            degraded_replacement_min_score_delta: 1,
            max_replacements_per_window: 8,
            stability_window_ticks: 8,
            profile_hysteresis_ticks: 4,
            resilient_region_spread_bonus_weight: 10,
        }
        .validate()
        .is_err()
    );
    assert!(
        MeshPeerTablePolicy {
            max_entries: 2,
            max_entries_per_region: 1,
            stale_after_ticks: 8,
            target_distinct_regions: 1,
            replacement_min_score_delta: 1,
            degraded_replacement_min_score_delta: 2,
            max_replacements_per_window: 8,
            stability_window_ticks: 8,
            profile_hysteresis_ticks: 4,
            resilient_region_spread_bonus_weight: 10,
        }
        .validate()
        .is_err()
    );
    assert!(
        MeshPeerTablePolicy {
            max_entries: 2,
            max_entries_per_region: 1,
            stale_after_ticks: 8,
            target_distinct_regions: 1,
            replacement_min_score_delta: 1,
            degraded_replacement_min_score_delta: 1,
            max_replacements_per_window: 0,
            stability_window_ticks: 8,
            profile_hysteresis_ticks: 4,
            resilient_region_spread_bonus_weight: 10,
        }
        .validate()
        .is_err()
    );
    assert!(
        MeshPeerTablePolicy {
            max_entries: 2,
            max_entries_per_region: 1,
            stale_after_ticks: 8,
            target_distinct_regions: 1,
            replacement_min_score_delta: 1,
            degraded_replacement_min_score_delta: 1,
            max_replacements_per_window: 8,
            stability_window_ticks: 0,
            profile_hysteresis_ticks: 4,
            resilient_region_spread_bonus_weight: 10,
        }
        .validate()
        .is_err()
    );
    assert!(
        MeshPeerTablePolicy {
            max_entries: 2,
            max_entries_per_region: 1,
            stale_after_ticks: 8,
            target_distinct_regions: 1,
            replacement_min_score_delta: 1,
            degraded_replacement_min_score_delta: 1,
            max_replacements_per_window: 8,
            stability_window_ticks: 8,
            profile_hysteresis_ticks: 4,
            resilient_region_spread_bonus_weight: 0,
        }
        .validate()
        .is_err()
    );
}
