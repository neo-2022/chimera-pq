use super::preemptive_helpers::{shadow_switch_guard_meta, tuning_source_label};
use super::preemptive_shadow_eval::{PreemptiveShadowExplainContext, evaluate_preemptive_shadow};
use super::*;

#[path = "preemptive_shadow_explain_sections.rs"]
mod sections;

pub(super) fn append_preemptive_shadow_explain(
    explain: &mut Vec<String>,
    ctx: &PreemptiveShadowExplainContext<'_>,
) {
    let eval = evaluate_preemptive_shadow(ctx);
    let shadow = eval.shadow;
    let (switch_guard, switch_guard_source) =
        shadow_switch_guard_meta(shadow.switch.reason.as_str());

    sections::append_preemptive_shadow_risk_lines(explain, &shadow);
    sections::append_preemptive_shadow_switch_lines(
        explain,
        &shadow,
        sections::PreemptiveShadowSwitchLineMeta {
            switch_guard,
            switch_guard_source,
            switch_target: &eval.switch_target,
            switch_candidate_sample_age_ticks: eval.switch_candidate_sample_age_ticks,
            switch_confidence_gate_min: eval.switch_confidence_gate_min,
            switch_confidence_gate_passed: eval.switch_confidence_gate_passed,
        },
    );
    sections::append_preemptive_shadow_confirm_lines(
        explain,
        &shadow,
        eval.confirm_ratio,
        eval.confirm_missing_signals,
    );
    sections::append_preemptive_shadow_validation_lines(
        explain,
        &shadow,
        eval.health_blocked_count,
        eval.replacements_window,
        ctx.table_policy.max_replacements_per_window,
        eval.switch_confidence_gate_passed,
        eval.switch_candidate_sample_age_ticks,
    );
    sections::append_preemptive_shadow_tuning_lines(
        explain,
        &eval.tuning,
        ctx.profile,
        tuning_source_label,
    );
}
