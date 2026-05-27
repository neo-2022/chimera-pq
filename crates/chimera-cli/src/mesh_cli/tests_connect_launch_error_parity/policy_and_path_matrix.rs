use super::*;
#[test]
fn connect_and_launch_error_contracts_match_on_peer_table_policy_stage_matrix() {
    let cases: [(&str, Vec<String>); 4] = [
        (
            "table_max_entries_zero",
            vec!["--table-max-entries".to_string(), "0".to_string()],
        ),
        (
            "table_max_entries_zero_with_timeout",
            vec![
                "--timeout-ms".to_string(),
                "25".to_string(),
                "--table-max-entries".to_string(),
                "0".to_string(),
            ],
        ),
        (
            "table_max_per_region_zero",
            vec!["--table-max-per-region".to_string(), "0".to_string()],
        ),
        (
            "table_stale_after_zero",
            vec!["--table-stale-after".to_string(), "0".to_string()],
        ),
    ];

    for (case_id, mut extra_flags) in cases {
        let mut args = base_core_args();
        args.push("--policy-payload".to_string());
        args.push("allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string());
        args.push("--peer".to_string());
        args.push("n1@198.51.100.1:443@eu@20@90".to_string());
        args.append(&mut extra_flags);
        run_error_parity_case_with_stage(
            &format!("peer_table_policy_{case_id}"),
            "peer_table_policy",
            args,
        );
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_peer_table_policy_position_matrix() {
    let cases: [(&str, Vec<String>); 3] = [
        (
            "peer_table_policy_front",
            vec![
                "--table-max-entries".to_string(),
                "0".to_string(),
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
            ],
        ),
        (
            "peer_table_policy_middle",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--table-max-entries".to_string(),
                "0".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
            ],
        ),
        (
            "peer_table_policy_tail",
            vec![
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
            ],
        ),
    ];

    for (case_id, args) in cases {
        run_error_parity_case_with_stage(
            &format!("peer_table_policy_pos_{case_id}"),
            "peer_table_policy",
            args,
        );
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_plan_path_stage_matrix() {
    let cases: [(&str, Vec<String>); 3] = [
        (
            "high_reliability_single_peer",
            vec![
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=99".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
            ],
        ),
        (
            "high_reliability_multiple_peers",
            vec![
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=2;mesh_min_reliability=98".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
                "--peer".to_string(),
                "n2@198.51.100.2:443@eu@25@92".to_string(),
            ],
        ),
        (
            "strict_region_and_reliability",
            vec![
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=97".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
                "--peer".to_string(),
                "n2@198.51.100.2:443@eu@25@92".to_string(),
            ],
        ),
    ];

    for (case_id, mut scenario_args) in cases {
        let mut args = base_core_args();
        args.append(&mut scenario_args);
        run_error_parity_case_with_stage(&format!("plan_path_{case_id}"), "plan_path", args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_plan_path_position_matrix() {
    let cases: [(&str, Vec<String>); 3] = [
        (
            "plan_path_front",
            vec![
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=99".to_string(),
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
            ],
        ),
        (
            "plan_path_middle",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=99".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
            ],
        ),
        (
            "plan_path_tail",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=99".to_string(),
            ],
        ),
    ];

    for (case_id, args) in cases {
        run_error_parity_case_with_stage(&format!("plan_path_pos_{case_id}"), "plan_path", args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_policy_parse_stage_matrix() {
    let cases: [(&str, String); 4] = [
        ("mesh_max_peers_zero", "mesh_max_peers=0".to_string()),
        (
            "unknown_field",
            "allow=mesh;mesh_unknown_field=1;mesh_max_peers=1".to_string(),
        ),
        (
            "invalid_token_format",
            "allow=mesh;mesh_max_peers;mesh_min_reliability=80".to_string(),
        ),
        (
            "invalid_reliability_range",
            "allow=mesh;mesh_max_peers=1;mesh_min_reliability=101".to_string(),
        ),
    ];

    for (case_id, policy_payload) in cases {
        let mut args = base_core_args();
        args.push("--policy-payload".to_string());
        args.push(policy_payload);
        args.push("--peer".to_string());
        args.push("n1@198.51.100.1:443@eu@20@90".to_string());
        run_error_parity_case_with_stage(&format!("policy_parse_{case_id}"), "policy_parse", args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_policy_parse_position_matrix() {
    let cases: [(&str, Vec<String>); 3] = [
        (
            "policy_parse_front",
            vec![
                "--policy-payload".to_string(),
                "mesh_max_peers=0".to_string(),
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
            ],
        ),
        (
            "policy_parse_middle",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--policy-payload".to_string(),
                "mesh_max_peers=0".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
            ],
        ),
        (
            "policy_parse_tail",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
                "--policy-payload".to_string(),
                "mesh_max_peers=0".to_string(),
            ],
        ),
    ];

    for (case_id, args) in cases {
        run_error_parity_case_with_stage(
            &format!("policy_parse_pos_{case_id}"),
            "policy_parse",
            args,
        );
    }
}
