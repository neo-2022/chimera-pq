use super::preemptive_shadow_eval::PreemptiveShadowExplainContext;
use super::preemptive_shadow_explain::append_preemptive_shadow_explain;
use super::*;
use crate::runtime::selection_profile::profile_label;

pub(super) fn append_plan_setup_preface_explain(
    runtime: &MeshRuntime,
    explain: &mut Vec<String>,
    ctx: &PlanSetupPrefaceExplainContext<'_>,
) {
    let decision_control_mode = if ctx.manual_overrides.is_empty() {
        "auto"
    } else {
        "manual_override"
    };
    let path_profile_source = if ctx.path_profile_overridden {
        "manual_override"
    } else {
        "auto"
    };

    explain.push(format!("decision_control_mode={decision_control_mode}"));
    explain.push(format!(
        "manual_override_count={}",
        ctx.manual_overrides.len()
    ));
    explain.push(format!(
        "manual_override_fields={}",
        if ctx.manual_overrides.is_empty() {
            "none".to_string()
        } else {
            ctx.manual_overrides.join(",")
        }
    ));
    explain.push(format!("path_profile={}", profile_label(ctx.path_profile)));
    explain.push(format!("path_profile_source={path_profile_source}"));
    explain.push(format!("path_profile_reason={}", ctx.path_profile_reason));
    explain.push(format!("runtime_avg_load_score={}", ctx.avg_load_score));
    explain.push(format!(
        "runtime_avg_reliability_score={}",
        ctx.avg_reliability_score
    ));

    let unhealthy_nodes = runtime.unhealthy_node_ids();
    append_preemptive_shadow_explain(
        explain,
        &PreemptiveShadowExplainContext {
            profile: runtime.profile_state.active_profile,
            avg_load_score: ctx.avg_load_score,
            avg_reliability_score: ctx.avg_reliability_score,
            peers: &runtime.peers,
            unhealthy_nodes: &unhealthy_nodes,
            health_state_count: runtime.health_state.len(),
            table_policy: &runtime.table_policy,
            peer_meta: &runtime.peer_meta,
            tick: runtime.tick,
        },
    );
}
