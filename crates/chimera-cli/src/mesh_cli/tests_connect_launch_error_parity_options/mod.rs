#![forbid(unsafe_code)]

use super::tests_connect_launch_error_parity::assert_error_parity;
use super::tests_json_runner_utils::run_mesh_subcommand_json;

fn assert_options_stage_case(case_id: &str, args: Vec<String>) {
    let connect_id = format!("parity_{case_id}_connect");
    let launch_id = format!("parity_{case_id}_launch");
    let connect = run_mesh_subcommand_json("connect-probe", args.clone(), 2, &connect_id);
    let launch = run_mesh_subcommand_json("launch-preflight", args, 2, &launch_id);
    assert_error_parity(&connect, &launch);
    assert_eq!(
        connect["error_stage"], "options_parse",
        "connect wrong stage for {case_id}"
    );
    assert_eq!(
        launch["error_stage"], "options_parse",
        "launch wrong stage for {case_id}"
    );
}

fn base_args() -> Vec<String> {
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

mod matrix_core;

mod position_and_alias_matrix;

mod duplicate_singletons;

mod invalid_numeric_singletons;

mod missing_value_singletons;
