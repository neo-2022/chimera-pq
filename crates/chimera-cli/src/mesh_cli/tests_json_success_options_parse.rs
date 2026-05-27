use super::tests_json_runner_utils::run_route_explain_json;
use super::tests_json_utils::{
    assert_route_explain_contract_blocks_presence, assert_route_explain_envelope, expected_kind_ok,
    expected_status_ok,
};

#[test]
fn mesh_route_explain_json_accepts_trimmed_table_numeric_flags() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "--table-max-entries".to_string(),
        " 42 ".to_string(),
        "--table-max-per-region".to_string(),
        " 7 ".to_string(),
        "--table-stale-after".to_string(),
        " 3 ".to_string(),
    ];

    let parsed = run_route_explain_json(args, 0, "json_success_trimmed_table_numeric_flags");
    assert_route_explain_envelope(&parsed, expected_status_ok(), expected_kind_ok());
    assert_route_explain_contract_blocks_presence(&parsed);
}

#[test]
fn mesh_route_explain_json_accepts_repeated_json_flag() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "--json".to_string(),
        "--json".to_string(),
    ];

    let parsed = run_route_explain_json(args, 0, "json_success_repeated_json_flag");
    assert_route_explain_envelope(&parsed, expected_status_ok(), expected_kind_ok());
    assert_route_explain_contract_blocks_presence(&parsed);
}
