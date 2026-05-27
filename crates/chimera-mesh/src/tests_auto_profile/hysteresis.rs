use crate::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPeerHealth, MeshPeerTablePolicy,
    MeshRuntime,
};

#[test]
fn plan_auto_profile_resilient_expands_peer_capacity() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-eu".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-us".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "us".to_string(),
            load_score: 25,
            reliability_score: 88,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    assert!(
        runtime
            .update_health_state(&[MeshPeerHealth {
                node_id: "node-eu".to_string(),
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
    let policy = MeshPathPolicy::default_auto();
    let plan = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 2);
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_peers=2"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_min_distinct_regions=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selection_headroom=0"))
    );
}

#[test]
fn plan_auto_profile_honors_hysteresis_before_return_to_balanced() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy(MeshPeerTablePolicy {
                max_entries: 16,
                max_entries_per_region: 16,
                stale_after_ticks: 16,
                target_distinct_regions: 1,
                replacement_min_score_delta: 1,
                degraded_replacement_min_score_delta: 1,
                max_replacements_per_window: 8,
                stability_window_ticks: 8,
                profile_hysteresis_ticks: 3,
                resilient_region_spread_bonus_weight: 10,
            })
            .is_ok()
    );
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.10:443".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 90,
    }];
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
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let policy = MeshPathPolicy::default_auto();
    let resilient_plan = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        resilient_plan
            .explain
            .iter()
            .any(|line| line.contains("path_profile=resilient"))
    );

    assert!(
        runtime
            .update_health_state(&[MeshPeerHealth {
                node_id: "node-a".to_string(),
                healthy: true,
                cooldown_active: false,
            }])
            .is_ok()
    );
    let still_resilient = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        still_resilient
            .explain
            .iter()
            .any(|line| line.contains("path_profile=resilient"))
    );
    assert!(
        still_resilient
            .explain
            .iter()
            .any(|line| line.contains("path_profile_reason=auto:hysteresis_hold"))
    );

    assert!(runtime.merge_discovery("seed-c", &[]).is_ok());
    assert!(runtime.merge_discovery("seed-d", &[]).is_ok());
    assert!(runtime.merge_discovery("seed-e", &[]).is_ok());
    let balanced_plan = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        balanced_plan
            .explain
            .iter()
            .any(|line| line.contains("path_profile=balanced"))
    );
}
