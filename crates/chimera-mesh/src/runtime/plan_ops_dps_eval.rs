use super::*;
use crate::dps_payload_snapshot::MeshDpsPayloadSnapshot;
use crate::runtime::plan_dps_adaptation::apply_dps_traffic_hints_adaptation;
use crate::runtime::standby_shadow_explain::adapt_standby_shadow_from_dps;

pub(super) struct DpsPayloadPlanContext {
    pub(super) policy: MeshPathPolicy,
    pub(super) snapshot: MeshDpsPayloadSnapshot,
}

pub(super) fn policy_and_snapshot_from_dps_payload(
    payload: &str,
) -> Result<DpsPayloadPlanContext, String> {
    if payload.trim().is_empty() {
        return Err("mesh policy payload must include at least one mesh_* field".to_string());
    }
    let parsed = crate::dps_payload_snapshot::parse_mesh_dps_payload(payload)?;
    if parsed.snapshot.mesh_field_count() == 0 {
        return Err("mesh policy payload must include at least one mesh_* field".to_string());
    }
    let mut policy = parsed.policy;
    apply_dps_traffic_hints_adaptation(&parsed.snapshot, &mut policy);
    Ok(DpsPayloadPlanContext {
        policy,
        snapshot: parsed.snapshot,
    })
}

pub(super) fn plan_path_from_dps_payload(
    runtime: &MeshRuntime,
    request: &MeshJoinRequest,
    payload: &str,
) -> Result<MeshPathPlan, String> {
    let context = policy_and_snapshot_from_dps_payload(payload)?;
    let mut plan = runtime.plan_path(request, &context.policy)?;
    annotate_dps_payload_explain(&mut plan.explain, &context.snapshot, "plan");
    adapt_standby_shadow_from_dps(
        &plan.selected_peers,
        &mut plan.explain,
        context
            .snapshot
            .traffic_hints()
            .multipath_mode
            .map(|mode| mode.as_str()),
    );
    apply_dps_multipath_schedule(&context.snapshot, &mut plan)?;
    Ok(plan)
}

pub(super) fn apply_dps_multipath_schedule(
    snapshot: &MeshDpsPayloadSnapshot,
    plan: &mut MeshPathPlan,
) -> Result<(), String> {
    let hints = snapshot.traffic_hints();
    if let Some(mode) = hints.multipath_mode {
        replace_multipath_schedule(
            plan,
            schedule_mode_from_multipath_hint(mode),
            snapshot.route_binding_id(),
            hints.multipath_demand,
        )?;
    }
    Ok(())
}

pub(super) fn evaluate_dps_policy_payload(
    runtime: &MeshRuntime,
    request: &MeshJoinRequest,
    payload: &str,
) -> Result<MeshPathPlan, String> {
    let mut plan = plan_path_from_dps_payload(runtime, request, payload)?;
    plan.explain.push("dps_policy_evaluation=true".to_string());
    Ok(plan)
}

pub(super) fn evaluate_dps_policy_payload_with_health(
    runtime: &MeshRuntime,
    request: &MeshJoinRequest,
    payload: &str,
    health: &[MeshPeerHealth],
) -> Result<MeshPathPlan, String> {
    let mut plan =
        runtime.reselection_plan_with_health_from_dps_payload(request, payload, health)?;
    plan.explain
        .push("dps_policy_evaluation_with_health=true".to_string());
    Ok(plan)
}

pub(super) fn evaluate_dps_failover_payload(
    runtime: &MeshRuntime,
    request: &MeshJoinRequest,
    payload: &str,
    event: &MeshFailoverEvent,
) -> Result<MeshPathPlan, String> {
    let mut plan = runtime.failover_plan_from_dps_payload(request, payload, event)?;
    plan.explain
        .push("dps_policy_evaluation_failover=true".to_string());
    Ok(plan)
}
