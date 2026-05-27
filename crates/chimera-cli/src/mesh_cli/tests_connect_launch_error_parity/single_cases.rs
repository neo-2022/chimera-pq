use super::*;
#[test]
fn connect_and_launch_error_contracts_match_on_options_parse_error() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "--namespace".to_string(),
        "cef-alt".to_string(),
    ];

    let connect =
        run_mesh_subcommand_json("connect-probe", args.clone(), 2, "parity_options_connect");
    let launch = run_mesh_subcommand_json("launch-preflight", args, 2, "parity_options_launch");

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_peer_error() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:4443@eu@20@90".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:5443@eu@25@88".to_string(),
        "--timeout-ms".to_string(),
        "25".to_string(),
    ];

    let connect = run_mesh_subcommand_json("connect-probe", args.clone(), 2, "parity_dupe_connect");
    let launch = run_mesh_subcommand_json("launch-preflight", args, 2, "parity_dupe_launch");

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_policy_parse_error() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "mesh_max_peers=0".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
    ];

    let connect =
        run_mesh_subcommand_json("connect-probe", args.clone(), 2, "parity_policy_connect");
    let launch = run_mesh_subcommand_json("launch-preflight", args, 2, "parity_policy_launch");

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_peer_spec_error() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "bad-peer-spec".to_string(),
    ];

    let connect =
        run_mesh_subcommand_json("connect-probe", args.clone(), 2, "parity_peer_spec_connect");
    let launch = run_mesh_subcommand_json("launch-preflight", args, 2, "parity_peer_spec_launch");

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_peer_spec_missing_port_error() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_peer_spec_missing_port_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_peer_spec_missing_port_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_peer_spec_invalid_load_error() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:443@eu@bad@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_peer_spec_invalid_load_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_peer_spec_invalid_load_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_peer_spec_invalid_reliability_error() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:443@eu@20@bad".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_peer_spec_invalid_reliability_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_peer_spec_invalid_reliability_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_peer_spec_empty_region_error() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:443@@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_peer_spec_empty_region_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_peer_spec_empty_region_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_peer_spec_empty_node_id_error() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "@127.0.0.1:443@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_peer_spec_empty_node_id_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_peer_spec_empty_node_id_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_peer_table_policy_error() {
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
        "0".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_peer_table_policy_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_peer_table_policy_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_plan_path_error() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=99".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
    ];

    let connect =
        run_mesh_subcommand_json("connect-probe", args.clone(), 2, "parity_plan_path_connect");
    let launch = run_mesh_subcommand_json("launch-preflight", args, 2, "parity_plan_path_launch");

    assert_error_parity(&connect, &launch);
}
