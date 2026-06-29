use super::{
    CarrierLaneSelectionMode, select_carrier_binding_from_mesh_plan,
    select_carrier_binding_from_registrations, select_carrier_lane_from_mesh_plan,
    select_carrier_lane_from_registrations,
};
use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
use chimera_mesh::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshMultipathFlowAction, MeshMultipathFlowKey,
    MeshMultipathLaneRole, MeshRuntime,
};

fn registration(route: u64, lane: u16) -> Result<TransitLaneRegistration, String> {
    TransitLaneRegistration::new(
        TransitPathBinding::new(TransitRouteId::new(route)?, TransitLaneId::new(lane)?),
        format!("198.51.100.{lane}:443"),
    )
}

fn planned_registration(
    route: u64,
    lane: u16,
    role: MeshMultipathLaneRole,
    capacity_weight_pct: Option<u8>,
) -> Result<TransitLaneRegistration, String> {
    TransitLaneRegistration::new_with_lane_plan(
        TransitPathBinding::new(TransitRouteId::new(route)?, TransitLaneId::new(lane)?),
        format!("198.51.100.{lane}:443"),
        Some(role),
        Some(capacity_weight_pct.unwrap_or_default()),
        capacity_weight_pct,
    )
}

fn multipath_plan() -> Result<chimera_mesh::MeshPathPlan, String> {
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
            "mesh_multipath_mode=flow_shard;",
            "mesh_route_binding_id=7003"
        ),
    )
}

#[test]
fn single_registration_preserves_single_path_behavior() -> Result<(), String> {
    let registrations = vec![registration(7, 1)?];
    let key = MeshMultipathFlowKey::from_opaque_flow_id("opaque-flow")?;
    let selected = select_carrier_lane_from_registrations(&registrations, key)?;

    assert_eq!(selected.action, MeshMultipathFlowAction::Assigned);
    assert_eq!(selected.mode, CarrierLaneSelectionMode::SinglePath);
    assert_eq!(selected.selected_lane_id, Some(1));
    assert_eq!(selected.reason, "single_carrier_lane_selected");
    assert!(!selected.rebuild_recommended);
    Ok(())
}

#[test]
fn binding_backed_selection_uses_opaque_flow_key_for_multiple_lanes() -> Result<(), String> {
    let registrations = vec![
        registration(7, 1)?,
        registration(7, 2)?,
        registration(7, 3)?,
    ];
    let key = MeshMultipathFlowKey::from_opaque_flow_id("opaque-live-flow")?;
    let expected_slot = key.select_slot_index(registrations.len())?;
    let selected = select_carrier_lane_from_registrations(&registrations, key)?;

    assert_eq!(selected.action, MeshMultipathFlowAction::Assigned);
    assert_eq!(selected.mode, CarrierLaneSelectionMode::Multipath);
    assert_eq!(
        selected.selected_binding,
        Some(registrations[expected_slot].binding())
    );
    assert_eq!(selected.reason, "opaque_flow_slot_selected");
    Ok(())
}

#[test]
fn planned_registration_selection_uses_capacity_weights() -> Result<(), String> {
    let registrations = vec![
        planned_registration(7, 1, MeshMultipathLaneRole::Active, Some(70))?,
        planned_registration(7, 2, MeshMultipathLaneRole::Active, Some(20))?,
    ];
    let key = MeshMultipathFlowKey::from_opaque_flow_id("opaque-weighted-live-flow")?;
    let mut bucket = u16::try_from(key.select_slot_index(90)?)
        .map_err(|_| "test bucket overflow".to_string())?;
    let expected = if bucket < 70 {
        registrations[0].binding()
    } else {
        bucket = bucket.saturating_sub(70);
        if bucket < 20 {
            registrations[1].binding()
        } else {
            return Err("test weighted bucket did not match".to_string());
        }
    };
    let selected = select_carrier_lane_from_registrations(&registrations, key)?;

    assert_eq!(selected.action, MeshMultipathFlowAction::Assigned);
    assert_eq!(selected.mode, CarrierLaneSelectionMode::Multipath);
    assert_eq!(selected.selected_binding, Some(expected));
    assert_eq!(selected.active_binding_count, 2);
    assert_eq!(selected.reason, "weighted_capacity_lane_selected");
    assert!(
        selected
            .explain
            .iter()
            .any(|line| { line == "carrier_lane_selection_weight_policy=capacity_weighted" })
    );
    Ok(())
}

#[test]
fn planned_registration_selection_excludes_standby_lanes() -> Result<(), String> {
    let registrations = vec![
        planned_registration(7, 1, MeshMultipathLaneRole::Active, Some(90))?,
        planned_registration(7, 2, MeshMultipathLaneRole::Standby, Some(10))?,
    ];
    for index in 0..16 {
        let key = MeshMultipathFlowKey::from_opaque_flow_id(&format!("opaque-flow-{index}"))?;
        let selected = select_carrier_lane_from_registrations(&registrations, key)?;
        assert_eq!(selected.selected_binding, Some(registrations[0].binding()));
        assert_eq!(selected.active_binding_count, 1);
    }
    Ok(())
}

