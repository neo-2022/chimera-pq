use super::*;
#[test]
fn connect_and_launch_error_contracts_match_on_failed_node_missing_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--failed-node".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    assert_required_case("failed_node_missing_value", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_cooldown_node_missing_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--cooldown-node".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    assert_required_case("cooldown_node_missing_value", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_policy_payload_missing_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    assert_required_case("policy_payload_missing_value", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_namespace_missing_value() {
    let args = vec![
        "--namespace".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    assert_required_case("namespace_missing_value", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_node_missing_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    assert_required_case("node_missing_value", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_peer_missing_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
    ];

    assert_required_case("peer_missing_value", "peer_spec", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_namespace_required_missing() {
    let args = vec![
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    assert_required_case("namespace_required_missing", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_node_required_missing() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    assert_required_case("node_required_missing", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_unknown_flag() {
    let args = vec![
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
    ];

    assert_required_case("unknown_flag", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_policy_payload_required_missing() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    assert_required_case("policy_payload_required_missing", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_peer_required_missing() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
    ];

    assert_required_case("peer_required_missing", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_blank_policy_payload_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        String::new(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    assert_required_case("blank_policy_payload_value", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_blank_peer_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        String::new(),
    ];

    assert_required_case("blank_peer_value", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_whitespace_namespace_value() {
    let args = vec![
        "--namespace".to_string(),
        "   ".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    assert_required_case("whitespace_namespace_value", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_whitespace_node_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "   ".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    assert_required_case("whitespace_node_value", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_whitespace_policy_payload_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "   ".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ];

    assert_required_case("whitespace_policy_payload_value", "options_parse", args);
}

#[test]
fn connect_and_launch_error_contracts_match_on_whitespace_peer_value() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "   ".to_string(),
    ];

    assert_required_case("whitespace_peer_value", "options_parse", args);
}
