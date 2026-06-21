use chimera_mesh::{MeshMultipathSchedule, MeshPathPlan};

use super::format::{
    PLAN_KEY_CARRIER_BINDING, PLAN_KEY_DEMAND_ADMITTED_CAPACITY_PCT, PLAN_KEY_DEMAND_PLANNED,
    PLAN_KEY_DEMAND_POLICY, PLAN_KEY_DEMAND_POLICY_SOURCE, PLAN_KEY_DEMAND_REBUILD_RECOMMENDED,
    PLAN_KEY_DEMAND_REQUESTED, PLAN_KEY_DEMAND_STATUS, PLAN_KEY_DEMAND_UNMET,
    PLAN_KEY_EXECUTION_STATUS, PLAN_KEY_EXPLAIN, PLAN_KEY_FAIRNESS_POLICY, PLAN_KEY_JOIN_MODE,
    PLAN_KEY_LANE_ADMITTED, PLAN_KEY_LANE_CAPACITY_STATUS, PLAN_KEY_LANE_REJECTED,
    PLAN_KEY_LANE_REQUESTED, PLAN_KEY_LOCAL_RESERVE_PCT, PLAN_KEY_MODE, PLAN_KEY_NAMESPACE,
    PLAN_KEY_PLANNER_REBUILD_REASON, PLAN_KEY_ROUTE_BINDING_ID, PLAN_KEY_SELECTED_PEER,
    PLAN_KEY_TRANSIT_BUDGET_PCT, PLAN_KEY_TRANSIT_PAYLOAD_POLICY, PLAN_KEY_VERSION,
    PLAN_SNAPSHOT_VERSION, cleaned_comment_field, join_mode_to_str, push_plan_comment,
    push_plan_tab_comment,
};

pub(super) fn render_transit_lane_plan_snapshot(
    plan: &MeshPathPlan,
    output: &mut String,
) -> Result<(), String> {
    push_plan_comment(output, PLAN_KEY_VERSION, PLAN_SNAPSHOT_VERSION)?;
    push_plan_comment(output, PLAN_KEY_NAMESPACE, &plan.namespace)?;
    push_plan_comment(
        output,
        PLAN_KEY_JOIN_MODE,
        join_mode_to_str(&plan.join_mode),
    )?;
    for (index, peer) in plan.selected_peers.iter().enumerate() {
        push_plan_tab_comment(
            output,
            PLAN_KEY_SELECTED_PEER,
            &[
                index.to_string(),
                cleaned_comment_field(&peer.node_id, "mesh plan selected peer node_id")?
                    .to_string(),
                cleaned_comment_field(&peer.endpoint, "mesh plan selected peer endpoint")?
                    .to_string(),
                cleaned_comment_field(&peer.region, "mesh plan selected peer region")?.to_string(),
                peer.load_score.to_string(),
                peer.reliability_score.to_string(),
                peer.selection_score.to_string(),
            ],
        )?;
    }
    push_schedule_snapshot(output, &plan.multipath_schedule)?;
    for (index, explain) in plan.explain.iter().enumerate() {
        push_plan_tab_comment(
            output,
            PLAN_KEY_EXPLAIN,
            &[
                index.to_string(),
                cleaned_comment_field(explain, "mesh plan explain line")?.to_string(),
            ],
        )?;
    }
    Ok(())
}

