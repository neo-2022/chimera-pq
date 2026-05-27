use crate::mesh_cli::options::{parse_mesh_peer_spec, parse_mesh_route_explain_options};
use std::fs;

use super::tests_json_utils::temp_out_file;

fn base_args() -> Vec<String> {
    vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=2;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
    ]
}

#[test]
fn parse_mesh_route_explain_options_trims_values() {
    let mut args = base_args();
    args[1] = "  cef-public  ".to_string();
    args[3] = "  node-client  ".to_string();
    args[5] = "  allow=mesh;mesh_max_peers=2;mesh_min_reliability=80  ".to_string();
    args[7] = "  n1@198.51.100.1:443@eu@20@90  ".to_string();
    args.extend([
        "--failed-node".to_string(),
        "  n1  ".to_string(),
        "--cooldown-node".to_string(),
        "  n2  ".to_string(),
        "--out".to_string(),
        "  /tmp/mesh.json  ".to_string(),
        "--table-max-entries".to_string(),
        " 42 ".to_string(),
        "--table-max-per-region".to_string(),
        " 7 ".to_string(),
        "--table-stale-after".to_string(),
        " 3 ".to_string(),
    ]);
    let parsed = parse_mesh_route_explain_options(&args);
    assert!(parsed.is_ok());
    let parsed = parsed.ok();
    assert_eq!(
        parsed.as_ref().map(|value| value.namespace.as_str()),
        Some("cef-public")
    );
    assert_eq!(
        parsed.as_ref().map(|value| value.node_name.as_str()),
        Some("node-client")
    );
    assert_eq!(
        parsed.as_ref().map(|value| value.policy_payload.as_str()),
        Some("allow=mesh;mesh_max_peers=2;mesh_min_reliability=80")
    );
    assert_eq!(
        parsed
            .as_ref()
            .and_then(|value| value.peers.first().map(String::as_str)),
        Some("n1@198.51.100.1:443@eu@20@90")
    );
    assert_eq!(
        parsed
            .as_ref()
            .and_then(|value| value.failed_node_id.as_deref()),
        Some("n1")
    );
    assert_eq!(
        parsed
            .as_ref()
            .and_then(|value| value.cooldown_node_id.as_deref()),
        Some("n2")
    );
    assert_eq!(
        parsed.as_ref().and_then(|value| value.out_path.as_deref()),
        Some("/tmp/mesh.json")
    );
    assert_eq!(
        parsed.as_ref().and_then(|value| value.table_max_entries),
        Some(42)
    );
    assert_eq!(
        parsed
            .as_ref()
            .and_then(|value| value.table_max_entries_per_region),
        Some(7)
    );
    assert_eq!(
        parsed
            .as_ref()
            .and_then(|value| value.table_stale_after_ticks),
        Some(3)
    );
}

#[test]
fn parse_mesh_route_explain_options_rejects_blank_required_values() {
    let mut blank_namespace = base_args();
    blank_namespace[1] = "   ".to_string();
    assert!(parse_mesh_route_explain_options(&blank_namespace).is_err());

    let mut blank_node = base_args();
    blank_node[3] = "   ".to_string();
    assert!(parse_mesh_route_explain_options(&blank_node).is_err());

    let mut blank_policy = base_args();
    blank_policy[5] = "   ".to_string();
    assert!(parse_mesh_route_explain_options(&blank_policy).is_err());

    let mut blank_peer = base_args();
    blank_peer[7] = "   ".to_string();
    assert!(parse_mesh_route_explain_options(&blank_peer).is_err());
}

#[test]
fn parse_mesh_route_explain_options_rejects_blank_optional_values_when_present() {
    let mut blank_failed = base_args();
    blank_failed.extend(["--failed-node".to_string(), "   ".to_string()]);
    assert!(parse_mesh_route_explain_options(&blank_failed).is_err());

    let mut blank_cooldown = base_args();
    blank_cooldown.extend(["--cooldown-node".to_string(), "   ".to_string()]);
    assert!(parse_mesh_route_explain_options(&blank_cooldown).is_err());

    let mut blank_out = base_args();
    blank_out.extend(["--out".to_string(), "   ".to_string()]);
    assert!(parse_mesh_route_explain_options(&blank_out).is_err());
}

#[test]
fn parse_mesh_route_explain_options_returns_specific_blank_flag_errors() {
    let mut blank_policy = base_args();
    blank_policy[5] = "   ".to_string();
    assert_eq!(
        parse_mesh_route_explain_options(&blank_policy).err(),
        Some("blank value for flag '--policy-payload'".to_string())
    );

    let mut blank_peer = base_args();
    blank_peer[7] = "   ".to_string();
    assert_eq!(
        parse_mesh_route_explain_options(&blank_peer).err(),
        Some("blank value for flag '--peer'".to_string())
    );
}

#[test]
fn parse_mesh_route_explain_options_returns_specific_missing_value_errors() {
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=2;mesh_min_reliability=80".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "--out".to_string(),
    ];
    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("missing value for flag '--out'".to_string())
    );
}

#[test]
fn parse_mesh_route_explain_options_prioritizes_required_field_errors() {
    let args = vec![
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=2;mesh_min_reliability=80".to_string(),
    ];
    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("missing --namespace".to_string())
    );

    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=2;mesh_min_reliability=80".to_string(),
    ];
    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("missing --node".to_string())
    );

    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
    ];
    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("missing --policy-payload".to_string())
    );
}

