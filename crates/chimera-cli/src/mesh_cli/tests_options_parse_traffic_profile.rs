use crate::mesh_cli::options::parse_mesh_route_explain_options;

fn base_args() -> Vec<String> {
    vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=2;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
    ]
}

#[test]
fn parse_mesh_route_explain_options_accepts_traffic_profile_preset_without_policy_payload() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--traffic-profile".to_string(),
        "high_speed_anonymous".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
    ];
    let parsed = parse_mesh_route_explain_options(&args).unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(parsed.namespace, "cef-public");
    assert_eq!(parsed.node_name, "node-client");
    assert!(
        parsed
            .policy_payload
            .contains("mesh_traffic_class=bulk_transfer")
    );
    assert!(
        parsed
            .policy_payload
            .contains("mesh_multipath_mode=flow_shard")
    );
}

#[test]
fn parse_mesh_route_explain_options_rejects_policy_payload_and_traffic_profile_together() {
    let mut args = base_args();
    args.extend(["--traffic-profile".to_string(), "privacy_first".to_string()]);
    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("cannot use both --policy-payload and --traffic-profile".to_string())
    );
}

#[test]
fn parse_mesh_route_explain_options_rejects_unknown_traffic_profile() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--traffic-profile".to_string(),
        "ultra_mode".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
    ];
    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("invalid --traffic-profile value (expected one of: high_speed_anonymous, privacy_first, speed_first, low_latency_private)".to_string())
    );
}
