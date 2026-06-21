use std::collections::BTreeSet;

use chimera_mesh::MeshPathPlan;

use super::transit_lane_registrations_from_mesh_plan;
use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};

pub fn render_transit_lane_registrations(
    registrations: &[TransitLaneRegistration],
) -> Result<String, String> {
    if registrations.is_empty() {
        return Err("sealed transit lane registrations are empty".to_string());
    }
    let mut output = String::from("# route_id,lane_index,endpoint\n");
    for registration in registrations {
        let route_id = registration.binding().route_id().get();
        let lane_index = registration
            .binding()
            .lane_id()
            .get()
            .checked_sub(1)
            .ok_or_else(|| "sealed transit lane binding id underflow".to_string())?;
        output.push_str(&format!(
            "{route_id},{lane_index},{}\n",
            registration.endpoint()
        ));
    }
    Ok(output)
}

pub fn render_transit_lane_registrations_from_mesh_plan(
    plan: &MeshPathPlan,
) -> Result<String, String> {
    let registrations = transit_lane_registrations_from_mesh_plan(plan)?;
    render_transit_lane_registrations(&registrations)
}

pub fn parse_transit_lane_registrations(
    input: &str,
) -> Result<Vec<TransitLaneRegistration>, String> {
    let mut registrations = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split(',').map(str::trim).collect();
        if parts.len() != 3 {
            return Err(format!(
                "sealed transit lane binding line {} must be route_id,lane_index,endpoint",
                index + 1
            ));
        }
        let route_id = parts[0]
            .parse::<u64>()
            .map_err(|_| format!("sealed transit route id invalid on line {}", index + 1))?;
        let lane_index = parts[1]
            .parse::<usize>()
            .map_err(|_| format!("sealed transit lane index invalid on line {}", index + 1))?;
        let binding = TransitPathBinding::new(
            TransitRouteId::new(route_id)?,
            TransitLaneId::from_zero_based_lane_index(lane_index)?,
        );
        if !seen.insert(binding) {
            return Err("sealed transit path binding ambiguous".to_string());
        }
        registrations.push(TransitLaneRegistration::new(binding, parts[2].to_string())?);
    }
    Ok(registrations)
}

pub fn load_transit_lane_registrations(path: &str) -> Result<Vec<TransitLaneRegistration>, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| format!("read sealed transit lane bindings failed: {error}"))?;
    let registrations = parse_transit_lane_registrations(&contents)?;
    if registrations.is_empty() {
        return Err("sealed transit lane bindings file has no registrations".to_string());
    }
    Ok(registrations)
}
