use crate::mesh_cli::options::{extract_non_empty_flag_value, wants_json_output};

#[test]
fn wants_json_output_detects_json_flag_presence() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--json".to_string(),
    ];
    assert!(wants_json_output(&args));

    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
    ];
    assert!(!wants_json_output(&args));

    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--jsonify".to_string(),
    ];
    assert!(!wants_json_output(&args));

    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--json=1".to_string(),
    ];
    assert!(!wants_json_output(&args));
}

#[test]
fn extract_non_empty_flag_value_trims_and_reads_first_match() {
    let args = vec![
        "--namespace".to_string(),
        "  cef-public  ".to_string(),
        "--namespace".to_string(),
        "cef-alt".to_string(),
    ];
    assert_eq!(
        extract_non_empty_flag_value(&args, "--namespace"),
        Some("cef-public".to_string())
    );
}

#[test]
fn extract_non_empty_flag_value_returns_none_for_missing_or_blank() {
    let missing_args = vec!["--node".to_string(), "node-client".to_string()];
    assert_eq!(
        extract_non_empty_flag_value(&missing_args, "--namespace"),
        None
    );

    let blank_args = vec!["--namespace".to_string(), "   ".to_string()];
    assert_eq!(
        extract_non_empty_flag_value(&blank_args, "--namespace"),
        None
    );

    let missing_value_args = vec!["--namespace".to_string()];
    assert_eq!(
        extract_non_empty_flag_value(&missing_value_args, "--namespace"),
        None
    );

    let blank_then_value_args = vec![
        "--namespace".to_string(),
        "   ".to_string(),
        "--namespace".to_string(),
        "cef-public".to_string(),
    ];
    assert_eq!(
        extract_non_empty_flag_value(&blank_then_value_args, "--namespace"),
        None
    );

    let next_flag_args = vec![
        "--namespace".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
    ];
    assert_eq!(
        extract_non_empty_flag_value(&next_flag_args, "--namespace"),
        None
    );

    let node_next_flag_args = vec![
        "--node".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh".to_string(),
    ];
    assert_eq!(
        extract_non_empty_flag_value(&node_next_flag_args, "--node"),
        None
    );

    let next_json_flag_args = vec![
        "--namespace".to_string(),
        "--json".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
    ];
    assert_eq!(
        extract_non_empty_flag_value(&next_json_flag_args, "--namespace"),
        None
    );

    let next_json_inline_flag_args = vec![
        "--namespace".to_string(),
        "--json=1".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
    ];
    assert_eq!(
        extract_non_empty_flag_value(&next_json_inline_flag_args, "--namespace"),
        None
    );
}
