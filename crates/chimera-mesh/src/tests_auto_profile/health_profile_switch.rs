use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPeerHealth, MeshRuntime};

#[test]
fn plan_auto_profile_switches_to_resilient_on_degraded_health() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
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
    let plan = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("path_profile=resilient"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("path_profile_reason=auto:degraded_active"))
    );
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
            .any(|line| line.contains("effective_prefer_region_diversity=true"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_selected_per_region=2"))
    );
}
