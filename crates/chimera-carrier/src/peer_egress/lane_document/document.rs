use std::collections::{BTreeMap, BTreeSet};

use super::format::{parse_role, parse_u8_field};
use super::registration::{parse_transit_lane_registrations, render_transit_lane_registrations};
use super::snapshot_draft::TransitLanePlanSnapshotDraft;
use super::snapshot_parse::parse_transit_lane_plan_snapshot_line;
use super::snapshot_render::render_transit_lane_plan_snapshot;
use super::{TransitLaneDocument, TransitLanePlanSnapshot};
use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};

fn render_transit_lane_document_rows(
    registrations: &[TransitLaneRegistration],
) -> Result<String, String> {
    if registrations.is_empty() {
        return Err("sealed transit lane document rows are empty".to_string());
    }
    let mut output =
        String::from("# route_id,lane_index,endpoint,role,weight_pct,capacity_weight_pct\n");
    for registration in registrations {
        let route_id = registration.binding().route_id().get();
        let lane_index = registration
            .binding()
            .lane_id()
            .get()
            .checked_sub(1)
            .ok_or_else(|| "sealed transit lane binding id underflow".to_string())?;
        let role = registration
            .role()
            .ok_or_else(|| "sealed transit lane document row missing role".to_string())?;
        let weight_pct = registration
            .weight_pct()
            .ok_or_else(|| "sealed transit lane document row missing weight pct".to_string())?;
        let capacity_weight_pct = registration.capacity_weight_pct().ok_or_else(|| {
            "sealed transit lane document row missing capacity weight pct".to_string()
        })?;
        output.push_str(&format!(
            "{route_id},{lane_index},{},{},{},{}\n",
            registration.endpoint(),
            role.as_str(),
            weight_pct,
            capacity_weight_pct
        ));
    }
    Ok(output)
}

fn parse_transit_lane_document_rows(input: &str) -> Result<Vec<TransitLaneRegistration>, String> {
    let mut registrations = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split(',').map(str::trim).collect();
        let registration = match parts.len() {
            3 => {
                let route_id = parts[0].parse::<u64>().map_err(|_| {
                    format!("sealed transit route id invalid on line {}", index + 1)
                })?;
                let lane_index = parts[1].parse::<usize>().map_err(|_| {
                    format!("sealed transit lane index invalid on line {}", index + 1)
                })?;
                let binding = TransitPathBinding::new(
                    TransitRouteId::new(route_id)?,
                    TransitLaneId::from_zero_based_lane_index(lane_index)?,
                );
                if !seen.insert(binding) {
                    return Err("sealed transit path binding ambiguous".to_string());
                }
                TransitLaneRegistration::new(binding, parts[2].to_string())?
            }
            6 => {
                let route_id = parts[0].parse::<u64>().map_err(|_| {
                    format!("sealed transit route id invalid on line {}", index + 1)
                })?;
                let lane_index = parts[1].parse::<usize>().map_err(|_| {
                    format!("sealed transit lane index invalid on line {}", index + 1)
                })?;
                let binding = TransitPathBinding::new(
                    TransitRouteId::new(route_id)?,
                    TransitLaneId::from_zero_based_lane_index(lane_index)?,
                );
                if !seen.insert(binding) {
                    return Err("sealed transit path binding ambiguous".to_string());
                }
                TransitLaneRegistration::new_with_lane_plan(
                    binding,
                    parts[2].to_string(),
                    Some(parse_role(parts[3])?),
                    Some(parse_u8_field(
                        parts[4],
                        "sealed transit lane document row",
                    )?),
                    Some(parse_u8_field(
                        parts[5],
                        "sealed transit lane document row",
                    )?),
                )?
            }
            _ => {
                return Err(format!(
                    "sealed transit lane document row {} must be route_id,lane_index,endpoint[,role,weight_pct,capacity_weight_pct]",
                    index + 1
                ));
            }
        };
        registrations.push(registration);
    }
    Ok(registrations)
}

pub fn render_transit_lane_document(document: &TransitLaneDocument) -> Result<String, String> {
    if document.is_empty() {
        return Err("transit lane document is empty".to_string());
    }

    let mut output = String::from("# chimera_transit_lane_document=v1\n");
    if let Some(snapshot) = document.plan_snapshot.as_ref() {
        render_transit_lane_plan_snapshot(snapshot.plan(), &mut output)?;
        if snapshot_needs_separator(document) {
            output.push('\n');
        }
        if !document.registrations.is_empty() {
            output.push_str(&render_transit_lane_document_rows(&document.registrations)?);
        }
    } else if !document.registrations.is_empty() {
        output.push_str(&render_transit_lane_registrations(&document.registrations)?);
    }
    Ok(output)
}

pub fn parse_transit_lane_document(input: &str) -> Result<TransitLaneDocument, String> {
    let mut draft = TransitLanePlanSnapshotDraft::default();
    let mut row_lines = Vec::new();

    for raw_line in input.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        if parse_transit_lane_plan_snapshot_line(line, &mut draft)? {
            continue;
        }
        row_lines.push(line.to_string());
    }

    let plan_snapshot = draft.finish()?;
    let registrations = if plan_snapshot.is_some() {
        parse_transit_lane_document_rows(&row_lines.join("\n"))?
    } else {
        parse_transit_lane_registrations(&row_lines.join("\n"))?
    };
    if let Some(plan) = plan_snapshot.as_ref() {
        validate_transit_lane_document_rows_match_plan(&registrations, plan)?;
    }
    Ok(TransitLaneDocument::new(
        registrations,
        plan_snapshot.map(TransitLanePlanSnapshot::new),
    ))
}

pub fn load_transit_lane_document(path: &str) -> Result<TransitLaneDocument, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| format!("read sealed transit lane document failed: {error}"))?;
    let document = parse_transit_lane_document(&contents)?;
    if document.registrations.is_empty() {
        return Err("sealed transit lane document has no registrations".to_string());
    }
    Ok(document)
}

fn snapshot_needs_separator(document: &TransitLaneDocument) -> bool {
    document.plan_snapshot.is_some() && !document.registrations.is_empty()
}

fn validate_transit_lane_document_rows_match_plan(
    registrations: &[TransitLaneRegistration],
    plan: &chimera_mesh::MeshPathPlan,
) -> Result<(), String> {
    let bindings = &plan.multipath_schedule.carrier_lane_bindings;
    if registrations.len() != bindings.len() {
        return Err("sealed transit lane document row count mismatches plan snapshot".to_string());
    }

    let mut by_binding = BTreeMap::new();
    for registration in registrations {
        if by_binding
            .insert(registration.binding(), registration)
            .is_some()
        {
            return Err("sealed transit lane document duplicate row binding".to_string());
        }
    }

    for binding in bindings {
        let path_binding = super::transit_path_binding_from_mesh_lane(binding)?;
        let registration = by_binding
            .get(&path_binding)
            .ok_or_else(|| "sealed transit lane document missing plan binding row".to_string())?;
        if registration.endpoint() != binding.carrier_endpoint {
            return Err(
                "sealed transit lane document endpoint mismatches plan snapshot".to_string(),
            );
        }
        if registration.role() != Some(binding.role.clone()) {
            return Err("sealed transit lane document role mismatches plan snapshot".to_string());
        }
        if registration.weight_pct() != Some(binding.weight_pct) {
            return Err("sealed transit lane document weight mismatches plan snapshot".to_string());
        }
        if registration.capacity_weight_pct() != Some(binding.capacity_weight_pct) {
            return Err(
                "sealed transit lane document capacity mismatches plan snapshot".to_string(),
            );
        }
    }
    Ok(())
}
