use super::*;
#[test]
fn connect_and_launch_error_contracts_match_on_stage_matrix() {
    let cases: [(&str, &str, Vec<String>); 6] = [
        (
            "options_parse_duplicate_namespace",
            "options_parse",
            vec![
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
            ],
        ),
        (
            "simulation_input_duplicate_node",
            "simulation_input",
            vec![
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
            ],
        ),
        (
            "policy_parse_invalid_payload",
            "policy_parse",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "mesh_max_peers=0".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
            ],
        ),
        (
            "peer_spec_invalid",
            "peer_spec",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "bad-peer-spec".to_string(),
            ],
        ),
        (
            "peer_table_policy_invalid",
            "peer_table_policy",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--table-max-entries".to_string(),
                "0".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
            ],
        ),
        (
            "plan_path_unreachable",
            "plan_path",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=99".to_string(),
                "--peer".to_string(),
                "n1@198.51.100.1:443@eu@20@90".to_string(),
            ],
        ),
    ];

    for (case_id, expected_stage, args) in cases {
        run_error_parity_case_with_stage(case_id, expected_stage, args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_peer_spec_stage_matrix() {
    let cases: [(&str, Vec<String>); 10] = [
        ("bad-peer-spec", vec!["bad-peer-spec".to_string()]),
        ("missing-port", vec!["n1@127.0.0.1@eu@20@90".to_string()]),
        (
            "invalid-load",
            vec!["n1@127.0.0.1:443@eu@bad@90".to_string()],
        ),
        (
            "invalid-reliability",
            vec!["n1@127.0.0.1:443@eu@20@bad".to_string()],
        ),
        ("empty-region", vec!["n1@127.0.0.1:443@@20@90".to_string()]),
        ("empty-endpoint", vec!["n1@@eu@20@90".to_string()]),
        (
            "non_numeric_port",
            vec!["n1@127.0.0.1:abc@eu@20@90".to_string()],
        ),
        (
            "empty-reliability",
            vec!["n1@127.0.0.1:443@eu@20@".to_string()],
        ),
        ("empty-host", vec!["n1@:443@eu@20@90".to_string()]),
        ("empty-port", vec!["n1@127.0.0.1:@eu@20@90".to_string()]),
    ];

    for (case_id, peers) in cases {
        let mut args = base_core_args();
        args.push("--policy-payload".to_string());
        args.push("allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string());
        for peer in peers {
            args.push("--peer".to_string());
            args.push(peer);
        }
        run_error_parity_case_with_stage(&format!("peer_spec_{case_id}"), "peer_spec", args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_simulation_input_duplicate_peer_matrix() {
    let cases: [(&str, Vec<String>); 3] = [
        (
            "dup_pair_basic",
            vec![
                "n1@127.0.0.1:4443@eu@20@90".to_string(),
                "n1@127.0.0.1:5443@eu@25@88".to_string(),
            ],
        ),
        (
            "dup_pair_with_third_unique",
            vec![
                "n1@127.0.0.1:4443@eu@20@90".to_string(),
                "n2@127.0.0.1:6443@eu@20@90".to_string(),
                "n1@127.0.0.1:5443@eu@25@88".to_string(),
            ],
        ),
        (
            "dup_pair_reordered",
            vec![
                "n2@127.0.0.1:6443@eu@20@90".to_string(),
                "n1@127.0.0.1:5443@eu@25@88".to_string(),
                "n1@127.0.0.1:4443@eu@20@90".to_string(),
            ],
        ),
    ];

    for (case_id, peers) in cases {
        let mut args = base_core_args();
        args.push("--policy-payload".to_string());
        args.push("allow=mesh;mesh_max_peers=2;mesh_min_reliability=80".to_string());
        for peer in peers {
            args.push("--peer".to_string());
            args.push(peer);
        }
        run_error_parity_case_with_stage(
            &format!("simulation_input_{case_id}"),
            "simulation_input",
            args,
        );
    }
}
