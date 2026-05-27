use super::*;
#[test]
fn connect_and_launch_error_contracts_match_on_profile_conflict_position_matrix() {
    let cases: [(&str, Vec<String>); 3] = [
        (
            "mx_profile_conflict_front",
            vec![
                "--traffic-profile".to_string(),
                "privacy_first".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_profile_conflict_tail",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--traffic-profile".to_string(),
                "privacy_first".to_string(),
            ],
        ),
        (
            "mx_profile_conflict_middle",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--traffic-profile".to_string(),
                "privacy_first".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
    ];

    for (case_id, args) in cases {
        assert_options_stage_case(case_id, args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_table_per_region_alias_conflict_matrix() {
    let cases: [(&str, Vec<String>); 3] = [
        (
            "mx_alias_conflict_legacy_first",
            vec![
                "--table-max-per-region".to_string(),
                "64".to_string(),
                "--table-max-entries-per-region".to_string(),
                "32".to_string(),
            ],
        ),
        (
            "mx_alias_conflict_new_first",
            vec![
                "--table-max-entries-per-region".to_string(),
                "32".to_string(),
                "--table-max-per-region".to_string(),
                "64".to_string(),
            ],
        ),
        (
            "mx_alias_conflict_with_extra_flag",
            vec![
                "--timeout-ms".to_string(),
                "25".to_string(),
                "--table-max-per-region".to_string(),
                "64".to_string(),
                "--table-max-entries-per-region".to_string(),
                "32".to_string(),
            ],
        ),
    ];

    for (case_id, mut extra_flags) in cases {
        let mut args = base_args();
        args.append(&mut extra_flags);
        assert_options_stage_case(case_id, args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_stale_alias_conflict_matrix() {
    let cases: [(&str, Vec<String>); 3] = [
        (
            "mx_stale_alias_conflict_legacy_first",
            vec![
                "--table-stale-after-ticks".to_string(),
                "10".to_string(),
                "--table-stale-after".to_string(),
                "20".to_string(),
            ],
        ),
        (
            "mx_stale_alias_conflict_new_first",
            vec![
                "--table-stale-after".to_string(),
                "20".to_string(),
                "--table-stale-after-ticks".to_string(),
                "10".to_string(),
            ],
        ),
        (
            "mx_stale_alias_conflict_with_extra_flag",
            vec![
                "--timeout-ms".to_string(),
                "25".to_string(),
                "--table-stale-after-ticks".to_string(),
                "10".to_string(),
                "--table-stale-after".to_string(),
                "20".to_string(),
            ],
        ),
    ];

    for (case_id, mut extra_flags) in cases {
        let mut args = base_args();
        args.append(&mut extra_flags);
        assert_options_stage_case(case_id, args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_blank_namespace_options_parse_error() {
    let args = vec![
        "--namespace".to_string(),
        String::new(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_blank_namespace_connect",
    );
    let launch =
        run_mesh_subcommand_json("launch-preflight", args, 2, "parity_blank_namespace_launch");

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_blank_node_options_parse_error() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        String::new(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_blank_node_connect",
    );
    let launch = run_mesh_subcommand_json("launch-preflight", args, 2, "parity_blank_node_launch");

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_timeout_flag_options_parse_error() {
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
        "25".to_string(),
        "--timeout-ms".to_string(),
        "50".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_duplicate_timeout_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_duplicate_timeout_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_table_max_entries_options_parse_error() {
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
        "256".to_string(),
        "--table-max-entries".to_string(),
        "128".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_duplicate_table_max_entries_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_duplicate_table_max_entries_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_table_per_region_options_parse_error() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--table-max-entries-per-region".to_string(),
        "64".to_string(),
        "--table-max-entries-per-region".to_string(),
        "32".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_duplicate_table_per_region_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_duplicate_table_per_region_launch",
    );

    assert_error_parity(&connect, &launch);
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_table_stale_ticks_options_parse_error() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
        "--table-stale-after-ticks".to_string(),
        "10".to_string(),
        "--table-stale-after-ticks".to_string(),
        "20".to_string(),
    ];

    let connect = run_mesh_subcommand_json(
        "connect-probe",
        args.clone(),
        2,
        "parity_duplicate_table_stale_ticks_connect",
    );
    let launch = run_mesh_subcommand_json(
        "launch-preflight",
        args,
        2,
        "parity_duplicate_table_stale_ticks_launch",
    );

    assert_error_parity(&connect, &launch);
}
