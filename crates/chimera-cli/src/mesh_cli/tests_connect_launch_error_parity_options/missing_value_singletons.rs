use super::*;
#[test]
fn connect_and_launch_error_contracts_match_on_hyphenated_traffic_profile_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--traffic-profile".to_string(),
        "high-speed-anonymous".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_hyphenated_profile_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_hyphenated_profile_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_timeout_missing_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--timeout-ms".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_timeout_missing_value_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_timeout_missing_value_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_table_max_entries_missing_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--table-max-entries".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_table_max_entries_missing_value_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_table_max_entries_missing_value_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_table_max_per_region_missing_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--table-max-per-region".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_table_max_per_region_missing_value_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_table_max_per_region_missing_value_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_table_stale_after_missing_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--table-stale-after".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_table_stale_after_missing_value_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_table_stale_after_missing_value_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_traffic_profile_missing_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--traffic-profile".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_traffic_profile_missing_value_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_traffic_profile_missing_value_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_invite_token_missing_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--invite-token".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_invite_token_missing_value_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_invite_token_missing_value_launch",
    );

    assert_error_parity(&connect, &launch);
}
