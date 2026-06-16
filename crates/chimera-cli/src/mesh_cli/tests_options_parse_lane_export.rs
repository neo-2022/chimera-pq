use std::fs;

use crate::mesh_cli::options::parse_mesh_route_explain_options;

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
fn parse_mesh_route_explain_options_accepts_transit_lane_bindings_out() {
    let mut args = base_args();
    args.extend([
        "--transit-lane-bindings-out".to_string(),
        "  /tmp/chimera-lanes.csv  ".to_string(),
    ]);

    let parsed = parse_mesh_route_explain_options(&args)
        .unwrap_or_else(|error| unreachable!("options should parse: {error}"));

    assert_eq!(
        parsed.transit_lane_bindings_out_path.as_deref(),
        Some("/tmp/chimera-lanes.csv")
    );
}

#[test]
fn parse_mesh_route_explain_options_rejects_bad_transit_lane_bindings_out() {
    let mut blank = base_args();
    blank.extend(["--transit-lane-bindings-out".to_string(), "   ".to_string()]);
    assert!(parse_mesh_route_explain_options(&blank).is_err());

    let mut duplicate = base_args();
    duplicate.extend([
        "--transit-lane-bindings-out".to_string(),
        "/tmp/a.csv".to_string(),
        "--transit-lane-bindings-out".to_string(),
        "/tmp/b.csv".to_string(),
    ]);
    assert_eq!(
        parse_mesh_route_explain_options(&duplicate).err(),
        Some("duplicate singleton flag '--transit-lane-bindings-out'".to_string())
    );
}

#[test]
fn route_explain_exports_transit_lane_bindings_from_dps_plan() {
    let out = temp_out_file("route_explain_lane_bindings");
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        concat!(
            "mesh_allowed_regions=eu;",
            "mesh_multipath_mode=flow_shard;",
            "mesh_route_binding_id=7009"
        )
        .to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "--peer".to_string(),
        "n2@198.51.100.2:443@eu@25@91".to_string(),
        "--transit-lane-bindings-out".to_string(),
        out.to_string_lossy().to_string(),
    ];

    let rc = super::mesh_command("usage", Some("route-explain"), &args);
    let body = fs::read_to_string(&out).unwrap_or_else(|error| unreachable!("{error}"));

    assert_eq!(rc, 0);
    assert!(body.starts_with("# route_id,lane_index,endpoint\n"));
    assert!(body.contains("7009,0,"));
    assert!(body.contains("7009,1,"));
    assert!(body.contains("198.51.100.1:443"));
    assert!(body.contains("198.51.100.2:443"));
    let _ = fs::remove_file(out);
}

#[test]
fn route_explain_exports_transit_lane_bindings_from_failover_plan() {
    let out = temp_out_file("route_explain_lane_bindings_failover_plan");
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        concat!(
            "mesh_allowed_regions=eu;",
            "mesh_multipath_mode=flow_shard;",
            "mesh_route_binding_id=7011"
        )
        .to_string(),
        "--peer".to_string(),
        "node-a@198.51.100.11:443@eu@10@95".to_string(),
        "--peer".to_string(),
        "node-b@198.51.100.12:443@eu@20@94".to_string(),
        "--peer".to_string(),
        "node-c@198.51.100.13:443@eu@30@93".to_string(),
        "--failed-node".to_string(),
        "node-a".to_string(),
        "--transit-lane-bindings-out".to_string(),
        out.to_string_lossy().to_string(),
    ];

    let rc = super::mesh_command("usage", Some("route-explain"), &args);
    let body = fs::read_to_string(&out).unwrap_or_else(|error| unreachable!("{error}"));

    assert_eq!(rc, 0);
    assert!(body.contains("7011,0,198.51.100.12:443"));
    assert!(body.contains("7011,1,198.51.100.13:443"));
    assert!(!body.contains("198.51.100.11:443"));
    let _ = fs::remove_file(out);
}

