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
fn parse_mesh_route_explain_options_reports_missing_value_for_required_value_flags() {
    for (flag, expected) in [
        ("--namespace", "missing value for flag '--namespace'"),
        ("--node", "missing value for flag '--node'"),
        (
            "--policy-payload",
            "missing value for flag '--policy-payload'",
        ),
        ("--peer", "missing value for flag '--peer'"),
    ] {
        let mut args = base_args();
        args.push(flag.to_string());
        assert_eq!(
            parse_mesh_route_explain_options(&args).err(),
            Some(expected.to_string())
        );
    }
}
