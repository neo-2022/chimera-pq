pub(super) struct AutoRecoveryExplainSummary<'a> {
    pub(super) health_relax_applied: bool,
    pub(super) health_relax_reason: &'a str,
    pub(super) health_relax_stage: &'a str,
    pub(super) auto_recovery_attempts: usize,
    pub(super) auto_recovery_final_result: &'a str,
    pub(super) auto_recovery_trace: &'a str,
    pub(super) auto_recovery_trace_steps: usize,
}

pub(super) fn append_auto_recovery_explain(
    explain: &mut Vec<String>,
    summary: AutoRecoveryExplainSummary<'_>,
) {
    explain.reserve(9);
    push_line_display(
        explain,
        "effective_health_relax_applied",
        summary.health_relax_applied,
    );
    push_line_str(
        explain,
        "effective_health_relax_reason",
        summary.health_relax_reason,
    );
    push_line_str(
        explain,
        "effective_health_relax_stage",
        summary.health_relax_stage,
    );
    push_line_display(
        explain,
        "auto_recovery_attempts",
        summary.auto_recovery_attempts,
    );
    push_line_display(
        explain,
        "auto_recovery_triggered",
        summary.auto_recovery_attempts > 0,
    );
    push_line_str(
        explain,
        "auto_recovery_final_result",
        summary.auto_recovery_final_result,
    );
    push_line_display(
        explain,
        "auto_recovery_trace_steps",
        summary.auto_recovery_trace_steps,
    );
    let auto_recovery_trace_consistent =
        summary.auto_recovery_trace_steps == summary.auto_recovery_attempts * 2;
    push_line_display(
        explain,
        "auto_recovery_trace_consistent",
        auto_recovery_trace_consistent,
    );
    let auto_recovery_trace = if summary.auto_recovery_trace.is_empty() {
        "none"
    } else {
        summary.auto_recovery_trace
    };
    push_line_str(explain, "auto_recovery_trace", auto_recovery_trace);
}

fn push_line_display<T: std::fmt::Display>(explain: &mut Vec<String>, key: &str, value: T) {
    let mut out = String::with_capacity(key.len().saturating_add(32));
    out.push_str(key);
    out.push('=');
    let _ = std::fmt::write(&mut out, format_args!("{}", value));
    explain.push(out);
}

fn push_line_str(explain: &mut Vec<String>, key: &str, value: &str) {
    let mut out = String::with_capacity(key.len().saturating_add(value.len()).saturating_add(1));
    out.push_str(key);
    out.push('=');
    out.push_str(value);
    explain.push(out);
}
