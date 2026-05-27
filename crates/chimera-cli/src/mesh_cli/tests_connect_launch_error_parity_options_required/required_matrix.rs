use super::*;
#[test]
fn connect_and_launch_error_contracts_match_on_required_stage_matrix() {
    let cases: [(&str, &str, Vec<String>); 6] = [
        (
            "mx_namespace_required_missing",
            "options_parse",
            vec![
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_node_required_missing",
            "options_parse",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_policy_required_missing",
            "options_parse",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_peer_required_missing",
            "options_parse",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
            ],
        ),
        (
            "mx_blank_policy",
            "options_parse",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                String::new(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_blank_peer",
            "options_parse",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                String::new(),
            ],
        ),
    ];

    for (case_id, expected_stage, args) in cases {
        assert_required_case(case_id, expected_stage, args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_missing_identity_value_stage_matrix() {
    let cases: [(&str, &str, Vec<String>); 6] = [
        (
            "mx_missing_invite_token_value",
            "options_parse",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--invite-token".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_missing_failed_node_value",
            "options_parse",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--failed-node".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_missing_cooldown_node_value",
            "options_parse",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--cooldown-node".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_missing_namespace_value",
            "options_parse",
            vec![
                "--namespace".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_missing_node_value",
            "options_parse",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_missing_peer_value",
            "peer_spec",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
            ],
        ),
    ];

    for (case_id, expected_stage, args) in cases {
        assert_required_case(case_id, expected_stage, args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_unknown_flag_stage_matrix() {
    let cases: [(&str, Vec<String>); 3] = [
        (
            "mx_unknown_long_flag",
            vec!["--unknown-flag".to_string(), "1".to_string()],
        ),
        (
            "mx_unknown_short_flag",
            vec!["-z".to_string(), "1".to_string()],
        ),
        (
            "mx_typo_flag",
            vec!["--table-max-entry".to_string(), "64".to_string()],
        ),
    ];

    for (case_id, mut extra_flags) in cases {
        let args = required_args_with(std::mem::take(&mut extra_flags));
        assert_required_case(case_id, "options_parse", args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_blank_identity_value_stage_matrix() {
    let cases: [(&str, Vec<String>); 6] = [
        (
            "mx_blank_invite_token",
            vec!["--invite-token".to_string(), String::new()],
        ),
        (
            "mx_blank_failed_node",
            vec!["--failed-node".to_string(), String::new()],
        ),
        (
            "mx_blank_cooldown_node",
            vec!["--cooldown-node".to_string(), String::new()],
        ),
        (
            "mx_whitespace_invite_token",
            vec!["--invite-token".to_string(), "   ".to_string()],
        ),
        (
            "mx_whitespace_failed_node",
            vec!["--failed-node".to_string(), "   ".to_string()],
        ),
        (
            "mx_whitespace_cooldown_node",
            vec!["--cooldown-node".to_string(), "   ".to_string()],
        ),
    ];

    for (case_id, mut extra_flags) in cases {
        let args = required_args_with(std::mem::take(&mut extra_flags));
        assert_required_case(case_id, "options_parse", args);
    }
}
