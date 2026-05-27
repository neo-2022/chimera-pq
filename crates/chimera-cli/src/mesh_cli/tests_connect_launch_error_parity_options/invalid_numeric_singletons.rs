use super::*;
#[test]
fn connect_and_launch_error_contracts_match_on_invalid_timeout_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--timeout-ms".to_string(),
        "abc".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_invalid_timeout_connect",
    );
    let launch =
        run_mesh_subcommand_json("launch-preflight", args, 2, "parity_invalid_timeout_launch");

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_invalid_table_max_entries_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--table-max-entries".to_string(),
        "abc".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_invalid_table_max_entries_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_invalid_table_max_entries_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_invalid_table_max_per_region_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--table-max-per-region".to_string(),
        "abc".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_invalid_table_max_per_region_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_invalid_table_max_per_region_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_invalid_table_stale_after_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--table-stale-after".to_string(),
        "abc".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_invalid_table_stale_after_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_invalid_table_stale_after_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_negative_timeout_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--timeout-ms".to_string(),
        "-1".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_negative_timeout_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_negative_timeout_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_negative_table_stale_after_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--table-stale-after".to_string(),
        "-1".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_negative_table_stale_after_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_negative_table_stale_after_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_negative_table_max_per_region_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--table-max-per-region".to_string(),
        "-1".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_negative_table_max_per_region_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_negative_table_max_per_region_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_negative_table_max_entries_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--table-max-entries".to_string(),
        "-1".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_negative_table_max_entries_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_negative_table_max_entries_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_decimal_table_stale_after_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--table-stale-after".to_string(),
        "1.5".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_decimal_table_stale_after_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_decimal_table_stale_after_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_decimal_table_max_per_region_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--table-max-per-region".to_string(),
        "1.5".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_decimal_table_max_per_region_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_decimal_table_max_per_region_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_decimal_table_max_entries_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--table-max-entries".to_string(),
        "1.5".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_decimal_table_max_entries_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_decimal_table_max_entries_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_decimal_timeout_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--timeout-ms".to_string(),
        "1.5".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_decimal_timeout_connect",
    );
    let launch =
        run_mesh_subcommand_json("launch-preflight", args, 2, "parity_decimal_timeout_launch");

    assert_error_parity(&connect, &launch);
}