fn push_schedule_snapshot(
    output: &mut String,
    schedule: &MeshMultipathSchedule,
) -> Result<(), String> {
    push_plan_comment(output, PLAN_KEY_MODE, schedule.mode.as_str())?;
    match schedule.route_binding_id.as_ref() {
        Some(route_binding_id) => {
            push_plan_comment(
                output,
                PLAN_KEY_ROUTE_BINDING_ID,
                &route_binding_id.get().to_string(),
            )?;
        }
        None => {
            push_plan_comment(output, PLAN_KEY_ROUTE_BINDING_ID, "none")?;
        }
    }
    push_plan_comment(
        output,
        PLAN_KEY_LANE_REQUESTED,
        &schedule
            .lane_admission_requested_active_lane_count
            .to_string(),
    )?;
    push_plan_comment(
        output,
        PLAN_KEY_LANE_ADMITTED,
        &schedule
            .lane_admission_admitted_active_lane_count
            .to_string(),
    )?;
    push_plan_comment(
        output,
        PLAN_KEY_LANE_REJECTED,
        &schedule
            .lane_admission_rejected_active_lane_count
            .to_string(),
    )?;
    push_plan_comment(
        output,
        PLAN_KEY_LANE_CAPACITY_STATUS,
        &schedule.lane_admission_capacity_status,
    )?;
    push_plan_comment(
        output,
        PLAN_KEY_LOCAL_RESERVE_PCT,
        &schedule.local_traffic_reserve_pct.to_string(),
    )?;
    push_plan_comment(
        output,
        PLAN_KEY_TRANSIT_BUDGET_PCT,
        &schedule.transit_capacity_budget_pct.to_string(),
    )?;
    push_plan_comment(output, PLAN_KEY_DEMAND_POLICY, &schedule.demand_policy)?;
    push_plan_comment(
        output,
        PLAN_KEY_DEMAND_POLICY_SOURCE,
        &schedule.demand_policy_source,
    )?;
    push_plan_comment(
        output,
        PLAN_KEY_DEMAND_REQUESTED,
        &schedule.demand_requested_active_lane_count.to_string(),
    )?;
    push_plan_comment(
        output,
        PLAN_KEY_DEMAND_PLANNED,
        &schedule.demand_planned_active_lane_count.to_string(),
    )?;
    push_plan_comment(
        output,
        PLAN_KEY_DEMAND_ADMITTED_CAPACITY_PCT,
        &schedule.demand_admitted_lane_capacity_pct.to_string(),
    )?;
    push_plan_comment(
        output,
        PLAN_KEY_DEMAND_UNMET,
        &schedule.demand_unmet_lane_count.to_string(),
    )?;
    push_plan_comment(output, PLAN_KEY_DEMAND_STATUS, &schedule.demand_status)?;
    push_plan_comment(
        output,
        PLAN_KEY_DEMAND_REBUILD_RECOMMENDED,
        &schedule.demand_rebuild_recommended.to_string(),
    )?;
    push_plan_comment(output, PLAN_KEY_FAIRNESS_POLICY, &schedule.fairness_policy)?;
    push_plan_comment(
        output,
        PLAN_KEY_EXECUTION_STATUS,
        &schedule.execution_status,
    )?;
    push_plan_comment(
        output,
        PLAN_KEY_TRANSIT_PAYLOAD_POLICY,
        &schedule.transit_payload_policy,
    )?;
    push_plan_comment(
        output,
        PLAN_KEY_PLANNER_REBUILD_REASON,
        &schedule.planner_rebuild_reason,
    )?;
    push_plan_comment(
        output,
        "chimera_plan_active_lane_count",
        &schedule.active_lane_count.to_string(),
    )?;
    push_plan_comment(
        output,
        "chimera_plan_standby_lane_count",
        &schedule.standby_lane_count.to_string(),
    )?;
    push_plan_comment(
        output,
        "chimera_plan_active_weight_sum_pct",
        &schedule.active_weight_sum_pct.to_string(),
    )?;
    push_plan_comment(
        output,
        "chimera_plan_active_capacity_sum_pct",
        &schedule.active_capacity_sum_pct.to_string(),
    )?;
    for binding in &schedule.carrier_lane_bindings {
        push_plan_tab_comment(
            output,
            PLAN_KEY_CARRIER_BINDING,
            &[
                binding.route_binding_id.get().to_string(),
                binding.lane_id.to_string(),
                cleaned_comment_field(
                    &binding.peer_node_id,
                    "mesh plan carrier binding peer_node_id",
                )?
                .to_string(),
                cleaned_comment_field(
                    &binding.carrier_endpoint,
                    "mesh plan carrier binding endpoint",
                )?
                .to_string(),
                binding.role.as_str().to_string(),
                binding.weight_pct.to_string(),
                binding.capacity_weight_pct.to_string(),
            ],
        )?;
    }
    Ok(())
}
