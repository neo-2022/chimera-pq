use super::table_consistency::evaluate_table_consistency;
use super::*;

fn table_enforcement_drop_breakdown_match(report: &MeshPeerTableEnforcementReport) -> bool {
    report.dropped_total
        == report
            .dropped_by_region_cap
            .saturating_add(report.dropped_by_global_cap)
}

fn table_enforcement_drop_components_sum(report: &MeshPeerTableEnforcementReport) -> usize {
    report
        .dropped_by_region_cap
        .saturating_add(report.dropped_by_global_cap)
}

fn table_enforcement_count_transition_valid(report: &MeshPeerTableEnforcementReport) -> bool {
    if report.total_peers_before < report.total_peers_after {
        return false;
    }
    report
        .total_peers_before
        .saturating_sub(report.total_peers_after)
        == report.dropped_total
}

fn table_enforcement_non_negative_drops(report: &MeshPeerTableEnforcementReport) -> bool {
    report.total_peers_after <= report.total_peers_before
}

fn table_enforcement_drop_delta(report: &MeshPeerTableEnforcementReport) -> usize {
    report
        .total_peers_before
        .saturating_sub(report.total_peers_after)
}

fn table_enforcement_drop_delta_matches_total(report: &MeshPeerTableEnforcementReport) -> bool {
    table_enforcement_drop_delta(report) == report.dropped_total
}

fn table_enforcement_drop_accounting_matches(report: &MeshPeerTableEnforcementReport) -> bool {
    let total = report.dropped_total;
    let components = table_enforcement_drop_components_sum(report);
    let delta = table_enforcement_drop_delta(report);
    total == components && total == delta
}

fn table_enforcement_capacity_valid(
    report: &MeshPeerTableEnforcementReport,
    policy: &MeshPeerTablePolicy,
) -> bool {
    report.total_peers_after <= policy.max_entries
}

pub(super) fn format_region_distribution(runtime: &MeshRuntime) -> String {
    let region_distribution = runtime
        .region_distribution()
        .into_iter()
        .map(|(region, count)| format!("{region}:{count}"))
        .collect::<Vec<_>>()
        .join(",");
    if region_distribution.is_empty() {
        "none".to_string()
    } else {
        region_distribution
    }
}

