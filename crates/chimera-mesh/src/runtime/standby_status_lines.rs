use super::*;

pub(super) fn status_standby_shadow_lines(report: &MeshRuntimeStatusReport) -> Vec<String> {
    vec![
        format!("status_standby_shadow_mode={}", report.standby_shadow_mode),
        format!(
            "status_standby_shadow_target={}",
            report.standby_shadow_target
        ),
        format!(
            "status_standby_shadow_target_source={}",
            report.standby_shadow_target_source
        ),
        format!(
            "status_standby_shadow_reason={}",
            report.standby_shadow_reason
        ),
        format!(
            "status_standby_shadow_source={}",
            report.standby_shadow_source
        ),
        format!(
            "status_standby_shadow_warm_ready={}",
            report.standby_shadow_warm_ready
        ),
        format!(
            "status_standby_shadow_hot_ready={}",
            report.standby_shadow_hot_ready
        ),
        format!(
            "status_standby_shadow_stage_source={}",
            report.standby_shadow_stage_source
        ),
        format!(
            "status_standby_shadow_summary={}",
            report.standby_shadow_summary
        ),
    ]
}
