use chimera_mesh::{
    MeshMultipathFlowAction, MeshMultipathFlowDecision, MeshMultipathFlowKey,
    MeshMultipathLaneRole, MeshMultipathSchedule, MeshPathPlan, plan_multipath_flow,
    plan_multipath_flow_decision,
};

use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};

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
    select_carrier_lane_from_multipath_schedule(&plan.multipath_schedule, flow_key)
}

pub fn select_carrier_lane_from_multipath_schedule(
    schedule: &MeshMultipathSchedule,
    flow_key: MeshMultipathFlowKey,
) -> CarrierLaneSelection {
    let flow_plan = plan_multipath_flow(schedule, flow_key);
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

    let binding = schedule
        .carrier_lane_bindings
        .iter()
        .find(|binding| binding.lane_id == selected_lane_id)
        .and_then(|binding| {
            let route_id = TransitRouteId::new(binding.route_binding_id.get()).ok()?;
            let lane_id = TransitLaneId::from_zero_based_lane_index(binding.lane_id).ok()?;
            Some(TransitPathBinding::new(route_id, lane_id))
        });
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

pub fn select_carrier_binding_from_mesh_plan(
    plan: &MeshPathPlan,
    flow_key: MeshMultipathFlowKey,
) -> Result<TransitPathBinding, &'static str> {
    select_carrier_binding_from_multipath_schedule(&plan.multipath_schedule, flow_key)
}

pub fn select_carrier_binding_from_multipath_schedule(
    schedule: &MeshMultipathSchedule,
    flow_key: MeshMultipathFlowKey,
) -> Result<TransitPathBinding, &'static str> {
    let decision = plan_multipath_flow_decision(schedule, flow_key);
    binding_from_schedule_decision(schedule, decision)
}

pub fn select_carrier_lane_from_registrations(
    registrations: &[TransitLaneRegistration],
    flow_key: MeshMultipathFlowKey,
) -> Result<CarrierLaneSelection, String> {
    if registrations.is_empty() {
        return Err("carrier lane selection has no registrations".to_string());
    }
    if registrations_have_lane_plan(registrations) {
        return select_weighted_carrier_lane_from_registrations(registrations, flow_key);
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

fn registrations_have_lane_plan(registrations: &[TransitLaneRegistration]) -> bool {
    registrations.iter().any(|registration| {
        registration.role().is_some()
            || registration.weight_pct().is_some()
            || registration.capacity_weight_pct().is_some()
    })
}

fn select_weighted_carrier_lane_from_registrations(
    registrations: &[TransitLaneRegistration],
    flow_key: MeshMultipathFlowKey,
) -> Result<CarrierLaneSelection, String> {
    let active = registrations
        .iter()
        .filter(|registration| registration.role() == Some(MeshMultipathLaneRole::Active))
        .collect::<Vec<_>>();
    if active.is_empty() {
        return Err("carrier lane selection has no active planned registrations".to_string());
    }

    let mut total_capacity_weight_pct: u16 = 0;
    let mut active_weights = Vec::with_capacity(active.len());
    for registration in active {
        let capacity_weight_pct = registration.capacity_weight_pct().ok_or_else(|| {
            "carrier lane selection active registration capacity missing".to_string()
        })?;
        if capacity_weight_pct == 0 {
            return Err("carrier lane selection active registration capacity is zero".to_string());
        }
        total_capacity_weight_pct = total_capacity_weight_pct
            .checked_add(u16::from(capacity_weight_pct))
            .ok_or_else(|| "carrier lane selection capacity overflow".to_string())?;
        active_weights.push((registration, capacity_weight_pct));
    }
    if total_capacity_weight_pct == 0 {
        return Err("carrier lane selection active registration capacity missing".to_string());
    }

    let mut bucket = u16::try_from(flow_key.select_slot_index(total_capacity_weight_pct as usize)?)
        .map_err(|_| "carrier lane selection capacity bucket overflow".to_string())?;
    for (registration, capacity_weight_pct) in &active_weights {
        if bucket < u16::from(*capacity_weight_pct) {
            return Ok(selection(SelectionInput::new(
                MeshMultipathFlowAction::Assigned,
                if active_weights.len() == 1 {
                    CarrierLaneSelectionMode::SinglePath
                } else {
                    CarrierLaneSelectionMode::Multipath
                },
                "weighted_capacity_lane_selected",
                Some(registration.binding()),
                active_weights.len(),
                "none",
                vec![
                    "carrier_lane_selection_action=assigned".to_string(),
                    "carrier_lane_selection_reason=weighted_capacity_lane_selected".to_string(),
                    format!(
                        "carrier_lane_selection_mode={}",
                        if active_weights.len() == 1 {
                            CarrierLaneSelectionMode::SinglePath.as_str()
                        } else {
                            CarrierLaneSelectionMode::Multipath.as_str()
                        }
                    ),
                    format!(
                        "carrier_lane_selection_active_bindings={}",
                        active_weights.len()
                    ),
                    format!(
                        "carrier_lane_selection_total_capacity_weight_pct={total_capacity_weight_pct}"
                    ),
                    "carrier_lane_selection_weight_policy=capacity_weighted".to_string(),
                    "carrier_lane_selection_privacy=sealed_opaque_only".to_string(),
                ],
            )));
        }
        bucket = bucket.saturating_sub(u16::from(*capacity_weight_pct));
    }

    Err("carrier lane selection weighted bucket did not match".to_string())
}

pub fn select_carrier_binding_from_registrations(
    registrations: &[TransitLaneRegistration],
    flow_key: MeshMultipathFlowKey,
) -> Result<TransitPathBinding, &'static str> {
    if registrations.is_empty() {
        return Err("carrier lane selection has no registrations");
    }
    if registrations_have_lane_plan(registrations) {
        return select_weighted_carrier_binding_from_registrations(registrations, flow_key);
    }
    if registrations.len() == 1 {
        return Ok(registrations[0].binding());
    }

    let slot = flow_key
        .select_slot_index(registrations.len())
        .map_err(|_| "carrier lane selection slot overflow")?;
    Ok(registrations[slot].binding())
}

fn binding_from_schedule_decision(
    schedule: &MeshMultipathSchedule,
    decision: MeshMultipathFlowDecision,
) -> Result<TransitPathBinding, &'static str> {
    if decision.action != MeshMultipathFlowAction::Assigned {
        return Err(decision.reason);
    }
    let Some(selected_lane_id) = decision.selected_lane_id else {
        return Err("selected_lane_missing");
    };

    schedule
        .carrier_lane_bindings
        .iter()
        .find(|binding| binding.lane_id == selected_lane_id)
        .and_then(|binding| {
            let route_id = TransitRouteId::new(binding.route_binding_id.get()).ok()?;
            let lane_id = TransitLaneId::from_zero_based_lane_index(binding.lane_id).ok()?;
            Some(TransitPathBinding::new(route_id, lane_id))
        })
        .ok_or("selected_lane_not_registered")
}

