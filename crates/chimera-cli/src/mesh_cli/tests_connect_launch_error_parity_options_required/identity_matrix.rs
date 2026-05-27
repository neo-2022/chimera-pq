use super::*;
#[test]
fn connect_and_launch_error_contracts_match_on_error_flag_position_stage_matrix() {
    let cases: [(&str, &str, Vec<String>); 4] = [
        (
            "mx_pos_unknown_flag_front",
            "options_parse",
            vec![
                "--unknown-flag".to_string(),
                "1".to_string(),
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_pos_unknown_flag_tail",
            "options_parse",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
                "--unknown-flag".to_string(),
                "1".to_string(),
            ],
        ),
        (
            "mx_pos_blank_peer_front",
            "options_parse",
            vec![
                "--peer".to_string(),
                String::new(),
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
            ],
        ),
        (
            "mx_pos_blank_peer_tail",
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
fn connect_and_launch_error_contracts_match_on_whitespace_identity_position_matrix() {
    let cases: [(&str, Vec<String>); 4] = [
        (
            "mx_pos_ws_namespace_front",
            vec![
                "--namespace".to_string(),
                "   ".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_pos_ws_namespace_tail",
            vec![
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
                "--namespace".to_string(),
                "   ".to_string(),
            ],
        ),
        (
            "mx_pos_ws_node_front",
            vec![
                "--node".to_string(),
                "   ".to_string(),
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_pos_ws_node_tail",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
                "--node".to_string(),
                "   ".to_string(),
            ],
        ),
    ];

    for (case_id, args) in cases {
        assert_required_case(case_id, "options_parse", args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_dangling_unknown_flag_matrix() {
    let cases: [(&str, Vec<String>); 4] = [
        (
            "mx_dangling_unknown_long_front",
            vec![
                "--unknown-flag".to_string(),
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_dangling_unknown_long_tail",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
                "--unknown-flag".to_string(),
            ],
        ),
        (
            "mx_dangling_unknown_short_front",
            vec![
                "-z".to_string(),
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
        (
            "mx_dangling_unknown_short_tail",
            vec![
                "--namespace".to_string(),
                "cef-public".to_string(),
                "--node".to_string(),
                "node-client".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
                "-z".to_string(),
            ],
        ),
    ];

    for (case_id, args) in cases {
        assert_required_case(case_id, "options_parse", args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_malformed_flag_token_matrix() {
    let cases: [(&str, Vec<String>); 3] = [
        ("mx_malformed_dash_only", vec!["-".to_string()]),
        ("mx_malformed_double_dash_only", vec!["--".to_string()]),
        ("mx_malformed_triple_dash", vec!["---bad".to_string()]),
    ];

    for (case_id, extra_flags) in cases {
        let args = required_args_with(extra_flags);
        assert_required_case(case_id, "options_parse", args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_error_identity_fallback_matrix() {
    let cases: [(&str, &str, &str, &str, Vec<String>); 3] = [
        (
            "identity_missing_namespace",
            "options_parse",
            "unknown",
            "node-client",
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
            "identity_missing_node",
            "options_parse",
            "cef-public",
            "unknown",
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
            "identity_missing_both",
            "options_parse",
            "unknown",
            "unknown",
            vec![
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
            ],
        ),
    ];

    for (case_id, expected_stage, expected_namespace, expected_node, args) in cases {
        assert_required_case_with_identity(
            case_id,
            expected_stage,
            expected_namespace,
            expected_node,
            args,
        );
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_error_identity_preserve_matrix() {
    let cases: [(&str, Vec<String>, &str, &str); 2] = [
        (
            "identity_preserve_known_values",
            vec![
                "--namespace".to_string(),
                "cef-stage".to_string(),
                "--node".to_string(),
                "edge-node-7".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
                "--unknown-flag".to_string(),
                "1".to_string(),
            ],
            "cef-stage",
            "edge-node-7",
        ),
        (
            "identity_preserve_with_extra_flags",
            vec![
                "--namespace".to_string(),
                "cef-prod".to_string(),
                "--node".to_string(),
                "node-a".to_string(),
                "--timeout-ms".to_string(),
                "25".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--peer".to_string(),
                "n1@127.0.0.1:1@eu@20@90".to_string(),
                "-z".to_string(),
            ],
            "cef-prod",
            "node-a",
        ),
    ];

    for (case_id, args, expected_namespace, expected_node) in cases {
        assert_required_case_with_identity(
            case_id,
            "options_parse",
            expected_namespace,
            expected_node,
            args,
        );
    }
}