#[test]
fn planned_registration_selection_fails_closed_without_active_capacity() -> Result<(), String> {
    let registrations = vec![
        planned_registration(7, 1, MeshMultipathLaneRole::Active, None)?,
        planned_registration(7, 2, MeshMultipathLaneRole::Standby, Some(0))?,
    ];
    let key = MeshMultipathFlowKey::from_opaque_flow_id("opaque-bad-capacity")?;
    let error = match select_carrier_lane_from_registrations(&registrations, key) {
        Ok(_) => return Err("planned registration without capacity must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("capacity missing") || error.contains("capacity is zero"));
    Ok(())
}

#[test]
fn empty_binding_backed_selection_fails_closed() -> Result<(), String> {
    let key = MeshMultipathFlowKey::from_opaque_flow_id("opaque-live-flow")?;
    let error = match select_carrier_lane_from_registrations(&[], key) {
        Ok(_) => return Err("empty live lane registrations must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("no registrations"));
    Ok(())
}

#[test]
fn plan_backed_selection_returns_live_carrier_lane_binding() -> Result<(), String> {
    let plan = multipath_plan()?;
    let key = MeshMultipathFlowKey::from_opaque_flow_id("opaque-plan-flow")?;
    let selected = select_carrier_lane_from_mesh_plan(&plan, key);

    assert_eq!(selected.action, MeshMultipathFlowAction::Assigned);
    assert_eq!(selected.mode, CarrierLaneSelectionMode::Multipath);
    assert!(matches!(selected.selected_lane_id, Some(1 | 2)));
    assert_eq!(selected.reason, "active_carrier_binding_selected");
    assert!(
        selected
            .explain
            .iter()
            .any(|line| line == "carrier_lane_selection_privacy=sealed_opaque_only")
    );
    Ok(())
}

#[test]
fn plan_without_route_binding_fails_closed_and_recommends_runtime_wiring() -> Result<(), String> {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
    runtime.merge_discovery(
        "seed-b",
        &[MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.31:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        }],
    )?;
    let plan = runtime.plan_path_from_dps_payload(
        &MeshJoinRequest {
            namespace: "cef-public".to_string(),
            node_name: "node-client".to_string(),
            invite_token: None,
        },
        "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard",
    )?;
    let key = MeshMultipathFlowKey::from_opaque_flow_id("opaque-plan-flow")?;
    let selected = select_carrier_lane_from_mesh_plan(&plan, key);

    assert_eq!(selected.action, MeshMultipathFlowAction::FailClosed);
    assert_eq!(selected.selected_lane_id, None);
    assert_eq!(selected.reason, "route_binding_missing");
    assert!(selected.rebuild_recommended);
    assert_eq!(selected.rebuild_reason, "active_lanes_below_plan");
    Ok(())
}

#[test]
fn selection_debug_redacts_binding_and_flow_material() -> Result<(), String> {
    let registrations = vec![registration(777, 2)?, registration(777, 3)?];
    let key = MeshMultipathFlowKey::from_opaque_flow_id("SECRET_FLOW_SENTINEL")?;
    let selected = select_carrier_lane_from_registrations(&registrations, key)?;
    let debug = format!("{selected:?}");

    assert!(debug.contains("<opaque>"));
    assert!(!debug.contains("SECRET_FLOW_SENTINEL"));
    assert!(!debug.contains("777"));
    assert!(!debug.contains("lane_id: 2"));
    assert!(!debug.contains("lane_id: 3"));
    Ok(())
}

#[test]
fn binding_only_selection_matches_registration_selection_contract() -> Result<(), String> {
    let registrations = vec![
        planned_registration(7, 1, MeshMultipathLaneRole::Active, Some(70))?,
        planned_registration(7, 2, MeshMultipathLaneRole::Active, Some(20))?,
    ];
    let key = MeshMultipathFlowKey::from_opaque_flow_id("opaque-binding-only-live-flow")?;
    let selected = select_carrier_lane_from_registrations(&registrations, key)?;
    let binding = select_carrier_binding_from_registrations(&registrations, key)
        .map_err(|error| error.to_string())?;

    assert_eq!(selected.action, MeshMultipathFlowAction::Assigned);
    assert_eq!(selected.selected_binding, Some(binding));
    Ok(())
}

#[test]
fn binding_only_plan_selection_matches_mesh_plan_selection_contract() -> Result<(), String> {
    let plan = multipath_plan()?;
    let key = MeshMultipathFlowKey::from_opaque_flow_id("opaque-binding-only-plan-flow")?;
    let selected = select_carrier_lane_from_mesh_plan(&plan, key);
    let binding =
        select_carrier_binding_from_mesh_plan(&plan, key).map_err(|error| error.to_string())?;

    assert_eq!(selected.action, MeshMultipathFlowAction::Assigned);
    assert_eq!(selected.selected_binding, Some(binding));
    Ok(())
}
