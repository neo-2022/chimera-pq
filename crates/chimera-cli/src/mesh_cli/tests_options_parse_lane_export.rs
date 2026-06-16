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
