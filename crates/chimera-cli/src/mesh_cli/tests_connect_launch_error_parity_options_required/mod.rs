#![forbid(unsafe_code)]

use super::tests_connect_launch_error_parity::assert_error_parity;
use super::tests_json_runner_utils::run_mesh_subcommand_json;

fn assert_required_case(case_id: &str, expected_stage: &str, args: Vec<String>) {
    let connect_id = format!("parity_{case_id}_connect");
    let launch_id = format!("parity_{case_id}_launch");
    let connect = run_mesh_subcommand_json("connect-probe", args.clone(), 2, &connect_id);
    let launch = run_mesh_subcommand_json("launch-preflight", args, 2, &launch_id);
    assert_error_parity(&connect, &launch);
    assert_eq!(
        connect["error_stage"], expected_stage,
        "connect wrong stage for {case_id}"
    );
    assert_eq!(
        launch["error_stage"], expected_stage,
        "launch wrong stage for {case_id}"
    );
}

fn assert_required_case_with_identity(
    case_id: &str,
    expected_stage: &str,
    expected_namespace: &str,
    expected_node: &str,
    args: Vec<String>,
) {
    let connect_id = format!("parity_{case_id}_connect");
    let launch_id = format!("parity_{case_id}_launch");
    let connect = run_mesh_subcommand_json("connect-probe", args.clone(), 2, &connect_id);
    let launch = run_mesh_subcommand_json("launch-preflight", args, 2, &launch_id);
    assert_error_parity(&connect, &launch);
    assert_eq!(
        connect["error_stage"], expected_stage,
        "connect wrong stage for {case_id}"
    );
    assert_eq!(
        launch["error_stage"], expected_stage,
        "launch wrong stage for {case_id}"
    );
    assert_eq!(
        connect["namespace"], expected_namespace,
        "connect wrong namespace for {case_id}"
    );
    assert_eq!(
        launch["namespace"], expected_namespace,
        "launch wrong namespace for {case_id}"
    );
    assert_eq!(
        connect["node"], expected_node,
        "connect wrong node for {case_id}"
    );
    assert_eq!(
        launch["node"], expected_node,
        "launch wrong node for {case_id}"
    );
}

fn base_required_args() -> Vec<String> {
    vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@127.0.0.1:1@eu@20@90".to_string(),
    ]
}

fn required_args_with(mut extra_flags: Vec<String>) -> Vec<String> {
    let mut args = base_required_args();
    args.append(&mut extra_flags);
    args
}

mod required_matrix;

mod identity_matrix;

mod singleton_cases;