#[test]
fn route_explain_exports_transit_lane_bindings_from_cooldown_plan() {
    let out = temp_out_file("route_explain_lane_bindings_cooldown_plan");
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        concat!(
            "mesh_allowed_regions=eu;",
            "mesh_multipath_mode=flow_shard;",
            "mesh_route_binding_id=7012"
        )
        .to_string(),
        "--peer".to_string(),
        "node-a@198.51.100.21:443@eu@10@95".to_string(),
        "--peer".to_string(),
        "node-b@198.51.100.22:443@eu@20@94".to_string(),
        "--peer".to_string(),
        "node-c@198.51.100.23:443@eu@30@93".to_string(),
        "--cooldown-node".to_string(),
        "node-a".to_string(),
        "--transit-lane-bindings-out".to_string(),
        out.to_string_lossy().to_string(),
    ];

    let rc = super::mesh_command("usage", Some("route-explain"), &args);
    let body = fs::read_to_string(&out).unwrap_or_else(|error| unreachable!("{error}"));

    assert_eq!(rc, 0);
    assert!(body.contains("7012,0,198.51.100.22:443"));
    assert!(body.contains("7012,1,198.51.100.23:443"));
    assert!(!body.contains("198.51.100.21:443"));
    let _ = fs::remove_file(out);
}

#[test]
fn route_explain_transit_lane_bindings_export_rejects_ambiguous_scenarios() {
    let out = temp_out_file("route_explain_lane_bindings_ambiguous");
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard;mesh_route_binding_id=7013"
            .to_string(),
        "--peer".to_string(),
        "node-a@198.51.100.31:443@eu@10@95".to_string(),
        "--peer".to_string(),
        "node-b@198.51.100.32:443@eu@20@94".to_string(),
        "--peer".to_string(),
        "node-c@198.51.100.33:443@eu@30@93".to_string(),
        "--failed-node".to_string(),
        "node-a".to_string(),
        "--cooldown-node".to_string(),
        "node-b".to_string(),
        "--transit-lane-bindings-out".to_string(),
        out.to_string_lossy().to_string(),
    ];

    let rc = super::mesh_command("usage", Some("route-explain"), &args);

    assert_eq!(rc, 2);
    assert!(!out.exists());
}

#[test]
fn route_explain_transit_lane_bindings_export_fails_closed_without_route_binding() {
    let out = temp_out_file("route_explain_lane_bindings_missing_route");
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "--peer".to_string(),
        "n2@198.51.100.2:443@eu@25@91".to_string(),
        "--transit-lane-bindings-out".to_string(),
        out.to_string_lossy().to_string(),
    ];

    let rc = super::mesh_command("usage", Some("route-explain"), &args);

    assert_eq!(rc, 2);
    assert!(!out.exists());
}

#[test]
fn route_explain_transit_lane_bindings_export_error_writes_error_json_to_out() {
    let json_out = temp_out_file("route_explain_lane_bindings_error_json");
    let lane_out = json_out.with_extension("csv");
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard".to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "--peer".to_string(),
        "n2@198.51.100.2:443@eu@25@91".to_string(),
        "--out".to_string(),
        json_out.to_string_lossy().to_string(),
        "--transit-lane-bindings-out".to_string(),
        lane_out.to_string_lossy().to_string(),
    ];

    let rc = super::mesh_command("usage", Some("route-explain"), &args);
    let body = fs::read_to_string(&json_out).unwrap_or_else(|error| unreachable!("{error}"));
    let parsed: serde_json::Value =
        serde_json::from_str(&body).unwrap_or_else(|error| unreachable!("{error}"));

    assert_eq!(rc, 2);
    assert_eq!(parsed["status"].as_str(), Some("error"));
    assert_eq!(
        parsed["error_stage"].as_str(),
        Some("transit_lane_bindings_export")
    );
    assert!(!lane_out.exists());
    let _ = fs::remove_file(json_out);
}

#[test]
fn route_explain_transit_lane_bindings_export_does_not_persist_on_late_failover_failure() {
    let out = temp_out_file("route_explain_lane_bindings_late_failover");
    let lane_out = out.with_extension("csv");
    let args = vec![
        "--namespace".to_string(),
        "cef-public".to_string(),
        "--node".to_string(),
        "node-client".to_string(),
        "--policy-payload".to_string(),
        "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard;mesh_route_binding_id=7010"
            .to_string(),
        "--peer".to_string(),
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "--peer".to_string(),
        "n2@198.51.100.2:443@eu@25@91".to_string(),
        "--failed-node".to_string(),
        "n-missing".to_string(),
        "--transit-lane-bindings-out".to_string(),
        lane_out.to_string_lossy().to_string(),
    ];

    let rc = super::mesh_command("usage", Some("route-explain"), &args);

    assert_eq!(rc, 2);
    assert!(!lane_out.exists());
}
