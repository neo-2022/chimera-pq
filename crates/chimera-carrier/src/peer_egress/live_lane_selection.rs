use chimera_mesh::{
    MeshMultipathFlowAction, MeshMultipathFlowKey, MeshPathPlan, plan_multipath_flow,
};

use crate::peer_egress::lane_binding::{
    TransitLaneRegistration, transit_lane_registrations_from_mesh_plan,
};
use crate::peer_egress::transit_binding::TransitPathBinding;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierLaneSelectionMode {
    SinglePath,
    Multipath,
}

impl CarrierLaneSelectionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SinglePath => "single_path",
            Self::Multipath => "multipath",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CarrierLaneSelection {
    pub action: MeshMultipathFlowAction,
    pub mode: CarrierLaneSelectionMode,
    pub reason: String,
    pub selected_lane_id: Option<u16>,
    pub selected_binding: Option<TransitPathBinding>,
    pub active_binding_count: usize,
    pub rebuild_recommended: bool,
    pub rebuild_reason: String,
    pub explain: Vec<String>,
}

impl std::fmt::Debug for CarrierLaneSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CarrierLaneSelection")
            .field("action", &self.action)
            .field("mode", &self.mode)
            .field("reason", &self.reason)
            .field(
                "selected_lane_id",
                &self.selected_lane_id.map(|_| "<opaque>"),
            )
            .field(
                "selected_binding",
                &self.selected_binding.as_ref().map(|_| "<opaque>"),
            )
            .field("active_binding_count", &self.active_binding_count)
            .field("rebuild_recommended", &self.rebuild_recommended)
            .field("rebuild_reason", &self.rebuild_reason)
            .finish()
    }
}

pub fn select_carrier_lane_from_mesh_plan(
    plan: &MeshPathPlan,
    flow_key: MeshMultipathFlowKey,
) -> CarrierLaneSelection {
    let flow_plan = plan_multipath_flow(&plan.multipath_schedule, flow_key);
    if flow_plan.action != MeshMultipathFlowAction::Assigned {
        return selection(
            SelectionInput::new(
                MeshMultipathFlowAction::FailClosed,
                CarrierLaneSelectionMode::Multipath,
                &flow_plan.reason,
                None,
                flow_plan.active_binding_count,
                &flow_plan.rebuild_reason,
                flow_plan.explain,
            )
            .with_rebuild_recommended(flow_plan.rebuild_recommended),
        );
    }

    let Some(selected_lane_id) = flow_plan.selected_lane_id else {
        return selection(
            SelectionInput::new(
                MeshMultipathFlowAction::FailClosed,
                CarrierLaneSelectionMode::Multipath,
                "selected_lane_missing",
                None,
                flow_plan.active_binding_count,
                &flow_plan.rebuild_reason,
                flow_plan.explain,
            )
            .with_rebuild_recommended(flow_plan.rebuild_recommended),
        );
    };

    let registrations = match transit_lane_registrations_from_mesh_plan(plan) {
        Ok(registrations) => registrations,
        Err(error) => {
            return selection(
                SelectionInput::new(
                    MeshMultipathFlowAction::FailClosed,
                    CarrierLaneSelectionMode::Multipath,
                    &error,
                    None,
                    flow_plan.active_binding_count,
                    &flow_plan.rebuild_reason,
                    flow_plan.explain,
                )
                .with_rebuild_recommended(flow_plan.rebuild_recommended),
            );
        }
    };

    let binding = registrations
        .iter()
        .find(|registration| {
            registration.binding().lane_id().get() as usize == selected_lane_id + 1
        })
        .map(TransitLaneRegistration::binding);
    let Some(binding) = binding else {
        return selection(
            SelectionInput::new(
                MeshMultipathFlowAction::FailClosed,
                CarrierLaneSelectionMode::Multipath,
                "selected_lane_not_registered",
                None,
                flow_plan.active_binding_count,
                &flow_plan.rebuild_reason,
                flow_plan.explain,
            )
            .with_rebuild_recommended(flow_plan.rebuild_recommended),
        );
    };

    selection(
        SelectionInput::new(
            MeshMultipathFlowAction::Assigned,
            CarrierLaneSelectionMode::Multipath,
            &flow_plan.reason,
            Some(binding),
            flow_plan.active_binding_count,
            &flow_plan.rebuild_reason,
            flow_plan.explain,
        )
        .with_rebuild_recommended(flow_plan.rebuild_recommended),
    )
}

