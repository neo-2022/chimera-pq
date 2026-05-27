use super::*;
#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_table_max_per_region_options_parse_error()
{
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
        "64".to_string(),
        "--table-max-per-region".to_string(),
        "32".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_duplicate_table_max_per_region_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_duplicate_table_max_per_region_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_policy_payload_and_profile_conflict() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--traffic-profile".to_string(),
        "high_speed_anonymous".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_policy_profile_conflict_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_policy_profile_conflict_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_invalid_traffic_profile_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--traffic-profile".to_string(),
        "ultra_mode".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_invalid_profile_connect",
    );
    let launch =
        run_mesh_subcommand_json("launch-preflight", args, 2, "parity_invalid_profile_launch");

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_traffic_profile_flag() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--traffic-profile".to_string(),
        "high_speed_anonymous".to_string(),
        "--traffic-profile".to_string(),
        "privacy_first".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_duplicate_traffic_profile_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_duplicate_traffic_profile_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_table_stale_after_flag() {
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
        "10".to_string(),
        "--table-stale-after".to_string(),
        "20".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_duplicate_table_stale_after_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_duplicate_table_stale_after_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_node_flag() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--node".to_string(),
        "node-client-2".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_duplicate_node_connect",
    );
    let launch =
        run_mesh_subcommand_json("launch-preflight", args, 2, "parity_duplicate_node_launch");

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_invite_token_flag() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--invite-token".to_string(),
        "inv-1".to_string(),
        "--invite-token".to_string(),
        "inv-2".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_duplicate_invite_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_duplicate_invite_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_failed_node_flag() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--failed-node".to_string(),
        "n1".to_string(),
        "--failed-node".to_string(),
        "n2".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_duplicate_failed_node_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_duplicate_failed_node_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_cooldown_node_flag() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--cooldown-node".to_string(),
        "n1".to_string(),
        "--cooldown-node".to_string(),
        "n2".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_duplicate_cooldown_node_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_duplicate_cooldown_node_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_policy_payload_flag() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=2;mesh_min_reliability=70".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_duplicate_policy_payload_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_duplicate_policy_payload_launch",
    );

    assert_error_parity(&connect, &launch);
}
