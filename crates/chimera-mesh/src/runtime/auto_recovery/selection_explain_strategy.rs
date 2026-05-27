use super::*;

pub(crate) fn append_selection_strategy_explain<'a>(
    explain: &mut Vec<String>,
    context: &SelectionStrategyExplainContext,
    selected_peers: &'a [MeshPeerState],
) -> BTreeSet<&'a str> {
    explain.push(format!(
        "selection_region_cap={}",
        context.selection_region_cap
    ));
    explain.push(format!(
        "region_cap_rejections={}",
        context.region_cap_rejections
    ));
    if context.prefer_region_diversity {
        explain.push("selection_strategy=region_diversity".to_string());
    } else {
        explain.push("selection_strategy=score_only".to_string());
    }
    explain.push(format!("selected_peers={}", selected_peers.len()));
    let selected_regions: BTreeSet<&str> = selected_peers
        .iter()
        .map(|peer| peer.region.as_str())
        .collect();
    explain.push(format!("selected_regions={}", selected_regions.len()));
    selected_regions
}

pub(crate) fn append_region_selection_diagnostics_explain(
    explain: &mut Vec<String>,
    diagnostics: &RegionSelectionDiagnostics,
) {
    explain.push(format!(
        "min_distinct_regions_target={}",
        diagnostics.min_distinct_regions_target
    ));
    explain.push(format!(
        "min_distinct_regions_met={}",
        diagnostics.min_distinct_regions_met
    ));
    explain.push(format!(
        "distinct_region_deficit={}",
        diagnostics.distinct_region_deficit
    ));
    explain.push(format!(
        "distinct_region_ratio_pct={}",
        diagnostics.distinct_region_ratio_pct
    ));
}

pub(crate) fn append_selection_region_diagnostics_explain(
    explain: &mut Vec<String>,
    selection_context: &SelectionFeasibilityContext,
    diagnostics: &RegionSelectionDiagnostics,
) {
    append_selection_feasibility_explain(explain, selection_context);
    append_region_selection_diagnostics_explain(explain, diagnostics);
}

pub(crate) fn append_spread_bonus_explain(explain: &mut Vec<String>, spread_bonus: (bool, i32)) {
    explain.push(format!(
        "resilient_region_spread_bonus_applied={}",
        spread_bonus.0
    ));
    explain.push(format!(
        "resilient_region_spread_bonus_total={}",
        spread_bonus.1
    ));
}

pub(crate) fn append_selection_feasibility_explain(
    explain: &mut Vec<String>,
    selection_context: &SelectionFeasibilityContext,
) {
    explain.push(format!(
        "candidate_distinct_regions={}",
        selection_context.candidate_distinct_regions
    ));
    explain.push(format!(
        "min_distinct_regions_feasible={}",
        selection_context.min_distinct_regions_feasible
    ));
    explain.push(format!(
        "min_distinct_regions_feasibility_gap={}",
        selection_context.min_distinct_regions_feasibility_gap
    ));
}