#[test]
fn parse_mesh_route_explain_options_rejects_duplicate_singleton_flags() {
    let mut dup_namespace = base_args();
    dup_namespace.extend(["--namespace".to_string(), "cef-alt".to_string()]);
    assert_eq!(
        parse_mesh_route_explain_options(&dup_namespace).err(),
        Some("duplicate singleton flag '--namespace'".to_string())
    );

    let mut dup_node = base_args();
    dup_node.extend(["--node".to_string(), "node-alt".to_string()]);
    assert_eq!(
        parse_mesh_route_explain_options(&dup_node).err(),
        Some("duplicate singleton flag '--node'".to_string())
    );

    let mut dup_policy = base_args();
    dup_policy.extend([
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=90".to_string(),
    ]);
    assert_eq!(
        parse_mesh_route_explain_options(&dup_policy).err(),
        Some("duplicate singleton flag '--policy-payload'".to_string())
    );

    let mut dup_failed = base_args();
    dup_failed.extend([
        "--failed-node".to_string(),
        "n1".to_string(),
        "--failed-node".to_string(),
        "n2".to_string(),
    ]);
    assert_eq!(
        parse_mesh_route_explain_options(&dup_failed).err(),
        Some("duplicate singleton flag '--failed-node'".to_string())
    );

    let mut dup_cooldown = base_args();
    dup_cooldown.extend([
        "--cooldown-node".to_string(),
        "n1".to_string(),
        "--cooldown-node".to_string(),
        "n2".to_string(),
    ]);
    assert_eq!(
        parse_mesh_route_explain_options(&dup_cooldown).err(),
        Some("duplicate singleton flag '--cooldown-node'".to_string())
    );

    let mut dup_invite = base_args();
    dup_invite.extend([
        "--invite-token".to_string(),
        "tok-a".to_string(),
        "--invite-token".to_string(),
        "tok-b".to_string(),
    ]);
    assert_eq!(
        parse_mesh_route_explain_options(&dup_invite).err(),
        Some("duplicate singleton flag '--invite-token'".to_string())
    );
}

#[test]
fn parse_mesh_route_explain_options_returns_specific_unknown_flag_error() {
    let mut args = base_args();
    args.extend(["--mystery-flag".to_string(), "x".to_string()]);
    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("unknown flag '--mystery-flag'".to_string())
    );
}

#[test]
fn parse_mesh_route_explain_options_rejects_positional_arguments() {
    let mut args = base_args();
    args.push("positional".to_string());
    assert_eq!(
        parse_mesh_route_explain_options(&args).err(),
        Some("unexpected positional argument 'positional'".to_string())
    );
}

#[test]
fn options_parse_error_writes_json_to_out_without_json_flag() {
    let out = temp_out_file("options_parse_err");
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "allow=mesh;mesh_max_peers=1;mesh_min_reliability=95".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "--namespace".to_string(),
        "cef-alt".to_string(),
        "--out".to_string(),
        out.to_string_lossy().to_string(),
    ];
    let rc = super::mesh_command("usage", Some("route-explain"), &args);
    assert_eq!(rc, 2);
    let json = fs::read_to_string(&out).ok();
    assert!(json.is_some());
    let parsed: Option<serde_json::Value> = json
        .as_ref()
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok());
    assert_eq!(
        parsed
            .as_ref()
            .and_then(|value| value.get("error_stage"))
            .and_then(|value| value.as_str()),
        Some("options_parse")
    );
    assert_eq!(
        parsed
            .as_ref()
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str()),
        Some("error")
    );
    assert_eq!(
        parsed
            .as_ref()
            .and_then(|value| value.get("error"))
            .and_then(|value| value.as_str()),
        Some("duplicate singleton flag '--namespace'")
    );
    let _ = fs::remove_file(&out);
}

#[test]
fn parse_mesh_peer_spec_trims_fields_and_scores() {
    let parsed = parse_mesh_peer_spec(" n1 @ 198.51.100.1:443 @ eu-west @ 20 @ 90 ");
    assert!(parsed.is_ok());
    let parsed = parsed.ok();
    assert_eq!(
        parsed.as_ref().map(|value| value.node_id.as_str()),
        Some("n1")
    );
    assert_eq!(
        parsed.as_ref().map(|value| value.endpoint.as_str()),
        Some("198.51.100.1:443")
    );
    assert_eq!(
        parsed.as_ref().map(|value| value.region.as_str()),
        Some("eu-west")
    );
    assert_eq!(parsed.as_ref().map(|value| value.load_score), Some(20));
    assert_eq!(
        parsed.as_ref().map(|value| value.reliability_score),
        Some(90)
    );
}

#[test]
fn parse_mesh_peer_spec_rejects_blank_core_fields() {
    assert_eq!(
        parse_mesh_peer_spec(" @198.51.100.1:443@eu@20@90").err(),
        Some("blank peer node_id".to_string())
    );
    assert_eq!(
        parse_mesh_peer_spec("n1@   @eu@20@90").err(),
        Some("blank peer endpoint".to_string())
    );
    assert_eq!(
        parse_mesh_peer_spec("n1@198.51.100.1:443@   @20@90").err(),
        Some("blank peer region".to_string())
    );
}

#[test]
fn parse_mesh_peer_spec_rejects_non_numeric_scores() {
    assert_eq!(
        parse_mesh_peer_spec("n1@198.51.100.1:443@eu@x@90").err(),
        Some("invalid load score".to_string())
    );
    assert_eq!(
        parse_mesh_peer_spec("n1@198.51.100.1:443@eu@20@y").err(),
        Some("invalid reliability score".to_string())
    );
}
