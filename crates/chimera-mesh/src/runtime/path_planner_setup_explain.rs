use super::*;

#[path = "path_planner_setup_explain_discovery.rs"]
mod discovery;
#[path = "path_planner_setup_explain_preface.rs"]
mod preface;

pub(super) struct PlanSetupPrefaceExplainContext<'a> {
    pub(super) manual_overrides: &'a [&'a str],
    pub(super) path_profile: MeshPathProfile,
    pub(super) path_profile_reason: &'static str,
    pub(super) path_profile_overridden: bool,
    pub(super) avg_load_score: u8,
    pub(super) avg_reliability_score: u8,
}

pub(super) fn append_plan_setup_preface_explain(
    runtime: &MeshRuntime,
    explain: &mut Vec<String>,
    ctx: &PlanSetupPrefaceExplainContext<'_>,
) {
    preface::append_plan_setup_preface_explain(runtime, explain, ctx);
}

pub(super) fn append_plan_setup_discovery_table_explain(
    runtime: &MeshRuntime,
    explain: &mut Vec<String>,
    join_mode: MeshJoinMode,
) {
    discovery::append_plan_setup_discovery_table_explain(runtime, explain, join_mode);
}
