use super::path_planner_selection_metrics::build_selection_metrics;
use super::*;
#[path = "path_planner_selection_explain_sections.rs"]
mod sections;

pub(super) struct SelectionExplainInput {
    pub(super) stats: CandidateStats,
    pub(super) region_cap_rejections: usize,
    pub(super) effective_max_peers: usize,
}

pub(super) fn append_selection_explain(
    runtime: &MeshRuntime,
    policy: &MeshPathPolicy,
    selected_peers: &[MeshPeerState],
    input: SelectionExplainInput,
    explain: &mut Vec<String>,
) {
    let metrics = build_selection_metrics(
        runtime,
        policy,
        selected_peers,
        input.stats,
        input.region_cap_rejections,
        input.effective_max_peers,
    );
    sections::append_selected_peer_lines(explain, &metrics);
    sections::append_stability_lines(explain, &metrics);
    sections::append_candidate_lines(explain, &metrics, &input);
}
