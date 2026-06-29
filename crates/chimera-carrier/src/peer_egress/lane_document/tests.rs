use super::{
    TransitLaneDocument, load_transit_lane_document, load_transit_lane_registrations,
    parse_transit_lane_document, parse_transit_lane_registrations, render_transit_lane_document,
    render_transit_lane_registrations, transit_lane_document_from_mesh_plan,
    transit_lane_registrations_from_mesh_plan, write_transit_lane_document_from_mesh_plan,
    write_transit_lane_registrations_from_mesh_plan,
};
use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::live_lane_selection::select_carrier_lane_from_mesh_plan;
use chimera_mesh::{
    MeshCarrierLaneBinding, MeshDiscoveryRecord, MeshJoinRequest, MeshMultipathFlowKey,
    MeshMultipathLaneRole, MeshPathPlan, MeshRouteBindingId, MeshRuntime,
};

fn mesh_binding(route: u64, lane: usize) -> MeshCarrierLaneBinding {
    MeshCarrierLaneBinding {
        route_binding_id: MeshRouteBindingId::new(route)
            .unwrap_or_else(|error| unreachable!("{error}")),
        lane_id: lane,
        peer_node_id: "node-sensitive".to_string(),
        carrier_endpoint: "198.51.100.10:443".to_string(),
        role: MeshMultipathLaneRole::Active,
        weight_pct: 100,
        capacity_weight_pct: 90,
    }
}

fn multipath_plan() -> Result<MeshPathPlan, String> {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
    runtime.merge_discovery(
        "seed-b",
        &[
            MeshDiscoveryRecord {
                node_id: "node-a".to_string(),
                endpoint: "198.51.100.31:443".to_string(),
                region: "eu".to_string(),
                load_score: 20,
                reliability_score: 90,
            },
            MeshDiscoveryRecord {
                node_id: "node-b".to_string(),
                endpoint: "198.51.100.32:443".to_string(),
                region: "eu".to_string(),
                load_score: 22,
                reliability_score: 91,
            },
        ],
    )?;
    runtime.plan_path_from_dps_payload(
        &MeshJoinRequest {
            namespace: "cef-public".to_string(),
            node_name: "node-client".to_string(),
            invite_token: None,
        },
        concat!(
            "mesh_allowed_regions=eu;",
            "mesh_max_peers=2;",
            "mesh_max_selected_per_region=2;",
            "mesh_multipath_mode=flow_shard;",
            "mesh_route_binding_id=7004"
        ),
    )
}

#[test]
fn mesh_lane_registration_uses_redacted_endpoint_and_matching_binding() -> Result<(), String> {
    let mesh = mesh_binding(77, 0);
    let registration = super::transit_lane_registration_from_mesh_lane(&mesh)?;
    let debug = format!("{registration:?}");

    assert_eq!(registration.binding().route_id().get(), 77);
    assert_eq!(registration.binding().lane_id().get(), 1);
    assert_eq!(registration.endpoint(), "198.51.100.10:443");
    assert!(!debug.contains("198.51.100.10:443"));
    assert!(!debug.contains("77"));
    assert!(debug.contains("<opaque>"));
    assert!(debug.contains("<redacted>"));
    Ok(())
}

#[test]
fn lane_registration_config_parses_and_rejects_duplicate_bindings() -> Result<(), String> {
    let parsed = parse_transit_lane_registrations(
        "# route,lane,endpoint\n77,0,198.51.100.10:443\n77,1,198.51.100.11:443\n",
    )?;

    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].binding().route_id().get(), 77);
    assert_eq!(parsed[0].binding().lane_id().get(), 1);
    assert_eq!(parsed[1].binding().lane_id().get(), 2);

    let error = match parse_transit_lane_registrations(
        "77,0,198.51.100.10:443\n77,0,198.51.100.11:443\n",
    ) {
        Ok(_) => return Err("duplicate transit binding must fail".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("ambiguous"));
    Ok(())
}

#[test]
fn lane_registration_config_rejects_zero_route_and_bad_endpoint() {
    assert!(parse_transit_lane_registrations("0,0,198.51.100.10:443\n").is_err());
    assert!(parse_transit_lane_registrations("77,0,not-an-endpoint\n").is_err());
}

