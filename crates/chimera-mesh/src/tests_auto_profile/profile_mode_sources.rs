use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPathProfile, MeshRuntime};

#[test]
fn plan_reports_auto_mode_when_policy_is_default_like() {
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
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let policy = MeshPathPolicy {
        allowed_regions: Vec::new(),
        blocked_node_ids: Vec::new(),
        require_min_reliability: 0,
        max_load_score: 100,
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
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("decision_control_mode=auto"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("manual_override_count=0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("manual_override_fields=none"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_filter_source=auto_profile"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_min_reliability=80"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_load=30"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("path_profile=balanced"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("path_profile_source=auto"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("path_profile_reason=auto:balanced_signals"))
    );
}

#[test]
fn plan_reports_manual_filter_source_when_policy_is_overridden() {
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
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 88,
        max_load_score: 25,
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
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_filter_source=manual_override"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_min_reliability=88"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_load=25"))
    );
}

#[test]
fn plan_reports_manual_path_profile_override() {
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
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let mut policy = MeshPathPolicy::default_auto();
    policy.path_profile_override = Some(MeshPathProfile::Fast);
    let plan = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("path_profile=fast"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("path_profile_source=manual_override"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("path_profile_reason=manual_override"))
    );
}
