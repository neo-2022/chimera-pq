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
fn parse_mesh_route_explain_options_rejects_short_flag_shape_as_positional() {
    let mut args = base_args();
    args.extend(["-x".to_string(), "1".to_string()]);
    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("unexpected positional argument '-x'".to_string())
    );
}

#[test]
fn parse_mesh_route_explain_options_rejects_double_dash_as_unknown_flag() {
    let mut args = base_args();
    args.extend(["--".to_string(), "1".to_string()]);
    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("unknown flag '--'".to_string())
    );
}
