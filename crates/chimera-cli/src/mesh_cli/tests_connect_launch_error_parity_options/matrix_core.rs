use super::*;
#[test]
fn connect_and_launch_error_contracts_match_on_numeric_options_stage_matrix() {
    let cases: [(&str, Vec<String>); 6] = [
        (
            "mx_timeout_invalid_alpha",
            vec!["--timeout-ms".to_string(), "abc".to_string()],
        ),
        (
            "mx_timeout_invalid_decimal",
            vec!["--timeout-ms".to_string(), "1.5".to_string()],
        ),
        (
            "mx_table_entries_invalid_alpha",
            vec!["--table-max-entries".to_string(), "abc".to_string()],
        ),
        (
            "mx_table_entries_invalid_negative",
            vec!["--table-max-entries".to_string(), "-1".to_string()],
        ),
        (
            "mx_table_per_region_invalid_decimal",
            vec!["--table-max-per-region".to_string(), "1.5".to_string()],
        ),
        (
            "mx_table_stale_invalid_alpha",
            vec!["--table-stale-after".to_string(), "abc".to_string()],
        ),
    ];

    for (case_id, mut extra_flags) in cases {
        let mut args = base_args();
        args.append(&mut extra_flags);
        assert_options_stage_case(case_id, args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_options_stage_matrix() {
    let cases: [(&str, Vec<String>); 6] = [
        (
            "mx_duplicate_timeout",
            vec![
                "--timeout-ms".to_string(),
                "25".to_string(),
                "--timeout-ms".to_string(),
                "50".to_string(),
            ],
        ),
        (
            "mx_duplicate_table_entries",
            vec![
                "--table-max-entries".to_string(),
                "256".to_string(),
                "--table-max-entries".to_string(),
                "128".to_string(),
            ],
        ),
        (
            "mx_duplicate_table_per_region",
            vec![
                "--table-max-per-region".to_string(),
                "64".to_string(),
                "--table-max-per-region".to_string(),
                "32".to_string(),
            ],
        ),
        (
            "mx_duplicate_table_stale_after",
            vec![
                "--table-stale-after".to_string(),
                "10".to_string(),
                "--table-stale-after".to_string(),
                "20".to_string(),
            ],
        ),
        (
            "mx_duplicate_policy_payload",
            vec![
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=2;mesh_min_reliability=70".to_string(),
            ],
        ),
        (
            "mx_duplicate_node",
            vec![
                "--node".to_string(),
                "node-client".to_string(),
                "--node".to_string(),
                "node-alt".to_string(),
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
fn connect_and_launch_error_contracts_match_on_missing_value_options_stage_matrix() {
    let cases: [(&str, Vec<String>); 6] = [
        ("mx_missing_timeout", vec!["--timeout-ms".to_string()]),
        (
            "mx_missing_table_entries",
            vec!["--table-max-entries".to_string()],
        ),
        (
            "mx_missing_table_per_region",
            vec!["--table-max-per-region".to_string()],
        ),
        (
            "mx_missing_table_stale",
            vec!["--table-stale-after".to_string()],
        ),
        ("mx_missing_profile", vec!["--traffic-profile".to_string()]),
        (
            "mx_missing_policy_payload",
            vec!["--policy-payload".to_string()],
        ),
    ];

    for (case_id, mut extra_flags) in cases {
        let mut args = base_args();
        args.append(&mut extra_flags);
        assert_options_stage_case(case_id, args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_duplicate_identity_options_stage_matrix() {
    let cases: [(&str, Vec<String>); 4] = [
        (
            "mx_duplicate_invite_token",
            vec![
                "--invite-token".to_string(),
                "token-a".to_string(),
                "--invite-token".to_string(),
                "token-b".to_string(),
            ],
        ),
        (
            "mx_duplicate_failed_node",
            vec![
                "--failed-node".to_string(),
                "n1".to_string(),
                "--failed-node".to_string(),
                "n2".to_string(),
            ],
        ),
        (
            "mx_duplicate_cooldown_node",
            vec![
                "--cooldown-node".to_string(),
                "n1".to_string(),
                "--cooldown-node".to_string(),
                "n2".to_string(),
            ],
        ),
        (
            "mx_duplicate_traffic_profile",
            vec![
                "--traffic-profile".to_string(),
                "high_speed_anonymous".to_string(),
                "--traffic-profile".to_string(),
                "privacy_first".to_string(),
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
fn connect_and_launch_error_contracts_match_on_profile_conflict_options_stage_matrix() {
    let cases: [(&str, Vec<String>); 4] = [
        (
            "mx_policy_profile_conflict",
            vec![
                "--policy-payload".to_string(),
                "allow=mesh;mesh_max_peers=1;mesh_min_reliability=80".to_string(),
                "--traffic-profile".to_string(),
                "high_speed_anonymous".to_string(),
            ],
        ),
        (
            "mx_invalid_profile_hyphenated",
            vec![
                "--traffic-profile".to_string(),
                "high-speed-anonymous".to_string(),
            ],
        ),
        (
            "mx_invalid_profile_unknown",
            vec!["--traffic-profile".to_string(), "ultra_mode".to_string()],
        ),
        (
            "mx_profile_and_payload_blank_conflict",
            vec![
                "--policy-payload".to_string(),
                String::new(),
                "--traffic-profile".to_string(),
                "privacy_first".to_string(),
            ],
        ),
    ];

    for (case_id, mut extra_flags) in cases {
        let mut args = base_args();
        if case_id == "mx_policy_profile_conflict" {
            // Replace default policy-payload with matrix-provided flags.
            args.truncate(4);
            args.push("--peer".to_string());
            args.push("n1@127.0.0.1:1@eu@20@90".to_string());
        }
        args.append(&mut extra_flags);
        assert_options_stage_case(case_id, args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_legacy_stale_ticks_options_stage_matrix() {
    let cases: [(&str, Vec<String>); 3] = [
        (
            "mx_legacy_stale_ticks_invalid_alpha",
            vec!["--table-stale-after-ticks".to_string(), "abc".to_string()],
        ),
        (
            "mx_legacy_stale_ticks_invalid_negative",
            vec!["--table-stale-after-ticks".to_string(), "-1".to_string()],
        ),
        (
            "mx_legacy_stale_ticks_missing_value",
            vec!["--table-stale-after-ticks".to_string()],
        ),
    ];

    for (case_id, mut extra_flags) in cases {
        let mut args = base_args();
        args.append(&mut extra_flags);
        assert_options_stage_case(case_id, args);
    }
}

#[test]
fn connect_and_launch_error_contracts_match_on_mixed_numeric_error_stage_matrix() {
    let cases: [(&str, Vec<String>); 4] = [
        (
            "mx_mixed_timeout_and_table_entries_invalid",
            vec![
                "--timeout-ms".to_string(),
                "abc".to_string(),
                "--table-max-entries".to_string(),
                "-1".to_string(),
            ],
        ),
        (
            "mx_mixed_table_per_region_and_stale_invalid",
            vec![
                "--table-max-per-region".to_string(),
                "1.5".to_string(),
                "--table-stale-after".to_string(),
                "abc".to_string(),
            ],
        ),
        (
            "mx_mixed_legacy_and_new_stale_invalid",
            vec![
                "--table-stale-after-ticks".to_string(),
                "abc".to_string(),
                "--table-stale-after".to_string(),
                "-1".to_string(),
            ],
        ),
        (
            "mx_mixed_timeout_missing_and_entries_invalid",
            vec![
                "--timeout-ms".to_string(),
                "--table-max-entries".to_string(),
                "abc".to_string(),
            ],
        ),
    ];

    for (case_id, mut extra_flags) in cases {
        let mut args = base_args();
        args.append(&mut extra_flags);
        assert_options_stage_case(case_id, args);
    }
}
