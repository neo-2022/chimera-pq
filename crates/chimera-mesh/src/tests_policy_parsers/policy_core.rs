use crate::{MeshPathPolicy, MeshPathProfile};

#[test]
fn policy_from_dps_payload_parses_and_validates() {
    let payload = "allow=mesh;mesh_allowed_regions=eu,ru;mesh_blocked_nodes=node-x,node-y;mesh_min_reliability=80;mesh_max_load=55;mesh_max_peers=2;mesh_prefer_region_diversity=false;mesh_max_selected_per_region=2;mesh_min_distinct_regions=1";
    let policy = MeshPathPolicy::from_dps_payload(payload).unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(
        policy.allowed_regions,
        vec!["eu".to_string(), "ru".to_string()]
    );
    assert_eq!(
        policy.blocked_node_ids,
        vec!["node-x".to_string(), "node-y".to_string()]
    );
    assert_eq!(policy.require_min_reliability, 80);
    assert_eq!(policy.max_load_score, 55);
    assert_eq!(policy.max_peers, 2);
    assert!(!policy.prefer_region_diversity);
    assert_eq!(policy.max_selected_per_region, 2);
    assert_eq!(policy.min_distinct_regions, 1);
    assert_eq!(policy.connect_fallback_ports, vec![443, 8443]);
}

#[test]
fn policy_from_dps_payload_normalizes_and_dedups_allowed_regions() {
    let payload = "allow=mesh;mesh_allowed_regions=EU, eu ,Ru,ru ;mesh_max_peers=2;mesh_max_selected_per_region=1";
    let policy = MeshPathPolicy::from_dps_payload(payload).unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(
        policy.allowed_regions,
        vec!["eu".to_string(), "ru".to_string()]
    );
}

#[test]
fn policy_from_dps_payload_parses_bool_case_insensitive() {
    let payload = "allow=mesh;mesh_max_peers=2;mesh_max_selected_per_region=1;mesh_prefer_region_diversity=TRUE";
    let policy = MeshPathPolicy::from_dps_payload(payload).unwrap_or_else(|e| unreachable!("{e}"));
    assert!(policy.prefer_region_diversity);
}

#[test]
fn policy_from_dps_payload_parses_case_insensitive_mesh_keys() {
    let payload = "allow=mesh;MESH_ALLOWED_REGIONS=EU,ru;Mesh_Max_Peers=2;mesh_max_selected_per_region=1;MESH_MIN_DISTINCT_REGIONS=1";
    let policy = MeshPathPolicy::from_dps_payload(payload).unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(
        policy.allowed_regions,
        vec!["eu".to_string(), "ru".to_string()]
    );
    assert_eq!(policy.max_peers, 2);
    assert_eq!(policy.min_distinct_regions, 1);
    assert_eq!(policy.path_profile_override, None);
}

#[test]
fn policy_from_dps_payload_parses_path_profile_override() {
    let payload =
        "allow=mesh;mesh_max_peers=1;mesh_max_selected_per_region=1;mesh_path_profile=resilient";
    let policy = MeshPathPolicy::from_dps_payload(payload).unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(
        policy.path_profile_override,
        Some(MeshPathProfile::Resilient)
    );
}

#[test]
fn policy_from_dps_payload_parses_connect_fallback_ports() {
    let payload = "allow=mesh;mesh_max_peers=1;mesh_max_selected_per_region=1;mesh_connect_fallback_ports=9443,443,8443,443";
    let policy = MeshPathPolicy::from_dps_payload(payload).unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(policy.connect_fallback_ports, vec![9443, 443, 8443]);
}

#[test]
fn policy_from_dps_payload_rejects_invalid_values() {
    assert!(MeshPathPolicy::from_dps_payload("mesh_max_peers=0").is_err());
    assert!(MeshPathPolicy::from_dps_payload("mesh_max_load=101").is_err());
    assert!(MeshPathPolicy::from_dps_payload("mesh_prefer_region_diversity=yes").is_err());
    assert!(MeshPathPolicy::from_dps_payload("mesh_max_selected_per_region=0").is_err());
    assert!(MeshPathPolicy::from_dps_payload("mesh_connect_fallback_ports=").is_err());
    assert!(MeshPathPolicy::from_dps_payload("mesh_connect_fallback_ports=0").is_err());
    assert!(MeshPathPolicy::from_dps_payload("mesh_connect_fallback_ports=abc").is_err());
    assert!(
        MeshPathPolicy::from_dps_payload("mesh_max_peers=1;mesh_max_selected_per_region=2")
            .is_err()
    );
    assert!(MeshPathPolicy::from_dps_payload("mesh_typo_key=1").is_err());
    assert!(MeshPathPolicy::from_dps_payload("mesh_max_peers=1;mesh_max_peers=2").is_err());
    assert!(MeshPathPolicy::from_dps_payload("mesh_max_peers=1;MESH_MAX_PEERS=2").is_err());
    assert!(MeshPathPolicy::from_dps_payload("mesh_min_distinct_regions=0").is_err());
    assert!(
        MeshPathPolicy::from_dps_payload("mesh_max_peers=1;mesh_min_distinct_regions=2").is_err()
    );
}
