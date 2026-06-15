use super::*;
use crate::runtime::plan_dps_adaptation::apply_dps_traffic_hints_adaptation;
use crate::runtime::standby_shadow_explain::adapt_standby_shadow_from_dps;

pub(super) fn policy_from_dps_payload_with_traffic_hints(
    payload: &str,
) -> Result<MeshPathPolicy, String> {
    let mut policy = MeshPathPolicy::from_dps_payload(payload)?;
    apply_dps_traffic_hints_adaptation(payload, &mut policy);
    Ok(policy)
}

pub(super) fn plan_path_from_dps_payload(
    runtime: &MeshRuntime,
    request: &MeshJoinRequest,
    payload: &str,
) -> Result<MeshPathPlan, String> {
    ensure_mesh_payload_nonempty(payload)?;
    let policy = policy_from_dps_payload_with_traffic_hints(payload)?;
    let mut plan = runtime.plan_path(request, &policy)?;
    annotate_dps_payload_explain(&mut plan.explain, payload, "plan");
    adapt_standby_shadow_from_dps(&mut plan.explain);
    apply_dps_multipath_schedule(payload, &mut plan);
    Ok(plan)
}

pub(super) fn apply_dps_multipath_schedule(payload: &str, plan: &mut MeshPathPlan) {
    if let Ok(hints) = traffic_hints_from_dps_payload(payload)
        && let Some(mode) = hints.multipath_mode
    {
        replace_multipath_schedule(plan, schedule_mode_from_multipath_hint(mode));
    }
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
