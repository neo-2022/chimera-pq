use crate::MeshPathPolicy;

#[test]
fn path_policy_default_auto_matches_auto_mode_contract() {
    let policy = MeshPathPolicy::default_auto();
    assert!(policy.manual_override_fields().is_empty());
    assert_eq!(policy.max_load_score, 100);
    assert_eq!(policy.max_peers, 1);
    assert!(policy.prefer_region_diversity);
}

#[test]
fn path_policy_manual_override_fields_detect_non_default_values() {
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: vec!["node-x".to_string()],
        require_min_reliability: 70,
        max_load_score: 60,
        max_peers: 2,
        prefer_region_diversity: false,
        max_selected_per_region: 2,
        min_distinct_regions: 2,
        path_profile_override: None,
        connect_fallback_ports: vec![443, 8443],
    };
    let fields = policy.manual_override_fields();
    assert!(fields.contains(&"allowed_regions"));
    assert!(fields.contains(&"blocked_node_ids"));
    assert!(fields.contains(&"require_min_reliability"));
    assert!(fields.contains(&"max_load_score"));
    assert!(fields.contains(&"max_peers"));
    assert!(fields.contains(&"prefer_region_diversity"));
    assert!(fields.contains(&"max_selected_per_region"));
    assert!(fields.contains(&"min_distinct_regions"));
}

#[test]
fn policy_validate_rejects_region_cap_above_max_peers() {
    let policy = MeshPathPolicy {
        allowed_regions: Vec::new(),
        blocked_node_ids: Vec::new(),
        require_min_reliability: 0,
        max_load_score: 100,
        max_peers: 1,
        prefer_region_diversity: true,
        max_selected_per_region: 2,
        min_distinct_regions: 1,
        path_profile_override: None,
        connect_fallback_ports: vec![443, 8443],
    };
    assert!(policy.validate().is_err());
}
