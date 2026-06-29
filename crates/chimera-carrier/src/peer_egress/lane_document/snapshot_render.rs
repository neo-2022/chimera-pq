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
    PLAN_SNAPSHOT_VERSION, begin_plan_tab_comment, finish_plan_tab_comment, join_mode_to_str,
    push_plan_comment, push_plan_comment_display, push_plan_tab_display, push_plan_tab_str,
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
        begin_plan_tab_comment(output, PLAN_KEY_SELECTED_PEER);
        push_plan_tab_display(output, index);
        push_plan_tab_str(output, PLAN_KEY_SELECTED_PEER, &peer.node_id)?;
        push_plan_tab_str(output, PLAN_KEY_SELECTED_PEER, &peer.endpoint)?;
        push_plan_tab_str(output, PLAN_KEY_SELECTED_PEER, &peer.region)?;
        push_plan_tab_display(output, peer.load_score);
        push_plan_tab_display(output, peer.reliability_score);
        push_plan_tab_display(output, peer.selection_score);
        finish_plan_tab_comment(output);
    }
    push_schedule_snapshot(output, &plan.multipath_schedule)?;
    for (index, explain) in plan.explain.iter().enumerate() {
        begin_plan_tab_comment(output, PLAN_KEY_EXPLAIN);
        push_plan_tab_display(output, index);
        push_plan_tab_str(output, PLAN_KEY_EXPLAIN, explain)?;
        finish_plan_tab_comment(output);
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
            push_plan_comment_display(output, PLAN_KEY_ROUTE_BINDING_ID, route_binding_id.get());
        }
        None => {
            push_plan_comment(output, PLAN_KEY_ROUTE_BINDING_ID, "none")?;
        }
    }
    push_plan_comment_display(
        output,
        PLAN_KEY_LANE_REQUESTED,
        schedule.lane_admission_requested_active_lane_count,
    );
    push_plan_comment_display(
        output,
        PLAN_KEY_LANE_ADMITTED,
        schedule.lane_admission_admitted_active_lane_count,
    );
    push_plan_comment_display(
        output,
        PLAN_KEY_LANE_REJECTED,
        schedule.lane_admission_rejected_active_lane_count,
    );
    push_plan_comment(
        output,
        PLAN_KEY_LANE_CAPACITY_STATUS,
        &schedule.lane_admission_capacity_status,
    )?;
    push_plan_comment_display(
        output,
        PLAN_KEY_LOCAL_RESERVE_PCT,
        schedule.local_traffic_reserve_pct,
    );
    push_plan_comment_display(
        output,
        PLAN_KEY_TRANSIT_BUDGET_PCT,
        schedule.transit_capacity_budget_pct,
    );
    push_plan_comment(output, PLAN_KEY_DEMAND_POLICY, &schedule.demand_policy)?;
    push_plan_comment(
        output,
        PLAN_KEY_DEMAND_POLICY_SOURCE,
        &schedule.demand_policy_source,
    )?;
    push_plan_comment_display(
        output,
        PLAN_KEY_DEMAND_REQUESTED,
        schedule.demand_requested_active_lane_count,
    );
    push_plan_comment_display(
        output,
        PLAN_KEY_DEMAND_PLANNED,
        schedule.demand_planned_active_lane_count,
    );
    push_plan_comment_display(
        output,
        PLAN_KEY_DEMAND_ADMITTED_CAPACITY_PCT,
        schedule.demand_admitted_lane_capacity_pct,
    );
    push_plan_comment_display(
        output,
        PLAN_KEY_DEMAND_UNMET,
        schedule.demand_unmet_lane_count,
    );
    push_plan_comment(output, PLAN_KEY_DEMAND_STATUS, &schedule.demand_status)?;
    push_plan_comment_display(
        output,
        PLAN_KEY_DEMAND_REBUILD_RECOMMENDED,
        schedule.demand_rebuild_recommended,
    );
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
    push_plan_comment_display(
        output,
        "chimera_plan_active_lane_count",
        schedule.active_lane_count,
    );
    push_plan_comment_display(
        output,
        "chimera_plan_standby_lane_count",
        schedule.standby_lane_count,
    );
    push_plan_comment_display(
        output,
        "chimera_plan_active_weight_sum_pct",
        schedule.active_weight_sum_pct,
    );
    push_plan_comment_display(
        output,
        "chimera_plan_active_capacity_sum_pct",
        schedule.active_capacity_sum_pct,
    );
    for binding in &schedule.carrier_lane_bindings {
        begin_plan_tab_comment(output, PLAN_KEY_CARRIER_BINDING);
        push_plan_tab_display(output, binding.route_binding_id.get());
        push_plan_tab_display(output, binding.lane_id);
        push_plan_tab_str(output, PLAN_KEY_CARRIER_BINDING, &binding.peer_node_id)?;
        push_plan_tab_str(output, PLAN_KEY_CARRIER_BINDING, &binding.carrier_endpoint)?;
        push_plan_tab_str(output, PLAN_KEY_CARRIER_BINDING, binding.role.as_str())?;
        push_plan_tab_display(output, binding.weight_pct);
        push_plan_tab_display(output, binding.capacity_weight_pct);
        finish_plan_tab_comment(output);
    }
    Ok(())
}
