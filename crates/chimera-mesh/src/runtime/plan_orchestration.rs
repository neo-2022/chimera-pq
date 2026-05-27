use super::plan_policy::{
    EffectiveSelectionPolicy, PlanPathFilterSets, build_effective_selection_policy,
    build_plan_path_filter_sets, select_health_blocked_filter,
};
use super::*;

#[path = "plan_orchestration_discovery_table.rs"]
mod discovery_table;
#[path = "plan_orchestration_preface.rs"]
mod preface;
#[path = "plan_orchestration_preemptive_shadow.rs"]
mod preemptive_shadow;

pub(super) struct PlanPathPrefaceContext<'a> {
    pub(super) decision_control_mode: &'a str,
    pub(super) manual_override_count: usize,
    pub(super) manual_override_fields: String,
    pub(super) path_profile: MeshPathProfile,
    pub(super) path_profile_source: &'a str,
    pub(super) path_profile_reason: &'a str,
    pub(super) avg_load_score: u8,
    pub(super) avg_reliability_score: u8,
}

pub(super) struct PlanPathDiscoveryTableContext {
    pub(super) join_mode: MeshJoinMode,
    pub(super) source_count: usize,
    pub(super) source_names: String,
}

pub(super) struct PlanPathPreemptiveContext<'a> {
    pub(super) path_profile: MeshPathProfile,
    pub(super) avg_load_score: u8,
    pub(super) avg_reliability_score: u8,
    pub(super) peers: &'a BTreeMap<String, MeshPeerState>,
    pub(super) health_state: &'a BTreeMap<String, MeshHealthMeta>,
    pub(super) peer_meta: &'a BTreeMap<String, MeshPeerMeta>,
    pub(super) table_policy: &'a MeshPeerTablePolicy,
    pub(super) tick: u64,
}

pub(super) fn append_plan_path_preface_explain(
    explain: &mut Vec<String>,
    context: &PlanPathPrefaceContext<'_>,
) {
    preface::append_plan_path_preface_explain(explain, context);
}

pub(super) fn append_plan_path_discovery_table_explain(
    explain: &mut Vec<String>,
    context: &PlanPathDiscoveryTableContext,
    table_policy: &MeshPeerTablePolicy,
    table_report: &MeshPeerTableEnforcementReport,
) {
    discovery_table::append_plan_path_discovery_table_explain(
        explain,
        context,
        table_policy,
        table_report,
    );
}

pub(super) fn append_plan_path_preemptive_shadow_explain(
    explain: &mut Vec<String>,
    context: &PlanPathPreemptiveContext<'_>,
    pri_tuning: &ShadowPriTuning,
) {
    preemptive_shadow::append_plan_path_preemptive_shadow_explain(explain, context, pri_tuning);
}
