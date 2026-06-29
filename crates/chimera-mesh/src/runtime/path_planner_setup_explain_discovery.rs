use super::table_consistency::{
    evaluate_table_consistency, format_setup_compact_with_join_mode, setup_compact_consistency,
};
use super::*;
use std::fmt::{Display, Write};

pub(super) fn append_plan_setup_discovery_table_explain(
    runtime: &MeshRuntime,
    explain: &mut Vec<String>,
    join_mode: MeshJoinMode,
) {
    explain.reserve(32);
    const EXPLAIN_CONTRACT_VERSION: &str = "mesh_explain_v1";
    let join_mode_label = join_mode.label();
    push_line_str(
        explain,
        "explain_contract_version",
        EXPLAIN_CONTRACT_VERSION,
    );
    push_line_str(explain, "join_mode", join_mode_label);
    push_line_display(explain, "discovery_sources", runtime.sources.len());
    push_line_display(
        explain,
        "discovery_source_names",
        join_runtime_sources(runtime),
    );
    push_line_display(
        explain,
        "peer_table_policy_max_entries",
        runtime.table_policy.max_entries,
    );
    push_line_display(
        explain,
        "peer_table_policy_max_entries_per_region",
        runtime.table_policy.max_entries_per_region,
    );
    push_line_display(
        explain,
        "peer_table_policy_target_distinct_regions",
        runtime.table_policy.target_distinct_regions,
    );
    push_line_display(
        explain,
        "peer_table_policy_profile_hysteresis_ticks",
        runtime.table_policy.profile_hysteresis_ticks,
    );
    push_line_display(
        explain,
        "peer_table_policy_resilient_region_spread_bonus_weight",
        runtime.table_policy.resilient_region_spread_bonus_weight,
    );
    let table_report = &runtime.last_table_enforcement_report;
    push_line_display(explain, "peer_table_tick", table_report.tick);
    push_line_display(
        explain,
        "peer_table_entries_before",
        table_report.total_peers_before,
    );
    push_line_display(
        explain,
        "peer_table_entries_after",
        table_report.total_peers_after,
    );
    push_line_display(
        explain,
        "peer_table_dropped_total",
        table_report.dropped_total,
    );
    push_line_display(
        explain,
        "peer_table_dropped_by_region_cap",
        table_report.dropped_by_region_cap,
    );
    push_line_display(
        explain,
        "peer_table_dropped_by_global_cap",
        table_report.dropped_by_global_cap,
    );
    push_line_display(
        explain,
        "peer_table_protected_region_skips",
        table_report.protected_region_skips,
    );
    push_line_display(
        explain,
        "peer_table_effective_profile",
        table_report.effective_profile.as_str(),
    );
    push_line_display(
        explain,
        "peer_table_effective_target_distinct_regions",
        table_report.effective_target_distinct_regions,
    );
    push_line_display(
        explain,
        "peer_table_effective_target_source",
        table_report.effective_target_source.as_str(),
    );
    let consistency = evaluate_table_consistency(&runtime.table_policy, table_report);
    let consistency_summary = consistency.consistency_summary();
    let degraded_summary = consistency.degraded_summary();
    let setup_compact = format_setup_compact_with_join_mode(
        join_mode_label,
        runtime.sources.len(),
        table_report.total_peers_after,
        &consistency.runtime_consistency_gate,
        consistency.preemptive_degraded_path,
    );
    push_line_str(
        explain,
        "plan_setup_discovery_table_compact",
        &setup_compact,
    );
    let (setup_compact_consistency, setup_compact_consistency_match) = setup_compact_consistency(
        &setup_compact,
        &consistency.runtime_consistency_gate,
        consistency.preemptive_degraded_path,
    );
    push_line_str(
        explain,
        "plan_setup_discovery_table_compact_consistency",
        &setup_compact_consistency,
    );
    push_line_display(
        explain,
        "plan_setup_discovery_table_compact_consistency_match",
        setup_compact_consistency_match,
    );
    explain.push(
        "plan_setup_discovery_table_compact_consistency_match_source=plan_setup_compact"
            .to_string(),
    );
    push_line_display(
        explain,
        "peer_table_runtime_consistency_gate",
        &consistency.runtime_consistency_gate,
    );
    push_line_display(
        explain,
        "peer_table_runtime_consistency_all_true",
        consistency.runtime_consistency_all_true,
    );
    push_line_display(
        explain,
        "peer_table_runtime_consistency_summary",
        consistency_summary,
    );
    push_line_display(
        explain,
        "preemptive_shadow_degraded_path",
        consistency.preemptive_degraded_path,
    );
    push_line_display(
        explain,
        "preemptive_shadow_degraded_reason",
        consistency.preemptive_degraded_reason.as_str(),
    );
    push_line_display(
        explain,
        "preemptive_shadow_degraded_summary",
        degraded_summary,
    );
}

fn join_runtime_sources(runtime: &MeshRuntime) -> String {
    let source_count = runtime.sources.len();
    if source_count == 0 {
        return String::new();
    }

    let capacity = runtime
        .sources
        .iter()
        .map(String::len)
        .sum::<usize>()
        .saturating_add(source_count.saturating_sub(1));
    let mut out = String::with_capacity(capacity);
    for (index, source) in runtime.sources.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(source);
    }
    out
}

fn push_line_display<T: Display>(explain: &mut Vec<String>, key: &str, value: T) {
    let mut out = String::with_capacity(key.len().saturating_add(32));
    out.push_str(key);
    out.push('=');
    let _ = write!(&mut out, "{}", value);
    explain.push(out);
}

fn push_line_str(explain: &mut Vec<String>, key: &str, value: &str) {
    let mut out = String::with_capacity(key.len().saturating_add(value.len()).saturating_add(1));
    out.push_str(key);
    out.push('=');
    out.push_str(value);
    explain.push(out);
}
