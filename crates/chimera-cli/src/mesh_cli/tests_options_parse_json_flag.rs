use crate::mesh_cli::options::parse_mesh_route_explain_options;

#[test]
fn parse_mesh_route_explain_options_accepts_repeated_json_flag() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=2;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "--json".to_string(),
        "--json".to_string(),
    ];

    let parsed = parse_mesh_route_explain_options(&args);
    assert!(parsed.is_ok());
    assert_eq!(parsed.ok().map(|value| value.json_output), Some(true));
}

#[test]
fn parse_mesh_route_explain_options_accepts_json_flag_at_end_without_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=2;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "--json".to_string(),
    ];

    let parsed = parse_mesh_route_explain_options(&args);
    assert!(parsed.is_ok());
    assert_eq!(parsed.ok().map(|value| value.json_output), Some(true));
}

#[test]
fn parse_mesh_route_explain_options_accepts_json_flag_between_value_flags() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--json".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=2;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
    ];

    let parsed = parse_mesh_route_explain_options(&args);
    assert!(parsed.is_ok());
    assert_eq!(parsed.ok().map(|value| value.json_output), Some(true));
}

#[test]
fn parse_mesh_route_explain_options_rejects_positional_after_json_flag() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=2;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "--json".to_string(),
        "extra".to_string(),
    ];

    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("unexpected positional argument 'extra'".to_string())
    );
}