fn select_weighted_carrier_binding_from_registrations(
    registrations: &[TransitLaneRegistration],
    flow_key: MeshMultipathFlowKey,
) -> Result<TransitPathBinding, &'static str> {
    let active = registrations
        .iter()
        .filter(|registration| registration.role() == Some(MeshMultipathLaneRole::Active))
        .collect::<Vec<_>>();
    if active.is_empty() {
        return Err("carrier lane selection has no active planned registrations");
    }

    let mut total_capacity_weight_pct: u16 = 0;
    for registration in &active {
        let Some(capacity_weight_pct) = registration.capacity_weight_pct() else {
            return Err("carrier lane selection active registration capacity missing");
        };
        if capacity_weight_pct == 0 {
            return Err("carrier lane selection active registration capacity is zero");
        }
        total_capacity_weight_pct = total_capacity_weight_pct
            .checked_add(u16::from(capacity_weight_pct))
            .ok_or("carrier lane selection capacity overflow")?;
    }
    if total_capacity_weight_pct == 0 {
        return Err("carrier lane selection active registration capacity missing");
    }

    let mut bucket = u16::try_from(
        flow_key
            .select_slot_index(total_capacity_weight_pct as usize)
            .map_err(|_| "carrier lane selection capacity bucket overflow")?,
    )
    .map_err(|_| "carrier lane selection capacity bucket overflow")?;
    for registration in active {
        let weight = u16::from(
            registration
                .capacity_weight_pct()
                .ok_or("carrier lane selection active registration capacity missing")?,
        );
        if bucket < weight {
            return Ok(registration.binding());
        }
        bucket = bucket.saturating_sub(weight);
    }

    Err("carrier lane selection weighted bucket did not match")
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
#[path = "live_lane_selection_tests.rs"]
mod tests;
