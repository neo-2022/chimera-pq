use chimera_mesh::{MeshCarrierLaneBinding, MeshPeerState, MeshRouteBindingId};

use super::format::{
    PLAN_KEY_CARRIER_BINDING, PLAN_KEY_DEMAND_ADMITTED_CAPACITY_PCT, PLAN_KEY_DEMAND_PLANNED,
    PLAN_KEY_DEMAND_POLICY, PLAN_KEY_DEMAND_POLICY_SOURCE, PLAN_KEY_DEMAND_REBUILD_RECOMMENDED,
    PLAN_KEY_DEMAND_REQUESTED, PLAN_KEY_DEMAND_STATUS, PLAN_KEY_DEMAND_UNMET,
    PLAN_KEY_EXECUTION_STATUS, PLAN_KEY_EXPLAIN, PLAN_KEY_FAIRNESS_POLICY, PLAN_KEY_JOIN_MODE,
    PLAN_KEY_LANE_ADMITTED, PLAN_KEY_LANE_CAPACITY_STATUS, PLAN_KEY_LANE_REJECTED,
    PLAN_KEY_LANE_REQUESTED, PLAN_KEY_LOCAL_RESERVE_PCT, PLAN_KEY_MODE, PLAN_KEY_NAMESPACE,
    PLAN_KEY_PLANNER_REBUILD_REASON, PLAN_KEY_PREFIX, PLAN_KEY_ROUTE_BINDING_ID,
    PLAN_KEY_SELECTED_PEER, PLAN_KEY_TRANSIT_BUDGET_PCT, PLAN_KEY_TRANSIT_PAYLOAD_POLICY,
    parse_bool_field, parse_i32_field, parse_join_mode, parse_multipath_mode,
    parse_optional_route_binding_id, parse_u8_field, parse_u16_field, parse_u64_field,
    parse_usize_field, split_tab_fields,
};

use super::snapshot_draft::TransitLanePlanSnapshotDraft;

pub(super) fn parse_transit_lane_plan_snapshot_line(
    line: &str,
    draft: &mut TransitLanePlanSnapshotDraft,
) -> Result<bool, String> {
    let Some(comment) = line.strip_prefix("# ") else {
        return Ok(false);
    };
    if comment == "chimera_plan_snapshot=v1" {
        draft.version_seen = true;
        draft.snapshot_seen = true;
        return Ok(true);
    }
    if let Some(value) = strip_plan_tab(comment, PLAN_KEY_SELECTED_PEER) {
        draft.snapshot_seen = true;
        let parts = split_tab_fields::<7>(value, PLAN_KEY_SELECTED_PEER)?;
        let index = parse_usize_field(parts[0], PLAN_KEY_SELECTED_PEER)?;
        let peer = MeshPeerState {
            node_id: parts[1].to_string(),
            endpoint: parts[2].to_string(),
            region: parts[3].to_string(),
            load_score: parse_u8_field(parts[4], PLAN_KEY_SELECTED_PEER)?,
            reliability_score: parse_u8_field(parts[5], PLAN_KEY_SELECTED_PEER)?,
            latency_ms: None,
            throughput_mbps: None,
            selection_score: parse_i32_field(parts[6], PLAN_KEY_SELECTED_PEER)?,
        };
        if draft.selected_peers.insert(index, peer).is_some() {
            return Err("transit plan snapshot selected peer duplicate index".to_string());
        }
        return Ok(true);
    }
    if let Some(value) = strip_plan_tab(comment, PLAN_KEY_CARRIER_BINDING) {
        draft.snapshot_seen = true;
        let parts = split_tab_fields::<7>(value, PLAN_KEY_CARRIER_BINDING)?;
        let route_id = parse_u64_field(parts[0], PLAN_KEY_CARRIER_BINDING)?;
        let lane_id = parse_usize_field(parts[1], PLAN_KEY_CARRIER_BINDING)?;
        let role = super::format::parse_role(parts[4])?;
        let binding = MeshCarrierLaneBinding {
            route_binding_id: MeshRouteBindingId::new(route_id)?,
            lane_id,
            peer_node_id: parts[2].to_string(),
            carrier_endpoint: parts[3].to_string(),
            role,
            weight_pct: parse_u8_field(parts[5], PLAN_KEY_CARRIER_BINDING)?,
            capacity_weight_pct: parse_u8_field(parts[6], PLAN_KEY_CARRIER_BINDING)?,
        };
        if draft.carrier_bindings.insert(lane_id, binding).is_some() {
            return Err("transit plan snapshot carrier binding duplicate lane".to_string());
        }
        return Ok(true);
    }
    if let Some(value) = strip_plan_tab(comment, PLAN_KEY_EXPLAIN) {
        draft.snapshot_seen = true;
        let parts = split_tab_fields::<2>(value, PLAN_KEY_EXPLAIN)?;
        let index = parse_usize_field(parts[0], PLAN_KEY_EXPLAIN)?;
        if draft.explain.insert(index, parts[1].to_string()).is_some() {
            return Err("transit plan snapshot explain duplicate index".to_string());
        }
        return Ok(true);
    }
    if let Some((key, value)) = split_plan_value(comment) {
        return parse_plan_value(key, value, draft);
    }

    if comment.starts_with(PLAN_KEY_PREFIX) {
        let key = split_plan_key(comment);
        return Err(format!("unknown transit plan snapshot key: {key}"));
    }

    Ok(false)
}

