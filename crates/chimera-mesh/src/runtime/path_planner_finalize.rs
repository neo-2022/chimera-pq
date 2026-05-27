use super::path_planner_selection_explain::{SelectionExplainInput, append_selection_explain};
use super::selection_policy::{select_by_score_with_region_cap, select_with_region_diversity};
use super::*;

pub(super) struct SelectionFinalizeInput<'a> {
    pub(super) policy: &'a MeshPathPolicy,
    pub(super) stats: CandidateStats,
    pub(super) candidates: Vec<MeshPeerState>,
    pub(super) effective_prefer_region_diversity: bool,
    pub(super) effective_max_peers: usize,
    pub(super) effective_max_selected_per_region: usize,
    pub(super) effective_min_distinct_regions: usize,
}

pub(super) fn finalize_selection(
    runtime: &MeshRuntime,
    input: SelectionFinalizeInput<'_>,
    explain: &mut Vec<String>,
) -> Result<Vec<MeshPeerState>, String> {
    let SelectionFinalizeInput {
        policy,
        stats,
        mut candidates,
        effective_prefer_region_diversity,
        effective_max_peers,
        effective_max_selected_per_region,
        effective_min_distinct_regions,
    } = input;
    candidates.sort_by(|a, b| {
        b.selection_score
            .cmp(&a.selection_score)
            .then_with(|| a.load_score.cmp(&b.load_score))
            .then_with(|| b.reliability_score.cmp(&a.reliability_score))
            .then_with(|| a.node_id.cmp(&b.node_id))
    });
    let candidate_distinct_regions: BTreeSet<String> = candidates
        .iter()
        .map(|peer| normalize_region_key(&peer.region))
        .collect();
    let min_distinct_regions_feasible =
        candidate_distinct_regions.len() >= policy.min_distinct_regions;
    let min_distinct_regions_feasibility_gap = policy
        .min_distinct_regions
        .saturating_sub(candidate_distinct_regions.len());

    let (selected_peers, region_cap_rejections): (Vec<MeshPeerState>, usize) =
        if effective_prefer_region_diversity {
            select_with_region_diversity(
                candidates,
                effective_max_peers,
                effective_max_selected_per_region,
            )
        } else {
            select_by_score_with_region_cap(
                candidates,
                effective_max_peers,
                effective_max_selected_per_region,
            )
        };
    if selected_peers.is_empty() {
        return Err("mesh path plan has zero eligible peers".to_string());
    }
    explain.push(format!(
        "selection_region_cap={}",
        effective_max_selected_per_region
    ));
    explain.push(format!("region_cap_rejections={region_cap_rejections}"));
    if effective_prefer_region_diversity {
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
    let distinct_region_ratio_pct = if selected_peers.is_empty() {
        0
    } else {
        selected_regions.len().saturating_mul(100) / selected_peers.len()
    };
    let min_distinct_regions_met = selected_regions.len() >= effective_min_distinct_regions;
    let distinct_region_deficit =
        effective_min_distinct_regions.saturating_sub(selected_regions.len());
    explain.push(format!(
        "candidate_distinct_regions={}",
        candidate_distinct_regions.len()
    ));
    explain.push(format!(
        "min_distinct_regions_feasible={min_distinct_regions_feasible}"
    ));
    explain.push(format!(
        "min_distinct_regions_feasibility_gap={min_distinct_regions_feasibility_gap}"
    ));
    explain.push(format!(
        "min_distinct_regions_target={}",
        effective_min_distinct_regions
    ));
    explain.push(format!(
        "min_distinct_regions_met={min_distinct_regions_met}"
    ));
    explain.push(format!("distinct_region_deficit={distinct_region_deficit}"));
    explain.push(format!(
        "distinct_region_ratio_pct={distinct_region_ratio_pct}"
    ));
    append_selection_explain(
        runtime,
        policy,
        &selected_peers,
        SelectionExplainInput {
            stats,
            region_cap_rejections,
            effective_max_peers,
        },
        explain,
    );
    Ok(selected_peers)
}
