use super::*;

pub(super) fn failover_plan(
    runtime: &MeshRuntime,
    request: &MeshJoinRequest,
    policy: &MeshPathPolicy,
    event: &MeshFailoverEvent,
) -> Result<MeshPathPlan, String> {
    validate_runtime_node_id(&event.failed_node_id, "mesh failover failed_node_id")?;
    if event.reason.trim().is_empty() {
        return Err("mesh failover reason is empty".to_string());
    }

    let mut failover_policy = policy.clone();
    if !failover_policy
        .blocked_node_ids
        .iter()
        .any(|node| node == &event.failed_node_id)
    {
        failover_policy
            .blocked_node_ids
            .push(event.failed_node_id.clone());
    }

    let mut plan = runtime.plan_path(request, &failover_policy)?;
    let public_failed_node =
        diagnostic_redaction::peer_label_from_table(&event.failed_node_id, &runtime.peers);
    plan.explain.push(format!(
        "failover_triggered node={} reason={}",
        public_failed_node, event.reason
    ));
    Ok(plan)
}
