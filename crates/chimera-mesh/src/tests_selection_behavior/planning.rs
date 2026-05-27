use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshRuntime};

#[test]
fn plan_explain_contains_candidate_summary_counters() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-blocked".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-region".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "us".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-reliability".to_string(),
            endpoint: "198.51.100.12:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 40,
        },
        MeshDiscoveryRecord {
            node_id: "node-load".to_string(),
            endpoint: "198.51.100.13:443".to_string(),
            region: "eu".to_string(),
            load_score: 90,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-ok".to_string(),
            endpoint: "198.51.100.14:443".to_string(),
            region: "eu".to_string(),
            load_score: 15,
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
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: vec!["node-blocked".to_string()],
        require_min_reliability: 80,
        max_load_score: 60,
        max_peers: 1,
        prefer_region_diversity: true,
        max_selected_per_region: 1,
        min_distinct_regions: 1,
        path_profile_override: None,
        connect_fallback_ports: vec![443, 8443],
    };

    let plan = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers[0].node_id, "node-ok");
    assert!(plan.explain.iter().any(|line| {
        line.contains(
            "candidate_summary=accepted:1,rejected_blocked:1,rejected_health:0,rejected_region:1,rejected_reliability:1,rejected_load:1",
        )
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_peer_ids=node-ok"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_peer_regions=eu"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selection_headroom=0"))
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains(
            "selection_pressure_summary=considered:5;selected:1;rejected:4;limit_skipped:0;utilization_pct:100;headroom:0",
        )
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selection_pressure_level=saturated"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selection_pressure_score=100"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selection_pressure_dominant=capacity"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selection_pressure_action_hint=capacity_full"))
    );
    assert!(plan.explain.iter().any(|line| line.contains(
        "selection_pressure_compact=level:saturated;score:100;dominant:capacity;action:capacity_full"
    )));
    assert!(plan.explain.iter().any(|line| {
        line.contains("selection_pressure_reason=level=saturated;dominant=capacity")
            && line.contains(";region=1;reliability=1;load=1")
    }));
}

#[test]
fn planning_rejects_when_no_peer_matches_policy() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.10:443".to_string(),
        region: "eu".to_string(),
        load_score: 95,
        reliability_score: 40,
    }];
    assert!(runtime.merge_discovery("seed-a", &records).is_ok());

    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["us".to_string()],
        blocked_node_ids: vec!["node-a".to_string()],
        require_min_reliability: 80,
        max_load_score: 50,
        max_peers: 2,
        prefer_region_diversity: true,
        max_selected_per_region: 1,
        min_distinct_regions: 1,
        path_profile_override: None,
        connect_fallback_ports: vec![443, 8443],
    };
    assert!(runtime.plan_path(&req, &policy).is_err());
}

#[test]
fn score_only_strategy_still_respects_region_cap() {
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
            load_score: 20,
            reliability_score: 85,
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
        prefer_region_diversity: false,
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
            .any(|line| line.contains("selection_strategy=score_only"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selection_region_cap=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("region_cap_rejections=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selection_headroom=1"))
    );
}
