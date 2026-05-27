use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshRuntime};

#[test]
fn planning_prefers_diverse_regions_before_filling_duplicates() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-eu-1".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 5,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-eu-2".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-us-1".to_string(),
            endpoint: "198.51.100.12:443".to_string(),
            region: "us".to_string(),
            load_score: 12,
            reliability_score: 89,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
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
        max_peers: 2,
        prefer_region_diversity: true,
        max_selected_per_region: 1,
        min_distinct_regions: 1,
        path_profile_override: None,
        connect_fallback_ports: vec![443, 8443],
    };
    let plan = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 2);
    assert_eq!(plan.selected_peers[0].node_id, "node-eu-1");
    assert_eq!(plan.selected_peers[1].node_id, "node-us-1");
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selection_strategy=region_diversity"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selection_region_cap=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_regions=2"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("distinct_region_ratio_pct=100"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("candidate_distinct_regions=2"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("min_distinct_regions_feasible=true"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("min_distinct_regions_feasibility_gap=0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("distinct_region_deficit=0"))
    );
}

#[test]
fn planning_respects_max_selected_per_region_cap() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-eu-1".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 5,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-eu-2".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-eu-3".to_string(),
            endpoint: "198.51.100.12:443".to_string(),
            region: "eu".to_string(),
            load_score: 12,
            reliability_score: 89,
        },
        MeshDiscoveryRecord {
            node_id: "node-us-1".to_string(),
            endpoint: "198.51.100.13:443".to_string(),
            region: "us".to_string(),
            load_score: 15,
            reliability_score: 88,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
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
        max_peers: 3,
        prefer_region_diversity: true,
        max_selected_per_region: 2,
        min_distinct_regions: 1,
        path_profile_override: None,
        connect_fallback_ports: vec![443, 8443],
    };
    let plan = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 3);
    let eu_count = plan
        .selected_peers
        .iter()
        .filter(|peer| peer.region == "eu")
        .count();
    assert_eq!(eu_count, 2);
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_region_counts=eu:2,us:1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("distinct_region_ratio_pct=66"))
    );
}
