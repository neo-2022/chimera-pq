use super::*;

pub(super) fn append_plan_path_preface_explain(
    explain: &mut Vec<String>,
    context: &PlanPathPrefaceContext<'_>,
) {
    explain.push(format!(
        "decision_control_mode={}",
        context.decision_control_mode
    ));
    explain.push(format!(
        "manual_override_count={}",
        context.manual_override_count
    ));
    explain.push(format!(
        "manual_override_fields={}",
        context.manual_override_fields
    ));
    explain.push(format!(
        "path_profile={}",
        profile_label(context.path_profile)
    ));
    explain.push(format!(
        "path_profile_source={}",
        context.path_profile_source
    ));
    explain.push(format!(
        "path_profile_reason={}",
        context.path_profile_reason
    ));
    explain.push(format!("runtime_avg_load_score={}", context.avg_load_score));
    explain.push(format!(
        "runtime_avg_reliability_score={}",
        context.avg_reliability_score
    ));
}