fn split_plan_value(comment: &str) -> Option<(&str, &str)> {
    let (key, value) = comment.split_once('=')?;
    if key.starts_with(PLAN_KEY_PREFIX) {
        Some((key, value))
    } else {
        None
    }
}

fn split_plan_key(comment: &str) -> &str {
    let boundary = comment
        .find(|byte: char| byte == '=' || byte.is_ascii_whitespace())
        .unwrap_or(comment.len());
    &comment[..boundary]
}

fn parse_plan_value(
    key: &str,
    value: &str,
    draft: &mut TransitLanePlanSnapshotDraft,
) -> Result<bool, String> {
    draft.snapshot_seen = true;
    match key {
        PLAN_KEY_NAMESPACE => draft.namespace = Some(value.to_string()),
        PLAN_KEY_JOIN_MODE => draft.join_mode = Some(parse_join_mode(value)?),
        PLAN_KEY_MODE => draft.mode = Some(parse_multipath_mode(value)?),
        PLAN_KEY_ROUTE_BINDING_ID => {
            draft.route_binding_id = parse_optional_route_binding_id(value)?
        }
        PLAN_KEY_LANE_REQUESTED => {
            draft.lane_requested = Some(parse_usize_field(value, PLAN_KEY_LANE_REQUESTED)?)
        }
        PLAN_KEY_LANE_ADMITTED => {
            draft.lane_admitted = Some(parse_usize_field(value, PLAN_KEY_LANE_ADMITTED)?)
        }
        PLAN_KEY_LANE_REJECTED => {
            draft.lane_rejected = Some(parse_usize_field(value, PLAN_KEY_LANE_REJECTED)?)
        }
        PLAN_KEY_LANE_CAPACITY_STATUS => draft.lane_capacity_status = Some(value.to_string()),
        PLAN_KEY_LOCAL_RESERVE_PCT => {
            draft.local_reserve_pct = Some(parse_u8_field(value, PLAN_KEY_LOCAL_RESERVE_PCT)?)
        }
        PLAN_KEY_TRANSIT_BUDGET_PCT => {
            draft.transit_budget_pct = Some(parse_u8_field(value, PLAN_KEY_TRANSIT_BUDGET_PCT)?)
        }
        PLAN_KEY_DEMAND_POLICY => draft.demand_policy = Some(value.to_string()),
        PLAN_KEY_DEMAND_POLICY_SOURCE => draft.demand_policy_source = Some(value.to_string()),
        PLAN_KEY_DEMAND_REQUESTED => {
            draft.demand_requested = Some(parse_usize_field(value, PLAN_KEY_DEMAND_REQUESTED)?)
        }
        PLAN_KEY_DEMAND_PLANNED => {
            draft.demand_planned = Some(parse_usize_field(value, PLAN_KEY_DEMAND_PLANNED)?)
        }
        PLAN_KEY_DEMAND_ADMITTED_CAPACITY_PCT => {
            draft.demand_admitted_capacity_pct = Some(parse_u8_field(
                value,
                PLAN_KEY_DEMAND_ADMITTED_CAPACITY_PCT,
            )?)
        }
        PLAN_KEY_DEMAND_UNMET => {
            draft.demand_unmet = Some(parse_usize_field(value, PLAN_KEY_DEMAND_UNMET)?)
        }
        PLAN_KEY_DEMAND_STATUS => draft.demand_status = Some(value.to_string()),
        PLAN_KEY_DEMAND_REBUILD_RECOMMENDED => {
            draft.demand_rebuild_recommended = Some(parse_bool_field(
                value,
                PLAN_KEY_DEMAND_REBUILD_RECOMMENDED,
            )?)
        }
        PLAN_KEY_FAIRNESS_POLICY => draft.fairness_policy = Some(value.to_string()),
        PLAN_KEY_EXECUTION_STATUS => draft.execution_status = Some(value.to_string()),
        PLAN_KEY_TRANSIT_PAYLOAD_POLICY => draft.transit_payload_policy = Some(value.to_string()),
        PLAN_KEY_PLANNER_REBUILD_REASON => draft.planner_rebuild_reason = Some(value.to_string()),
        "chimera_plan_active_lane_count" => {
            draft.active_lane_count =
                Some(parse_usize_field(value, "chimera_plan_active_lane_count")?)
        }
        "chimera_plan_standby_lane_count" => {
            draft.standby_lane_count =
                Some(parse_usize_field(value, "chimera_plan_standby_lane_count")?)
        }
        "chimera_plan_active_weight_sum_pct" => {
            draft.active_weight_sum_pct = Some(parse_u16_field(
                value,
                "chimera_plan_active_weight_sum_pct",
            )?)
        }
        "chimera_plan_active_capacity_sum_pct" => {
            draft.active_capacity_sum_pct = Some(parse_u16_field(
                value,
                "chimera_plan_active_capacity_sum_pct",
            )?)
        }
        _ => return Err(format!("unknown transit plan snapshot key: {key}")),
    }
    Ok(true)
}

fn strip_plan_tab<'a>(comment: &'a str, key: &str) -> Option<&'a str> {
    let value = comment.strip_prefix(key)?;
    value.strip_prefix('\t')
}