pub fn select_carrier_lane_from_registrations(
    registrations: &[TransitLaneRegistration],
    flow_key: MeshMultipathFlowKey,
) -> Result<CarrierLaneSelection, String> {
    if registrations.is_empty() {
        return Err("carrier lane selection has no registrations".to_string());
    }
    if registrations.len() == 1 {
        return Ok(selection(SelectionInput::new(
            MeshMultipathFlowAction::Assigned,
            CarrierLaneSelectionMode::SinglePath,
            "single_carrier_lane_selected",
            Some(registrations[0].binding()),
            1,
            "none",
            vec![
                "carrier_lane_selection_action=assigned".to_string(),
                "carrier_lane_selection_reason=single_carrier_lane_selected".to_string(),
                "carrier_lane_selection_mode=single_path".to_string(),
                "carrier_lane_selection_privacy=sealed_opaque_only".to_string(),
            ],
        )));
    }

    let slot = flow_key.select_slot_index(registrations.len())?;
    Ok(selection(SelectionInput::new(
        MeshMultipathFlowAction::Assigned,
        CarrierLaneSelectionMode::Multipath,
        "opaque_flow_slot_selected",
        Some(registrations[slot].binding()),
        registrations.len(),
        "none",
        vec![
            "carrier_lane_selection_action=assigned".to_string(),
            "carrier_lane_selection_reason=opaque_flow_slot_selected".to_string(),
            "carrier_lane_selection_mode=multipath".to_string(),
            format!(
                "carrier_lane_selection_active_bindings={}",
                registrations.len()
            ),
            "carrier_lane_selection_privacy=sealed_opaque_only".to_string(),
        ],
    )))
}

struct SelectionInput<'a> {
    action: MeshMultipathFlowAction,
    mode: CarrierLaneSelectionMode,
    reason: &'a str,
    selected_binding: Option<TransitPathBinding>,
    active_binding_count: usize,
    rebuild_recommended: bool,
    rebuild_reason: &'a str,
    explain: Vec<String>,
}

impl<'a> SelectionInput<'a> {
    fn new(
        action: MeshMultipathFlowAction,
        mode: CarrierLaneSelectionMode,
        reason: &'a str,
        selected_binding: Option<TransitPathBinding>,
        active_binding_count: usize,
        rebuild_reason: &'a str,
        explain: Vec<String>,
    ) -> Self {
        Self {
            action,
            mode,
            reason,
            selected_binding,
            active_binding_count,
            rebuild_recommended: false,
            rebuild_reason,
            explain,
        }
    }

    fn with_rebuild_recommended(mut self, rebuild_recommended: bool) -> Self {
        self.rebuild_recommended = rebuild_recommended;
        self
    }
}

fn selection(input: SelectionInput<'_>) -> CarrierLaneSelection {
    let selected_lane_id = input
        .selected_binding
        .map(|binding| binding.lane_id().get());
    let mut explain = input.explain;
    explain.push(format!(
        "carrier_lane_selection_action={}",
        input.action.as_str()
    ));
    explain.push(format!("carrier_lane_selection_reason={}", input.reason));
    explain.push(format!(
        "carrier_lane_selection_mode={}",
        input.mode.as_str()
    ));
    explain.push(format!(
        "carrier_lane_selection_selected_lane={}",
        if selected_lane_id.is_some() {
            "active"
        } else {
            "none"
        }
    ));
    explain.push("carrier_lane_selection_privacy=sealed_opaque_only".to_string());
    CarrierLaneSelection {
        action: input.action,
        mode: input.mode,
        reason: input.reason.to_string(),
        selected_lane_id,
        selected_binding: input.selected_binding,
        active_binding_count: input.active_binding_count,
        rebuild_recommended: input.rebuild_recommended,
        rebuild_reason: input.rebuild_reason.to_string(),
        explain,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CarrierLaneSelectionMode, select_carrier_lane_from_mesh_plan,
        select_carrier_lane_from_registrations,
    };
    use crate::peer_egress::lane_binding::TransitLaneRegistration;
    use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
    use chimera_mesh::{
        MeshDiscoveryRecord, MeshJoinRequest, MeshMultipathFlowAction, MeshMultipathFlowKey,
        MeshRuntime,
    };

    fn registration(route: u64, lane: u16) -> Result<TransitLaneRegistration, String> {
        TransitLaneRegistration::new(
            TransitPathBinding::new(TransitRouteId::new(route)?, TransitLaneId::new(lane)?),
            format!("198.51.100.{lane}:443"),
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
    fn plan_without_route_binding_fails_closed_and_recommends_runtime_wiring() -> Result<(), String>
    {
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
}
