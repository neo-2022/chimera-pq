use std::collections::BTreeSet;
use std::fmt::Write;

use chimera_mesh::MeshPathPlan;

use super::format::split_comma_fields;
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
        let lane_index = registration
            .binding()
            .lane_id()
            .get()
            .checked_sub(1)
            .ok_or_else(|| "sealed transit lane binding id underflow".to_string())?;
        append_registration_row_fields(
            &mut output,
            registration.binding().route_id().get(),
            lane_index,
            registration.endpoint(),
        )?;
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
        if let Some(registration) = parse_transit_lane_registration_line(line, index, &mut seen)? {
            registrations.push(registration);
        }
    }
    Ok(registrations)
}

pub(super) fn parse_transit_lane_registration_line(
    line: &str,
    zero_based_line_index: usize,
    seen: &mut BTreeSet<TransitPathBinding>,
) -> Result<Option<TransitLaneRegistration>, String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }
    let Some(parts) = split_comma_fields::<3>(line) else {
        return Err(format!(
            "sealed transit lane binding line {} must be route_id,lane_index,endpoint",
            zero_based_line_index + 1
        ));
    };
    let route_id = parts[0].parse::<u64>().map_err(|_| {
        format!(
            "sealed transit route id invalid on line {}",
            zero_based_line_index + 1
        )
    })?;
    let lane_index = parts[1].parse::<usize>().map_err(|_| {
        format!(
            "sealed transit lane index invalid on line {}",
            zero_based_line_index + 1
        )
    })?;
    let binding = TransitPathBinding::new(
        TransitRouteId::new(route_id)?,
        TransitLaneId::from_zero_based_lane_index(lane_index)?,
    );
    if !seen.insert(binding) {
        return Err("sealed transit path binding ambiguous".to_string());
    }
    TransitLaneRegistration::new(binding, parts[2].to_string()).map(Some)
}

pub(super) fn append_registration_row_fields(
    output: &mut String,
    route_id: u64,
    lane_index: u16,
    endpoint: &str,
) -> Result<(), String> {
    if endpoint.contains(',') || endpoint.contains('\n') || endpoint.contains('\r') {
        return Err("sealed transit lane endpoint contains invalid separator".to_string());
    }
    let _ = writeln!(output, "{route_id},{lane_index},{endpoint}");
    Ok(())
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
