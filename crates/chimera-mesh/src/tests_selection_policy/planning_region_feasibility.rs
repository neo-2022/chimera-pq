use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshRuntime};

#[test]
fn planning_marks_when_min_distinct_regions_not_met() {
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
        max_selected_per_region: 2,
        min_distinct_regions: 2,
        path_profile_override: None,
        connect_fallback_ports: vec![443, 8443],
    };
    let plan = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 2);
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("min_distinct_regions_target=2"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("candidate_distinct_regions=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("min_distinct_regions_feasible=false"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("min_distinct_regions_feasibility_gap=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("min_distinct_regions_met=false"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("distinct_region_deficit=1"))
    );
}