#[test]
fn document_round_trips_snapshot_and_rows() -> Result<(), String> {
    let plan = multipath_plan()?;
    let document = transit_lane_document_from_mesh_plan(&plan)?;
    let rendered = render_transit_lane_document(&document)?;
    let reparsed = parse_transit_lane_document(&rendered)?;

    let reparsed_plan = reparsed
        .mesh_path_plan()?
        .ok_or_else(|| "plan snapshot missing".to_string())?;
    assert_eq!(reparsed_plan.namespace, plan.namespace);
    assert_eq!(reparsed_plan.join_mode, plan.join_mode);
    assert_eq!(
        reparsed_plan.multipath_schedule.mode,
        plan.multipath_schedule.mode
    );
    assert_eq!(
        reparsed_plan.multipath_schedule.carrier_lane_bindings.len(),
        plan.multipath_schedule.carrier_lane_bindings.len()
    );
    assert_eq!(
        reparsed.registrations().len(),
        document.registrations().len()
    );
    Ok(())
}

#[test]
fn document_borrowed_snapshot_plan_matches_owned_access() -> Result<(), String> {
    let plan = multipath_plan()?;
    let document = transit_lane_document_from_mesh_plan(&plan)?;
    let borrowed = document.require_mesh_path_plan_ref()?;
    let owned = document.require_mesh_path_plan()?;

    assert_eq!(borrowed.namespace, owned.namespace);
    assert_eq!(
        borrowed.multipath_schedule.carrier_lane_bindings.len(),
        owned.multipath_schedule.carrier_lane_bindings.len()
    );
    assert_eq!(borrowed.join_mode, owned.join_mode);
    Ok(())
}

#[test]
fn document_rejects_rows_that_diverge_from_snapshot() -> Result<(), String> {
    let plan = multipath_plan()?;
    let document = transit_lane_document_from_mesh_plan(&plan)?;
    let rendered = render_transit_lane_document(&document)?;
    let mut tampered = String::new();
    let mut changed = false;
    for line in rendered.lines() {
        let trimmed = line.trim();
        if !changed && !trimmed.is_empty() && !trimmed.starts_with('#') {
            let mut parts = trimmed.split(',').map(str::trim).collect::<Vec<_>>();
            if parts.len() != 6 {
                return Err("test requires plan-backed lane rows".to_string());
            }
            parts[3] = if parts[3] == "active" {
                "standby"
            } else {
                "active"
            };
            tampered.push_str(&parts.join(","));
            tampered.push('\n');
            changed = true;
            continue;
        }
        tampered.push_str(line);
        tampered.push('\n');
    }
    if !changed {
        return Err("test did not find a lane row to tamper".to_string());
    }

    let error = match parse_transit_lane_document(&tampered) {
        Ok(_) => return Err("tampered lane row must fail".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("mismatches plan snapshot"));
    assert!(!error.contains("198.51.100"));
    Ok(())
}

#[test]
fn document_falls_back_to_registrations_without_snapshot() -> Result<(), String> {
    let document = TransitLaneDocument::new(
        vec![TransitLaneRegistration::new(
            super::transit_path_binding_from_mesh_lane(&mesh_binding(80, 0))?,
            "198.51.100.80:443".to_string(),
        )?],
        None,
    );
    let rendered = render_transit_lane_document(&document)?;
    let reparsed = parse_transit_lane_document(&rendered)?;

    assert!(reparsed.mesh_path_plan()?.is_none());
    assert_eq!(reparsed.registrations().len(), 1);
    Ok(())
}

#[test]
fn render_registration_file_round_trips() -> Result<(), String> {
    let registrations = transit_lane_registrations_from_mesh_plan(&multipath_plan()?)?;
    let rendered = render_transit_lane_registrations(&registrations)?;
    let reparsed = parse_transit_lane_registrations(&rendered)?;

    assert_eq!(reparsed.len(), registrations.len());
    Ok(())
}

#[test]
fn write_registration_file_round_trips() -> Result<(), String> {
    let plan = multipath_plan()?;
    let path = std::env::temp_dir().join(format!(
        "chimera-test-lane-bindings-{}-{}.csv",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("clock failed: {error}"))?
            .as_nanos()
    ));

    let written = write_transit_lane_registrations_from_mesh_plan(
        &plan,
        path.to_str()
            .ok_or_else(|| "temp path invalid utf-8".to_string())?,
    )?;
    let loaded = load_transit_lane_registrations(
        path.to_str()
            .ok_or_else(|| "temp path invalid utf-8".to_string())?,
    );
    let _ = std::fs::remove_file(&path);

    assert_eq!(written, plan.multipath_schedule.carrier_lane_bindings.len());
    assert!(loaded.is_ok());
    Ok(())
}

