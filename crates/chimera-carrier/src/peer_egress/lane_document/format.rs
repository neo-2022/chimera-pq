use chimera_mesh::{MeshJoinMode, MeshMultipathLaneRole, MeshMultipathMode, MeshRouteBindingId};

pub(super) const PLAN_SNAPSHOT_VERSION: &str = "v1";
pub(super) const PLAN_KEY_PREFIX: &str = "chimera_plan_";
pub(super) const PLAN_KEY_VERSION: &str = "chimera_plan_snapshot";
pub(super) const PLAN_KEY_NAMESPACE: &str = "chimera_plan_namespace";
pub(super) const PLAN_KEY_JOIN_MODE: &str = "chimera_plan_join_mode";
pub(super) const PLAN_KEY_SELECTED_PEER: &str = "chimera_plan_selected_peer";
pub(super) const PLAN_KEY_MODE: &str = "chimera_plan_mode";
pub(super) const PLAN_KEY_ROUTE_BINDING_ID: &str = "chimera_plan_route_binding_id";
pub(super) const PLAN_KEY_LANE_REQUESTED: &str =
    "chimera_plan_lane_admission_requested_active_lanes";
pub(super) const PLAN_KEY_LANE_ADMITTED: &str = "chimera_plan_lane_admission_admitted_active_lanes";
pub(super) const PLAN_KEY_LANE_REJECTED: &str = "chimera_plan_lane_admission_rejected_active_lanes";
pub(super) const PLAN_KEY_LANE_CAPACITY_STATUS: &str =
    "chimera_plan_lane_admission_capacity_status";
pub(super) const PLAN_KEY_LOCAL_RESERVE_PCT: &str = "chimera_plan_local_traffic_reserve_pct";
pub(super) const PLAN_KEY_TRANSIT_BUDGET_PCT: &str = "chimera_plan_transit_capacity_budget_pct";
pub(super) const PLAN_KEY_DEMAND_POLICY: &str = "chimera_plan_demand_policy";
pub(super) const PLAN_KEY_DEMAND_POLICY_SOURCE: &str = "chimera_plan_demand_policy_source";
pub(super) const PLAN_KEY_DEMAND_REQUESTED: &str = "chimera_plan_demand_requested_active_lanes";
pub(super) const PLAN_KEY_DEMAND_PLANNED: &str = "chimera_plan_demand_planned_active_lanes";
pub(super) const PLAN_KEY_DEMAND_ADMITTED_CAPACITY_PCT: &str =
    "chimera_plan_demand_admitted_lane_capacity_pct";
pub(super) const PLAN_KEY_DEMAND_UNMET: &str = "chimera_plan_demand_unmet_lanes";
pub(super) const PLAN_KEY_DEMAND_STATUS: &str = "chimera_plan_demand_status";
pub(super) const PLAN_KEY_DEMAND_REBUILD_RECOMMENDED: &str =
    "chimera_plan_demand_rebuild_recommended";
pub(super) const PLAN_KEY_FAIRNESS_POLICY: &str = "chimera_plan_fairness_policy";
pub(super) const PLAN_KEY_EXECUTION_STATUS: &str = "chimera_plan_execution_status";
pub(super) const PLAN_KEY_TRANSIT_PAYLOAD_POLICY: &str = "chimera_plan_transit_payload_policy";
pub(super) const PLAN_KEY_PLANNER_REBUILD_REASON: &str = "chimera_plan_planner_rebuild_reason";
pub(super) const PLAN_KEY_CARRIER_BINDING: &str = "chimera_plan_carrier_binding";
pub(super) const PLAN_KEY_EXPLAIN: &str = "chimera_plan_explain";

const EXPECTED_LOCAL_TRAFFIC_RESERVE_PCT: u8 = 10;
const EXPECTED_TRANSIT_CAPACITY_BUDGET_PCT: u8 = 90;
const EXPECTED_TRANSIT_PAYLOAD_POLICY: &str = "sealed_opaque_only";

pub(super) fn push_plan_comment(output: &mut String, key: &str, value: &str) -> Result<(), String> {
    cleaned_comment_field(value, key)?;
    output.push_str("# ");
    output.push_str(key);
    output.push('=');
    output.push_str(value);
    output.push('\n');
    Ok(())
}

pub(super) fn push_plan_tab_comment(
    output: &mut String,
    key: &str,
    fields: &[String],
) -> Result<(), String> {
    for field in fields {
        cleaned_comment_field(field, key)?;
    }
    output.push_str("# ");
    output.push_str(key);
    for field in fields {
        output.push('\t');
        output.push_str(field);
    }
    output.push('\n');
    Ok(())
}

