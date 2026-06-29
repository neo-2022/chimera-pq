use super::path_planner_selection_explain::{SelectionExplainInput, append_selection_explain};
use super::selection_policy::{select_by_score_with_region_cap, select_with_region_diversity};
use super::*;
use std::collections::HashSet;

pub(super) struct SelectionFinalizeInput<'a> {
    pub(super) policy: &'a MeshPathPolicy,
    pub(super) stats: CandidateStats,
    pub(super) candidates: Vec<CandidateSlot<'a>>,
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
    explain.reserve(52);
    candidates.sort_by(|a, b| {
        b.selection_score
            .cmp(&a.selection_score)
            .then_with(|| a.peer.load_score.cmp(&b.peer.load_score))
            .then_with(|| b.peer.reliability_score.cmp(&a.peer.reliability_score))
            .then_with(|| a.peer.node_id.cmp(&b.peer.node_id))
    });
    let candidate_distinct_region_count = distinct_region_count(
        candidates
            .iter()
            .map(|candidate| candidate.normalized_region.as_str()),
        candidates.len(),
    );
    let min_distinct_regions_feasible =
        candidate_distinct_region_count >= policy.min_distinct_regions;
    let min_distinct_regions_feasibility_gap = policy
        .min_distinct_regions
        .saturating_sub(candidate_distinct_region_count);

    let (selected_slots, region_cap_rejections) = if effective_prefer_region_diversity {
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
    let mut selected_peers = Vec::with_capacity(selected_slots.len());
    for selected_slot in &selected_slots {
        selected_peers.push(selected_slot.materialize_peer());
    }
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
    let selected_region_count = distinct_region_count(
        selected_peers.iter().map(|peer| peer.region.as_str()),
        selected_peers.len(),
    );
    explain.push(format!("selected_regions={}", selected_region_count));
    let distinct_region_ratio_pct = if selected_peers.is_empty() {
        0
    } else {
        selected_region_count.saturating_mul(100) / selected_peers.len()
    };
    let min_distinct_regions_met = selected_region_count >= effective_min_distinct_regions;
    let distinct_region_deficit =
        effective_min_distinct_regions.saturating_sub(selected_region_count);
    explain.push(format!(
        "candidate_distinct_regions={}",
        candidate_distinct_region_count
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

fn distinct_region_count<'a, I>(regions: I, capacity: usize) -> usize
where
    I: IntoIterator<Item = &'a str>,
{
    let mut distinct_regions: HashSet<&'a str> = HashSet::with_capacity(capacity);
    for region in regions {
        distinct_regions.insert(region);
    }
    distinct_regions.len()
}