pub(super) fn status_base_lines(
    report: &MeshRuntimeStatusReport,
    region_distribution: &str,
) -> Vec<String> {
    const EXPLAIN_CONTRACT_VERSION: &str = "mesh_explain_v1";
    let policy_target_within_capacity =
        report.table_policy.target_distinct_regions <= report.table_policy.max_entries;
    let policy_region_quota_within_capacity =
        report.table_policy.max_entries_per_region <= report.table_policy.max_entries;
    let policy_limits_invariants_all_true = report.table_policy.max_entries > 0
        && report.table_policy.max_entries_per_region > 0
        && report.table_policy.max_entries_per_region <= report.table_policy.max_entries
        && report.table_policy.target_distinct_regions > 0
        && report.table_policy.target_distinct_regions <= report.table_policy.max_entries
        && report.table_policy.stale_after_ticks > 0
        && report.table_policy.replacement_min_score_delta > 0
        && report.table_policy.degraded_replacement_min_score_delta > 0
        && report.table_policy.max_replacements_per_window > 0
        && report.table_policy.stability_window_ticks > 0
        && report.table_policy.profile_hysteresis_ticks > 0
        && report.table_policy.resilient_region_spread_bonus_weight > 0;
    let policy_summary_core = format!(
        "max_entries:{};max_entries_per_region:{};target_distinct_regions:{};stale_after_ticks:{};max_replacements_per_window:{};stability_window_ticks:{};replacement_min_score_delta:{};degraded_replacement_min_score_delta:{};profile_hysteresis_ticks:{};resilient_region_spread_bonus_weight:{};target_within_capacity:{};region_quota_within_capacity:{};limits_invariants_all_true:{}",
        report.table_policy.max_entries,
        report.table_policy.max_entries_per_region,
        report.table_policy.target_distinct_regions,
        report.table_policy.stale_after_ticks,
        report.table_policy.max_replacements_per_window,
        report.table_policy.stability_window_ticks,
        report.table_policy.replacement_min_score_delta,
        report.table_policy.degraded_replacement_min_score_delta,
        report.table_policy.profile_hysteresis_ticks,
        report.table_policy.resilient_region_spread_bonus_weight,
        policy_target_within_capacity,
        policy_region_quota_within_capacity,
        policy_limits_invariants_all_true,
    );
    let policy_summary_matches_fields = policy_summary_core.contains(&format!(
        "max_entries:{};max_entries_per_region:{};target_distinct_regions:{};stale_after_ticks:{};max_replacements_per_window:{};stability_window_ticks:{};replacement_min_score_delta:{};degraded_replacement_min_score_delta:{};profile_hysteresis_ticks:{};resilient_region_spread_bonus_weight:{};target_within_capacity:{};region_quota_within_capacity:{};limits_invariants_all_true:{}",
        report.table_policy.max_entries,
        report.table_policy.max_entries_per_region,
        report.table_policy.target_distinct_regions,
        report.table_policy.stale_after_ticks,
        report.table_policy.max_replacements_per_window,
        report.table_policy.stability_window_ticks,
        report.table_policy.replacement_min_score_delta,
        report.table_policy.degraded_replacement_min_score_delta,
        report.table_policy.profile_hysteresis_ticks,
        report.table_policy.resilient_region_spread_bonus_weight,
        policy_target_within_capacity,
        policy_region_quota_within_capacity,
        policy_limits_invariants_all_true,
    ));
    let policy_consistency_all_true = policy_target_within_capacity
        && policy_region_quota_within_capacity
        && policy_limits_invariants_all_true
        && policy_summary_matches_fields;
    let enforcement = &report.table_enforcement;
    let drop_breakdown_match = table_enforcement_drop_breakdown_match(enforcement);
    let drop_components_sum = table_enforcement_drop_components_sum(enforcement);
    let count_transition_valid = table_enforcement_count_transition_valid(enforcement);
    let non_negative_drops = table_enforcement_non_negative_drops(enforcement);
    let drop_delta = table_enforcement_drop_delta(enforcement);
    let drop_delta_matches_total = table_enforcement_drop_delta_matches_total(enforcement);
    let drop_accounting_matches = table_enforcement_drop_accounting_matches(enforcement);
    let capacity_valid = table_enforcement_capacity_valid(enforcement, &report.table_policy);
    let invariants_all_true = drop_breakdown_match
        && count_transition_valid
        && non_negative_drops
        && drop_delta_matches_total
        && drop_accounting_matches
        && capacity_valid;
    let drop_accounting = format!(
        "total:{};components:{};delta:{}",
        enforcement.dropped_total, drop_components_sum, drop_delta
    );
    let table_enforcement_summary_core = format!(
        "tick:{};before:{};after:{};drop_total:{};drop_region:{};drop_global:{};protected_region_skips:{};profile:{};target:{};source:{};invariants_all_true:{}",
        enforcement.tick,
        enforcement.total_peers_before,
        enforcement.total_peers_after,
        enforcement.dropped_total,
        enforcement.dropped_by_region_cap,
        enforcement.dropped_by_global_cap,
        enforcement.protected_region_skips,
        enforcement.effective_profile,
        enforcement.effective_target_distinct_regions,
        enforcement.effective_target_source,
        invariants_all_true,
    );
    let table_enforcement_summary_matches_fields = table_enforcement_summary_core.contains(&format!(
        "tick:{};before:{};after:{};drop_total:{};drop_region:{};drop_global:{};protected_region_skips:{};profile:{};target:{};source:{};invariants_all_true:{}",
        enforcement.tick,
        enforcement.total_peers_before,
        enforcement.total_peers_after,
        enforcement.dropped_total,
        enforcement.dropped_by_region_cap,
        enforcement.dropped_by_global_cap,
        enforcement.protected_region_skips,
        enforcement.effective_profile,
        enforcement.effective_target_distinct_regions,
        enforcement.effective_target_source,
        invariants_all_true,
    ));
    let table_summary_consistency_all_true =
        policy_summary_matches_fields && table_enforcement_summary_matches_fields;
    let helper_consistency = evaluate_table_consistency(&report.table_policy, enforcement);
    let table_runtime_consistency_all_true =
        helper_consistency.runtime_consistency_all_true && table_summary_consistency_all_true;
    let runtime_consistency_gate = if table_runtime_consistency_all_true {
        "ok".to_string()
    } else if !table_summary_consistency_all_true {
        if helper_consistency.runtime_consistency_gate == "ok" {
            "warn:summary_consistency".to_string()
        } else {
            format!(
                "{},summary_consistency",
                helper_consistency.runtime_consistency_gate
            )
        }
    } else {
        helper_consistency.runtime_consistency_gate
    };
    let policy_summary = format!(
        "{policy_summary_core};summary_matches_fields:{policy_summary_matches_fields};runtime_consistency_all_true:{table_runtime_consistency_all_true}"
    );
    let policy_invariants = format!(
        "target_within_capacity:{policy_target_within_capacity};region_quota_within_capacity:{policy_region_quota_within_capacity};limits_invariants_all_true:{policy_limits_invariants_all_true};summary_matches_fields:{policy_summary_matches_fields};policy_consistency_all_true:{policy_consistency_all_true};runtime_consistency_all_true:{table_runtime_consistency_all_true}"
    );
    let table_enforcement_summary = format!(
        "{table_enforcement_summary_core};summary_matches_fields:{table_enforcement_summary_matches_fields};runtime_consistency_all_true:{table_runtime_consistency_all_true}"
    );
    vec![
        format!("status_explain_contract_version={EXPLAIN_CONTRACT_VERSION}"),
        format!("status_namespace={}", report.namespace),
        format!("status_sources={}", report.source_count),
        format!("status_peers={}", report.peer_count),
        format!("status_health_entries={}", report.health_state_count),
        format!("status_active_profile={}", report.active_profile),
        format!("status_region_distribution={region_distribution}"),
        format!(
            "status_table_policy_max_entries={}",
            report.table_policy.max_entries
        ),
        format!(
            "status_table_policy_target_distinct_regions={}",
            report.table_policy.target_distinct_regions
        ),
        format!(
            "status_table_policy_max_entries_per_region={}",
            report.table_policy.max_entries_per_region
        ),
        format!(
            "status_table_policy_stale_after_ticks={}",
            report.table_policy.stale_after_ticks
        ),
        format!(
            "status_table_policy_max_replacements_per_window={}",
            report.table_policy.max_replacements_per_window
        ),
        format!(
            "status_table_policy_stability_window_ticks={}",
            report.table_policy.stability_window_ticks
        ),
        format!(
            "status_table_policy_replacement_min_score_delta={}",
            report.table_policy.replacement_min_score_delta
        ),
        format!(
            "status_table_policy_degraded_replacement_min_score_delta={}",
            report.table_policy.degraded_replacement_min_score_delta
        ),
        format!(
            "status_table_policy_profile_hysteresis_ticks={}",
            report.table_policy.profile_hysteresis_ticks
        ),
        format!(
            "status_table_policy_target_within_capacity={}",
            policy_target_within_capacity
        ),
        format!(
            "status_table_policy_region_quota_within_capacity={}",
            policy_region_quota_within_capacity
        ),
        format!(
            "status_table_policy_resilient_region_spread_bonus_weight={}",
            report.table_policy.resilient_region_spread_bonus_weight
        ),
        format!(
            "status_table_policy_limits_invariants_all_true={}",
            policy_limits_invariants_all_true
        ),
        format!(
            "status_table_policy_consistency_all_true={}",
            policy_consistency_all_true
        ),
        format!("status_table_policy_summary={policy_summary}"),
        format!(
            "status_table_policy_summary_matches_fields={}",
            policy_summary_matches_fields
        ),
        format!("status_table_policy_invariants={policy_invariants}"),
        format!(
            "status_table_enforcement_dropped_total={}",
            report.table_enforcement.dropped_total
        ),
        format!(
            "status_table_enforcement_total_peers_before={}",
            report.table_enforcement.total_peers_before
        ),
        format!(
            "status_table_enforcement_total_peers_after={}",
            report.table_enforcement.total_peers_after
        ),
        format!(
            "status_table_enforcement_dropped_by_region_cap={}",
            report.table_enforcement.dropped_by_region_cap
        ),
        format!(
            "status_table_enforcement_dropped_by_global_cap={}",
            report.table_enforcement.dropped_by_global_cap
        ),
        format!(
            "status_table_enforcement_drop_components_sum={}",
            drop_components_sum
        ),
        format!(
            "status_table_enforcement_protected_region_skips={}",
            report.table_enforcement.protected_region_skips
        ),
        format!(
            "status_table_enforcement_tick={}",
            report.table_enforcement.tick
        ),
        format!(
            "status_table_enforcement_effective_profile={}",
            report.table_enforcement.effective_profile
        ),
        format!(
            "status_table_enforcement_effective_target={}",
            report.table_enforcement.effective_target_distinct_regions
        ),
        format!(
            "status_table_enforcement_effective_target_source={}",
            report.table_enforcement.effective_target_source
        ),
        format!(
            "status_table_enforcement_drop_breakdown_match={}",
            drop_breakdown_match
        ),
        format!(
            "status_table_enforcement_count_transition_valid={}",
            count_transition_valid
        ),
        format!(
            "status_table_enforcement_non_negative_drops={}",
            non_negative_drops
        ),
        format!("status_table_enforcement_drop_delta={}", drop_delta),
        format!(
            "status_table_enforcement_drop_delta_matches_total={}",
            drop_delta_matches_total
        ),
        format!("status_table_enforcement_capacity_valid={}", capacity_valid),
        format!(
            "status_table_enforcement_invariants=drop_breakdown_match:{drop_breakdown_match};count_transition_valid:{count_transition_valid};non_negative_drops:{non_negative_drops};drop_delta_matches_total:{drop_delta_matches_total};drop_accounting_matches:{drop_accounting_matches};capacity_valid:{capacity_valid};summary_matches_fields:{table_enforcement_summary_matches_fields}"
        ),
        format!(
            "status_table_enforcement_summary={}",
            table_enforcement_summary
        ),
        format!(
            "status_table_enforcement_summary_matches_fields={}",
            table_enforcement_summary_matches_fields
        ),
        format!(
            "status_table_summary_consistency_all_true={}",
            table_summary_consistency_all_true
        ),
        format!(
            "status_table_runtime_consistency_all_true={}",
            table_runtime_consistency_all_true
        ),
        format!("status_table_runtime_consistency_gate={runtime_consistency_gate}"),
        format!(
            "status_table_runtime_consistency_summary={}",
            report.table_runtime_consistency_summary
        ),
        format!(
            "status_plan_setup_discovery_table_compact={}",
            report.plan_setup_discovery_table_compact
        ),
        format!(
            "status_plan_setup_discovery_table_compact_consistency={}",
            report.plan_setup_discovery_table_compact_consistency
        ),
        format!(
            "status_setup_compact_consistency_match={}",
            report.setup_compact_consistency_match
        ),
        format!(
            "status_plan_setup_discovery_table_compact_consistency_match={}",
            report.setup_compact_consistency_match
        ),
        format!(
            "status_setup_compact_consistency_match_source={}",
            report.setup_compact_consistency_match_source
        ),
        format!(
            "status_plan_setup_discovery_table_compact_consistency_match_source={}",
            report.plan_setup_compact_consistency_match_source
        ),
        format!(
            "status_table_enforcement_drop_accounting={}",
            drop_accounting
        ),
        format!(
            "status_table_enforcement_drop_accounting_matches={}",
            drop_accounting_matches
        ),
        format!(
            "status_table_enforcement_invariants_all_true={}",
            invariants_all_true
        ),
    ]
}