pub(super) fn cleaned_comment_field<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    if value.contains('\n') || value.contains('\r') || value.contains('\t') {
        return Err(format!("{label} contains control whitespace"));
    }
    Ok(value)
}

pub(super) fn parse_u64_field(value: &str, label: &str) -> Result<u64, String> {
    value.parse::<u64>().map_err(|_| format!("{label} invalid"))
}

pub(super) fn split_tab_fields<'a>(
    value: &'a str,
    expected_fields: usize,
    label: &str,
) -> Result<Vec<&'a str>, String> {
    let parts: Vec<&str> = value.split('\t').collect();
    if parts.len() != expected_fields {
        return Err(format!(
            "{label} line must contain exactly {expected_fields} tab-separated fields"
        ));
    }
    Ok(parts)
}

pub(super) fn parse_u8_field(value: &str, label: &str) -> Result<u8, String> {
    value.parse::<u8>().map_err(|_| format!("{label} invalid"))
}

pub(super) fn parse_u16_field(value: &str, label: &str) -> Result<u16, String> {
    value.parse::<u16>().map_err(|_| format!("{label} invalid"))
}

pub(super) fn parse_usize_field(value: &str, label: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("{label} invalid"))
}

pub(super) fn parse_i32_field(value: &str, label: &str) -> Result<i32, String> {
    value.parse::<i32>().map_err(|_| format!("{label} invalid"))
}

pub(super) fn parse_bool_field(value: &str, label: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("{label} invalid")),
    }
}

pub(super) fn parse_optional_route_binding_id(
    value: &str,
) -> Result<Option<MeshRouteBindingId>, String> {
    if value == "none" {
        return Ok(None);
    }
    Ok(Some(MeshRouteBindingId::new(
        value
            .parse::<u64>()
            .map_err(|_| "transit plan snapshot route binding id invalid".to_string())?,
    )?))
}

pub(super) fn join_mode_to_str(mode: &MeshJoinMode) -> &'static str {
    match mode {
        MeshJoinMode::InvitationOnly => "invitation_only",
        MeshJoinMode::PublicDiscovery => "public_discovery",
    }
}

pub(super) fn parse_join_mode(value: &str) -> Result<MeshJoinMode, String> {
    match value {
        "invitation_only" => Ok(MeshJoinMode::InvitationOnly),
        "public_discovery" => Ok(MeshJoinMode::PublicDiscovery),
        _ => Err("transit plan snapshot join mode invalid".to_string()),
    }
}

pub(super) fn parse_multipath_mode(value: &str) -> Result<MeshMultipathMode, String> {
    match value {
        "off" => Ok(MeshMultipathMode::Off),
        "standby_only" => Ok(MeshMultipathMode::StandbyOnly),
        "flow_shard" => Ok(MeshMultipathMode::FlowShard),
        "aggregate_buffered" => Ok(MeshMultipathMode::AggregateBuffered),
        _ => Err("transit plan snapshot multipath mode invalid".to_string()),
    }
}

pub(super) fn validate_snapshot_capacity_contract(
    local_reserve_pct: u8,
    transit_budget_pct: u8,
    active_capacity_sum_pct: u16,
    transit_payload_policy: &str,
) -> Result<(), String> {
    if transit_payload_policy != EXPECTED_TRANSIT_PAYLOAD_POLICY {
        return Err("transit plan snapshot payload policy must be sealed_opaque_only".to_string());
    }
    if local_reserve_pct != EXPECTED_LOCAL_TRAFFIC_RESERVE_PCT {
        return Err("transit plan snapshot local reserve pct mismatch".to_string());
    }
    if transit_budget_pct != EXPECTED_TRANSIT_CAPACITY_BUDGET_PCT {
        return Err("transit plan snapshot transit budget pct mismatch".to_string());
    }
    if local_reserve_pct.saturating_add(transit_budget_pct) != 100 {
        return Err("transit plan snapshot reserve and budget sum mismatch".to_string());
    }
    if active_capacity_sum_pct > u16::from(transit_budget_pct) {
        return Err("transit plan snapshot active capacity exceeds transit budget".to_string());
    }
    Ok(())
}

pub(super) fn parse_role(value: &str) -> Result<MeshMultipathLaneRole, String> {
    match value {
        "active" => Ok(MeshMultipathLaneRole::Active),
        "standby" => Ok(MeshMultipathLaneRole::Standby),
        _ => Err("transit plan snapshot lane role invalid".to_string()),
    }
}
