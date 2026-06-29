use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshRuntime};

fn dps_plan_explain(payload: &str) -> Vec<String> {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-eu".to_string(),
            endpoint: "198.51.100.50:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-us".to_string(),
            endpoint: "198.51.100.51:443".to_string(),
            region: "us".to_string(),
            load_score: 12,
            reliability_score: 92,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    runtime
        .plan_path_from_dps_payload(&req, payload)
        .unwrap_or_else(|e| unreachable!("{e}"))
        .explain
}

fn line_position(explain: &[String], prefix: &str) -> Option<usize> {
    explain.iter().position(|line| line.starts_with(prefix))
}

fn assert_before(explain: &[String], earlier: &str, later: &str) {
    let earlier_position = line_position(explain, earlier);
    let later_position = line_position(explain, later);
    assert!(
        matches!(
            (earlier_position, later_position),
            (Some(earlier_position), Some(later_position)) if earlier_position < later_position
        ),
        "{earlier} must appear before {later}"
    );
}

#[test]
fn dps_payload_metadata_tail_keeps_summary_order() {
    let explain = dps_plan_explain(
        "mesh_allowed_regions=eu;mesh_max_peers=1;mesh_max_selected_per_region=1;mesh_min_distinct_regions=1;mesh_traffic_class=gaming_fps;mesh_multipath_mode=standby_only;mesh_continuity_policy=same_egress_only",
    );

    assert_before(
        &explain,
        "plan_setup_discovery_table_compact=",
        "dps_payload_explain_contract_version=",
    );
    assert_before(
        &explain,
        "dps_payload_explain_contract_version=",
        "policy_source=dps_payload",
    );
    assert_before(&explain, "policy_source=dps_payload", "dps_payload_origin=");
    assert_before(
        &explain,
        "dps_payload_origin=",
        "dps_payload_mesh_field_count=",
    );
    assert_before(
        &explain,
        "dps_payload_mesh_keys=",
        "dps_payload_traffic_class=",
    );
    assert_before(
        &explain,
        "dps_payload_traffic_profile=",
        "preemptive_shadow_switch_mode=",
    );
    assert_before(
        &explain,
        "dps_payload_hints_summary=",
        "dps_payload_switch_guard=",
    );
    assert_before(
        &explain,
        "dps_payload_switch_guard=",
        "dps_payload_confirm_summary=",
    );
    assert_before(
        &explain,
        "dps_payload_confirm_summary=",
        "dps_payload_risk_summary=",
    );
    assert_before(
        &explain,
        "dps_payload_risk_summary=",
        "dps_payload_preemptive_switch_confidence_summary=",
    );
    assert_before(
        &explain,
        "dps_payload_preemptive_shadow_compact=",
        "dps_payload_consistency_source_matrix=",
    );
    assert_before(
        &explain,
        "dps_payload_consistency_source_matrix=",
        "dps_payload_standby_mode=",
    );
    assert_before(
        &explain,
        "dps_payload_standby_hot_ready=",
        "dps_payload_standby_summary=",
    );
}

#[test]
fn dps_payload_metadata_tail_does_not_export_raw_payload_notes_or_binding_value() {
    let explain = dps_plan_explain(
        "mesh_allowed_regions=eu;mesh_max_peers=1;mesh_max_selected_per_region=1;mesh_route_binding_id=7009;non_mesh_note=SECRET_DESTINATION_EXAMPLE",
    );
    let joined = explain.join("\n");

    assert!(!joined.contains("SECRET_DESTINATION_EXAMPLE"));
    assert!(!joined.contains("non_mesh_note"));
    assert!(!joined.contains("mesh_route_binding_id=7009"));
    assert!(!joined.contains("route_binding_id=7009"));
    assert!(!joined.contains("198.51.100.50:443"));
    assert!(!joined.contains("198.51.100.51:443"));
    assert!(joined.contains("dps_payload_mesh_keys=mesh_allowed_regions,mesh_max_peers,mesh_max_selected_per_region,mesh_route_binding_id"));
    assert!(joined.contains("multipath_schedule_route_binding_configured=false"));
}
