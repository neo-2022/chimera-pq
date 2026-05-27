use super::tests_json_utils::{temp_out_file, with_json_out_args};
use std::fs;

pub(crate) fn run_mesh_subcommand_json(
    subcommand: &str,
    args: Vec<String>,
    expected_rc: i32,
    label: &str,
) -> serde_json::Value {
    let out = temp_out_file(label);
    let full_args = with_json_out_args(args, &out);
    let rc = super::mesh_command("usage", Some(subcommand), &full_args);
    assert_eq!(rc, expected_rc);
    let json = fs::read_to_string(&out)
        .unwrap_or_else(|e| unreachable!("{subcommand} json should be written: {e}"));
    let _ = fs::remove_file(&out);
    serde_json::from_str(&json)
        .unwrap_or_else(|e| unreachable!("{subcommand} json should be valid: {e}"))
}

pub(crate) fn run_route_explain_json(
    args: Vec<String>,
    expected_rc: i32,
    label: &str,
) -> serde_json::Value {
    run_mesh_subcommand_json("route-explain", args, expected_rc, label)
}
