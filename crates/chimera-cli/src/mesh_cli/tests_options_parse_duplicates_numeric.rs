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
fn parse_mesh_route_explain_options_rejects_duplicate_table_max_per_region() {
    let mut args = base_args();
    args.extend([
        "--table-max-per-region".to_string(),
        "7".to_string(),
        "--table-max-per-region".to_string(),
        "8".to_string(),
    ]);
    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("duplicate singleton flag '--table-max-per-region'".to_string())
    );
}

#[test]
fn parse_mesh_route_explain_options_rejects_duplicate_table_max_entries() {
    let mut args = base_args();
    args.extend([
        "--table-max-entries".to_string(),
        "10".to_string(),
        "--table-max-entries".to_string(),
        "11".to_string(),
    ]);
    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("duplicate singleton flag '--table-max-entries'".to_string())
    );
}

#[test]
fn parse_mesh_route_explain_options_rejects_duplicate_table_stale_after() {
    let mut args = base_args();
    args.extend([
        "--table-stale-after".to_string(),
        "3".to_string(),
        "--table-stale-after".to_string(),
        "4".to_string(),
    ]);
    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("duplicate singleton flag '--table-stale-after'".to_string())
    );
}
