use super::*;
use super::table_consistency::{
    evaluate_table_consistency, format_setup_compact_with_join_mode, setup_compact_consistency_match,
    setup_compact_consistency_summary,
};

pub(super) fn append_plan_path_discovery_table_explain(
    explain: &mut Vec<String>,
    context: &PlanPathDiscoveryTableContext,
    table_policy: &MeshPeerTablePolicy,
    table_report: &MeshPeerTableEnforcementReport,
) {
    const EXPLAIN_CONTRACT_VERSION: &str = "mesh_explain_v1";
    let consistency = evaluate_table_consistency(table_policy, table_report);
    explain.push(format!("explain_contract_version={EXPLAIN_CONTRACT_VERSION}"));
    explain.push(format!("join_mode={:?}", context.join_mode));
    explain.push(format!("discovery_sources={}", context.source_count));
    explain.push(format!("discovery_source_names={}", context.source_names));
    let setup_compact = format_setup_compact_with_join_mode(
        &format!("{:?}", context.join_mode),
        context.source_count,
        table_report.total_peers_after,
        &consistency.runtime_consistency_gate,
        consistency.preemptive_degraded_path,
    );
    explain.push(format!("plan_setup_discovery_table_compact={setup_compact}"));
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
    append_peer_table_policy_explain(explain, table_policy);
    append_peer_table_enforcement_explain(explain, table_report);
}