#[test]
fn write_document_file_replaces_existing_path_atomically() -> Result<(), String> {
    let plan_a = multipath_plan()?;
    let mut plan_b = multipath_plan()?;
    plan_b.multipath_schedule.lanes.reverse();
    plan_b.multipath_schedule.carrier_lane_bindings.reverse();
    plan_b.selected_peers.reverse();
    if let Some(lane) = plan_b.multipath_schedule.lanes.get_mut(1) {
        lane.lane_id = 2;
    }
    if let Some(binding) = plan_b.multipath_schedule.carrier_lane_bindings.get_mut(1) {
        binding.lane_id = 2;
    }

    let path = std::env::temp_dir().join(format!(
        "chimera-test-lane-document-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("clock failed: {error}"))?
            .as_nanos()
    ));

    let first = write_transit_lane_document_from_mesh_plan(
        &plan_a,
        path.to_str()
            .ok_or_else(|| "temp path invalid utf-8".to_string())?,
    )?;
    let loaded_a = load_transit_lane_document(
        path.to_str()
            .ok_or_else(|| "temp path invalid utf-8".to_string())?,
    )?;
    let second = write_transit_lane_document_from_mesh_plan(
        &plan_b,
        path.to_str()
            .ok_or_else(|| "temp path invalid utf-8".to_string())?,
    )?;
    let loaded_b = load_transit_lane_document(
        path.to_str()
            .ok_or_else(|| "temp path invalid utf-8".to_string())?,
    )?;
    let _ = std::fs::remove_file(&path);

    assert_eq!(first, plan_a.multipath_schedule.carrier_lane_bindings.len());
    assert_eq!(
        second,
        plan_b.multipath_schedule.carrier_lane_bindings.len()
    );
    assert_eq!(
        loaded_a
            .mesh_path_plan()?
            .ok_or_else(|| "first snapshot missing".to_string())?
            .selected_peers,
        plan_a.selected_peers
    );
    assert_eq!(
        loaded_b
            .mesh_path_plan()?
            .ok_or_else(|| "second snapshot missing".to_string())?
            .selected_peers,
        plan_b.selected_peers
    );

    let rebuilt_plan = loaded_b
        .mesh_path_plan()?
        .ok_or_else(|| "second snapshot missing".to_string())?;
    let before_document = loaded_a
        .mesh_path_plan()?
        .ok_or_else(|| "first snapshot missing".to_string())?;
    let mut changed = false;
    for index in 0..256usize {
        let flow = format!("rebuild-flow-{index}");
        let key = MeshMultipathFlowKey::from_opaque_flow_id(&flow)?;
        let before = select_carrier_lane_from_mesh_plan(&before_document, key);
        let after = select_carrier_lane_from_mesh_plan(&rebuilt_plan, key);
        if before.selected_binding != after.selected_binding {
            changed = true;
            break;
        }
    }
    assert!(changed);
    Ok(())
}

#[test]
fn parse_document_rejects_tampered_capacity_and_policy_contract() -> Result<(), String> {
    let plan = multipath_plan()?;
    let document = transit_lane_document_from_mesh_plan(&plan)?;
    let rendered = render_transit_lane_document(&document)?;
    let tampered = rendered
        .replace(
            "chimera_plan_local_traffic_reserve_pct=10",
            "chimera_plan_local_traffic_reserve_pct=0",
        )
        .replace(
            "chimera_plan_transit_capacity_budget_pct=90",
            "chimera_plan_transit_capacity_budget_pct=100",
        );

    assert!(parse_transit_lane_document(&tampered).is_err());
    assert!(
        parse_transit_lane_document(&rendered.replace(
            "chimera_plan_transit_payload_policy=sealed_opaque_only",
            "chimera_plan_transit_payload_policy=plaintext_forbidden"
        ))
        .is_err()
    );
    Ok(())
}
