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
fn parse_mesh_route_explain_options_reports_missing_value_for_singleton_flags() {
    for (flag, expected) in [
        ("--invite-token", "missing value for flag '--invite-token'"),
        ("--failed-node", "missing value for flag '--failed-node'"),
        (
            "--cooldown-node",
            "missing value for flag '--cooldown-node'",
        ),
        (
            "--table-max-entries",
            "missing value for flag '--table-max-entries'",
        ),
        (
            "--table-max-per-region",
            "missing value for flag '--table-max-per-region'",
        ),
        (
            "--table-stale-after",
            "missing value for flag '--table-stale-after'",
        ),
        ("--out", "missing value for flag '--out'"),
    ] {
        let mut args = base_args();
        args.push(flag.to_string());
        assert_eq!(
            parse_mesh_route_explain_options(&args).err(),
            Some(expected.to_string())
        );
    }
}
