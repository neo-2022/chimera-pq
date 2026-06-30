use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use super::format::{parse_role, parse_u8_field, split_comma_fields};
use super::registration::render_transit_lane_registrations_into;
use super::snapshot_draft::TransitLanePlanSnapshotDraft;
use super::snapshot_parse::parse_transit_lane_plan_snapshot_line;
use super::snapshot_render::render_transit_lane_plan_snapshot;
use super::{TransitLaneDocument, TransitLanePlanSnapshot};
use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};

pub fn render_transit_lane_document(document: &TransitLaneDocument) -> Result<String, String> {
    if document.is_empty() {
        return Err("transit lane document is empty".to_string());
    }

    let mut output = String::with_capacity(estimate_document_render_capacity(document));
    output.push_str("# chimera_transit_lane_document=v1\n");
    if let Some(snapshot) = document.plan_snapshot.as_ref() {
        render_transit_lane_plan_snapshot(snapshot.plan(), &mut output)?;
        if snapshot_needs_separator(document) {
            output.push('\n');
        }
        if !document.registrations.is_empty() {
            render_transit_lane_document_rows_into(&mut output, &document.registrations)?;
        }
    } else if !document.registrations.is_empty() {
        render_transit_lane_registrations_into(&mut output, &document.registrations)?;
    }
    Ok(output)
}

pub fn parse_transit_lane_document(input: &str) -> Result<TransitLaneDocument, String> {
    let mut draft = TransitLanePlanSnapshotDraft::default();
    let mut row_lines = Vec::new();

    for (index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        if parse_transit_lane_plan_snapshot_line(line, &mut draft)? {
            continue;
        }
        row_lines.push((index, line));
    }

    let plan_snapshot = draft.finish()?;
    let registrations = if plan_snapshot.is_some() {
        parse_transit_lane_document_row_lines(&row_lines)?
    } else {
        parse_transit_lane_registration_row_lines(&row_lines)?
    };
    if let Some(plan) = plan_snapshot.as_ref() {
        validate_transit_lane_document_rows_match_plan(&registrations, plan)?;
    }
    Ok(TransitLaneDocument::new(
        registrations,
        plan_snapshot.map(TransitLanePlanSnapshot::new),
    ))
}

fn estimate_document_render_capacity(document: &TransitLaneDocument) -> usize {
    let mut capacity = "# chimera_transit_lane_document=v1\n".len();
    if let Some(snapshot) = document.plan_snapshot.as_ref() {
        let plan = snapshot.plan();
        capacity = capacity
            .saturating_add(1_024)
            .saturating_add(plan.selected_peers.len().saturating_mul(96))
            .saturating_add(
                plan.multipath_schedule
                    .carrier_lane_bindings
                    .len()
                    .saturating_mul(112),
            )
            .saturating_add(plan.explain.iter().map(|line| line.len() + 32).sum());
    }
    if !document.registrations.is_empty() {
        capacity = capacity
            .saturating_add(
                "# route_id,lane_index,endpoint,role,weight_pct,capacity_weight_pct\n".len(),
            )
            .saturating_add(
                document
                    .registrations
                    .iter()
                    .map(|registration| registration.endpoint().len() + 32)
                    .sum(),
            );
    }
    capacity
}

fn render_transit_lane_document_rows_into(
    output: &mut String,
    registrations: &[TransitLaneRegistration],
) -> Result<(), String> {
    if registrations.is_empty() {
        return Err("sealed transit lane document rows are empty".to_string());
    }
    output.push_str("# route_id,lane_index,endpoint,role,weight_pct,capacity_weight_pct\n");
    for registration in registrations {
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
        append_document_row_fields(
            output,
            registration.binding().route_id().get(),
            lane_index,
            registration.endpoint(),
            role.as_str(),
            weight_pct,
            capacity_weight_pct,
        )?;
    }
    Ok(())
}

fn append_document_row_fields(
    output: &mut String,
    route_id: u64,
    lane_index: u16,
    endpoint: &str,
    role: &str,
    weight_pct: u8,
    capacity_weight_pct: u8,
) -> Result<(), String> {
    if endpoint.contains(',') || endpoint.contains('\n') || endpoint.contains('\r') {
        return Err("sealed transit lane endpoint contains invalid separator".to_string());
    }
    let _ = writeln!(
        output,
        "{route_id},{lane_index},{endpoint},{role},{weight_pct},{capacity_weight_pct}"
    );
    Ok(())
}

fn parse_transit_lane_document_row_lines(
    rows: &[(usize, &str)],
) -> Result<Vec<TransitLaneRegistration>, String> {
    let mut registrations = Vec::with_capacity(rows.len());
    let mut seen = BTreeSet::new();
    for (index, line) in rows {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let registration = parse_transit_lane_document_row(line, *index, &mut seen)?;
        registrations.push(registration);
    }
    Ok(registrations)
}

fn parse_transit_lane_registration_row_lines(
    rows: &[(usize, &str)],
) -> Result<Vec<TransitLaneRegistration>, String> {
    let mut registrations = Vec::with_capacity(rows.len());
    let mut seen = BTreeSet::new();
    for (index, row) in rows {
        let row = row.trim();
        if row.is_empty() || row.starts_with('#') {
            continue;
        }
        let registration = parse_transit_lane_registration_row(row, *index, &mut seen)?;
        registrations.push(registration);
    }
    Ok(registrations)
}

fn parse_transit_lane_document_row(
    line: &str,
    zero_based_line_index: usize,
    seen: &mut BTreeSet<TransitPathBinding>,
) -> Result<TransitLaneRegistration, String> {
    let Some(parts) = split_comma_fields::<6>(line) else {
        return Err(format!(
            "sealed transit lane document row {} must be route_id,lane_index,endpoint[,role,weight_pct,capacity_weight_pct]",
            zero_based_line_index + 1
        ));
    };
    let binding = parse_row_binding(parts[0], parts[1], zero_based_line_index, seen)?;
    TransitLaneRegistration::new_with_lane_plan(
        binding,
        parts[2],
        Some(parse_role(parts[3])?),
        Some(parse_u8_field(
            parts[4],
            "sealed transit lane document row",
        )?),
        Some(parse_u8_field(
            parts[5],
            "sealed transit lane document row",
        )?),
    )
}

fn parse_row_binding(
    route_id: &str,
    lane_index: &str,
    zero_based_line_index: usize,
    seen: &mut BTreeSet<TransitPathBinding>,
) -> Result<TransitPathBinding, String> {
    let route_id = route_id.parse::<u64>().map_err(|_| {
        format!(
            "sealed transit route id invalid on line {}",
            zero_based_line_index + 1
        )
    })?;
    let lane_index = lane_index.parse::<usize>().map_err(|_| {
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
    Ok(binding)
}

fn parse_transit_lane_registration_row(
    line: &str,
    zero_based_line_index: usize,
    seen: &mut BTreeSet<TransitPathBinding>,
) -> Result<TransitLaneRegistration, String> {
    let Some(parts) = split_comma_fields::<3>(line) else {
        return Err(format!(
            "sealed transit lane binding line {} must be route_id,lane_index,endpoint",
            zero_based_line_index + 1
        ));
    };
    let binding = parse_row_binding(parts[0], parts[1], zero_based_line_index, seen)?;
    TransitLaneRegistration::new(binding, parts[2])
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
        if registration.role() != Some(binding.role) {
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
