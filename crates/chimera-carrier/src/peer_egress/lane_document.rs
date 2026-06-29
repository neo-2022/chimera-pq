use std::fmt;

use chimera_mesh::{MeshCarrierLaneBinding, MeshPathPlan};

use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};

mod document;
mod format;
mod registration;
mod snapshot_draft;
mod snapshot_parse;
mod snapshot_render;
mod writer;

#[cfg(test)]
mod tests;

pub use document::{
    load_transit_lane_document, parse_transit_lane_document, render_transit_lane_document,
};
pub use registration::{
    load_transit_lane_registrations, parse_transit_lane_registrations,
    render_transit_lane_registrations, render_transit_lane_registrations_from_mesh_plan,
};
pub use writer::{
    write_transit_lane_document_from_mesh_plan, write_transit_lane_registrations_from_mesh_plan,
};

#[derive(Clone, PartialEq, Eq)]
pub struct TransitLanePlanSnapshot {
    plan: MeshPathPlan,
}

impl TransitLanePlanSnapshot {
    pub fn new(plan: MeshPathPlan) -> Self {
        Self { plan }
    }

    pub fn plan(&self) -> &MeshPathPlan {
        &self.plan
    }

    pub fn mesh_path_plan(&self) -> &MeshPathPlan {
        &self.plan
    }

    pub fn into_mesh_path_plan(self) -> MeshPathPlan {
        self.plan
    }
}

impl fmt::Debug for TransitLanePlanSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransitLanePlanSnapshot")
            .field("plan", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct TransitLaneDocument {
    registrations: Vec<TransitLaneRegistration>,
    plan_snapshot: Option<TransitLanePlanSnapshot>,
}

impl TransitLaneDocument {
    pub fn new(
        registrations: Vec<TransitLaneRegistration>,
        plan_snapshot: Option<TransitLanePlanSnapshot>,
    ) -> Self {
        Self {
            registrations,
            plan_snapshot,
        }
    }

    pub fn registrations(&self) -> &[TransitLaneRegistration] {
        &self.registrations
    }

    pub fn mesh_path_plan(&self) -> Result<Option<MeshPathPlan>, String> {
        Ok(self
            .plan_snapshot
            .as_ref()
            .map(|snapshot| snapshot.plan.clone()))
    }

    pub fn mesh_path_plan_ref(&self) -> Option<&MeshPathPlan> {
        self.plan_snapshot
            .as_ref()
            .map(TransitLanePlanSnapshot::plan)
    }

    pub fn require_mesh_path_plan(&self) -> Result<MeshPathPlan, String> {
        self.mesh_path_plan()?.ok_or_else(|| {
            "live sealed transit lane document requires a mesh plan snapshot".to_string()
        })
    }

    pub fn require_mesh_path_plan_ref(&self) -> Result<&MeshPathPlan, String> {
        self.mesh_path_plan_ref().ok_or_else(|| {
            "live sealed transit lane document requires a mesh plan snapshot".to_string()
        })
    }

    pub fn is_empty(&self) -> bool {
        self.registrations.is_empty() && self.plan_snapshot.is_none()
    }
}

impl fmt::Debug for TransitLaneDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransitLaneDocument")
            .field("registrations", &self.registrations.len())
            .field(
                "plan_snapshot",
                &self.plan_snapshot.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

fn transit_path_binding_from_mesh_lane(
    binding: &MeshCarrierLaneBinding,
) -> Result<TransitPathBinding, String> {
    Ok(TransitPathBinding::new(
        TransitRouteId::new(binding.route_binding_id.get())?,
        TransitLaneId::from_zero_based_lane_index(binding.lane_id)?,
    ))
}

fn transit_lane_registration_from_mesh_lane(
    binding: &MeshCarrierLaneBinding,
) -> Result<TransitLaneRegistration, String> {
    TransitLaneRegistration::new_with_lane_plan(
        transit_path_binding_from_mesh_lane(binding)?,
        binding.carrier_endpoint.clone(),
        Some(binding.role.clone()),
        Some(binding.weight_pct),
        Some(binding.capacity_weight_pct),
    )
}

pub fn transit_lane_registrations_from_mesh_plan(
    plan: &MeshPathPlan,
) -> Result<Vec<TransitLaneRegistration>, String> {
    let bindings = &plan.multipath_schedule.carrier_lane_bindings;
    if bindings.is_empty() {
        return Err("mesh path plan has no carrier lane bindings".to_string());
    }
    bindings
        .iter()
        .map(transit_lane_registration_from_mesh_lane)
        .collect()
}

pub fn transit_lane_document_from_mesh_plan(
    plan: &MeshPathPlan,
) -> Result<TransitLaneDocument, String> {
    Ok(TransitLaneDocument::new(
        transit_lane_registrations_from_mesh_plan(plan)?,
        Some(TransitLanePlanSnapshot::new(plan.clone())),
    ))
}
