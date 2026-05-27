use super::*;

pub(super) fn reselection_plan_with_health(
    runtime: &MeshRuntime,
    request: &MeshJoinRequest,
    policy: &MeshPathPolicy,
    health: &[MeshPeerHealth],
) -> Result<MeshPathPlan, String> {
    let mut adjusted_policy = policy.clone();
    for persisted in runtime.health_state.values() {
        if (!persisted.health.healthy || persisted.health.cooldown_active)
            && !adjusted_policy
                .blocked_node_ids
                .iter()
                .any(|blocked| blocked == &persisted.health.node_id)
        {
            adjusted_policy
                .blocked_node_ids
                .push(persisted.health.node_id.clone());
        }
    }
    for item in health {
        validate_runtime_node_id(&item.node_id, "mesh health node_id")?;
        if (!item.healthy || item.cooldown_active)
            && !adjusted_policy
                .blocked_node_ids
                .iter()
                .any(|blocked| blocked == &item.node_id)
        {
            adjusted_policy.blocked_node_ids.push(item.node_id.clone());
        }
    }

    let mut plan = runtime.plan_path(request, &adjusted_policy)?;
    let affected = health
        .iter()
        .filter(|h| !h.healthy || h.cooldown_active)
        .count();
    plan.explain
        .push(format!("health_reselection_applied={affected}"));
    Ok(plan)
}
