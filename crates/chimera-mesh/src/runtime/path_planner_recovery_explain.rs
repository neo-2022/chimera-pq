pub(super) struct AutoRecoveryExplainSummary<'a> {
    pub(super) health_relax_applied: bool,
    pub(super) health_relax_reason: &'a str,
    pub(super) health_relax_stage: &'a str,
    pub(super) auto_recovery_attempts: usize,
    pub(super) auto_recovery_final_result: &'a str,
    pub(super) auto_recovery_trace: &'a [&'a str],
}

pub(super) fn append_auto_recovery_explain(
    explain: &mut Vec<String>,
    summary: AutoRecoveryExplainSummary<'_>,
) {
    explain.push(format!(
        "effective_health_relax_applied={}",
        summary.health_relax_applied
    ));
    explain.push(format!(
        "effective_health_relax_reason={}",
        summary.health_relax_reason
    ));
    explain.push(format!(
        "effective_health_relax_stage={}",
        summary.health_relax_stage
    ));
    explain.push(format!(
        "auto_recovery_attempts={}",
        summary.auto_recovery_attempts
    ));
    explain.push(format!(
        "auto_recovery_triggered={}",
        summary.auto_recovery_attempts > 0
    ));
    explain.push(format!(
        "auto_recovery_final_result={}",
        summary.auto_recovery_final_result
    ));
    explain.push(format!(
        "auto_recovery_trace_steps={}",
        summary.auto_recovery_trace.len()
    ));
    let auto_recovery_trace_consistent =
        summary.auto_recovery_trace.len() == summary.auto_recovery_attempts * 2;
    explain.push(format!(
        "auto_recovery_trace_consistent={auto_recovery_trace_consistent}"
    ));
    explain.push(format!(
        "auto_recovery_trace={}",
        if summary.auto_recovery_trace.is_empty() {
            "none".to_string()
        } else {
            summary.auto_recovery_trace.join("->")
        }
    ));
}
