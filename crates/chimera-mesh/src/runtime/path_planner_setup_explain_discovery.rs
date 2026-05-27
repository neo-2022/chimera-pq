use super::table_consistency::{
    evaluate_table_consistency, format_setup_compact_with_join_mode,
    setup_compact_consistency_match, setup_compact_consistency_summary,
};
use super::*;

pub(super) fn append_plan_setup_discovery_table_explain(
    runtime: &MeshRuntime,
    explain: &mut Vec<String>,
    join_mode: MeshJoinMode,
) {
    const EXPLAIN_CONTRACT_VERSION: &str = "mesh_explain_v1";
    explain.push(format!(
        "explain_contract_version={EXPLAIN_CONTRACT_VERSION}"
    ));
    explain.push(format!("join_mode={join_mode:?}"));
    explain.push(format!("discovery_sources={}", runtime.sources.len()));
    let sources = runtime.source_list();
    explain.push(format!("discovery_source_names={}", sources.join(",")));
    explain.push(format!(
        "peer_table_policy_max_entries={}",
        runtime.table_policy.max_entries
    ));
    explain.push(format!(
        "peer_table_policy_max_entries_per_region={}",
        runtime.table_policy.max_entries_per_region
    ));
    explain.push(format!(
        "peer_table_policy_target_distinct_regions={}",
        runtime.table_policy.target_distinct_regions
    ));
    explain.push(format!(
        "peer_table_policy_profile_hysteresis_ticks={}",
        runtime.table_policy.profile_hysteresis_ticks
    ));
    explain.push(format!(
        "peer_table_policy_resilient_region_spread_bonus_weight={}",
        runtime.table_policy.resilient_region_spread_bonus_weight
    ));
    let table_report = &runtime.last_table_enforcement_report;
    explain.push(format!("peer_table_tick={}", table_report.tick));
    explain.push(format!(
        "peer_table_entries_before={}",
        table_report.total_peers_before
    ));
    explain.push(format!(
        "peer_table_entries_after={}",
        table_report.total_peers_after
    ));
    explain.push(format!(
        "peer_table_dropped_total={}",
        table_report.dropped_total
    ));
    explain.push(format!(
        "peer_table_dropped_by_region_cap={}",
        table_report.dropped_by_region_cap
    ));
    explain.push(format!(
        "peer_table_dropped_by_global_cap={}",
        table_report.dropped_by_global_cap
    ));
    explain.push(format!(
        "peer_table_protected_region_skips={}",
        table_report.protected_region_skips
    ));
    explain.push(format!(
        "peer_table_effective_profile={}",
        table_report.effective_profile
    ));
    explain.push(format!(
        "peer_table_effective_target_distinct_regions={}",
        table_report.effective_target_distinct_regions
    ));
    explain.push(format!(
        "peer_table_effective_target_source={}",
        table_report.effective_target_source
    ));
    let consistency = evaluate_table_consistency(&runtime.table_policy, table_report);
    let setup_compact = format_setup_compact_with_join_mode(
        &format!("{join_mode:?}"),
        runtime.sources.len(),
        table_report.total_peers_after,
        &consistency.runtime_consistency_gate,
        consistency.preemptive_degraded_path,
    );
    explain.push(format!(
        "plan_setup_discovery_table_compact={setup_compact}"
    ));
    let setup_compact_consistency = setup_compact_consistency_summary(
        &setup_compact,
        &consistency.runtime_consistency_gate,
        consistency.preemptive_degraded_path,
    );
    explain.push(format!(
        "plan_setup_discovery_table_compact_consistency={setup_compact_consistency}"
    ));
    explain.push(format!(
        "plan_setup_discovery_table_compact_consistency_match={}",
        setup_compact_consistency_match(&setup_compact_consistency)
    ));
    explain.push(
        "plan_setup_discovery_table_compact_consistency_match_source=plan_setup_compact"
            .to_string(),
    );
    explain.push(format!(
        "peer_table_runtime_consistency_gate={}",
        consistency.runtime_consistency_gate
    ));
    explain.push(format!(
        "peer_table_runtime_consistency_all_true={}",
        consistency.runtime_consistency_all_true
    ));
    explain.push(format!(
        "peer_table_runtime_consistency_summary={}",
        consistency.consistency_summary()
    ));
    explain.push(format!(
        "preemptive_shadow_degraded_path={}",
        consistency.preemptive_degraded_path
    ));
    explain.push(format!(
        "preemptive_shadow_degraded_reason={}",
        consistency.preemptive_degraded_reason
    ));
    explain.push(format!(
        "preemptive_shadow_degraded_summary={}",
        consistency.degraded_summary()
    ));
}
