use super::preemptive_helpers::{format_hints_summary_with_source, hints_source_from_status};
use super::status_shadow_snapshot::ShadowStatusSnapshot;

pub(super) struct StatusHintsSummary {
    pub(super) source: String,
    pub(super) summary: String,
}

pub(super) fn build_status_hints_summary(snapshot: &ShadowStatusSnapshot) -> StatusHintsSummary {
    let source = hints_source_from_status(&snapshot.hints_status).to_string();
    let summary = format_hints_summary_with_source(
        &snapshot.hints_status,
        snapshot.hints_present,
        &snapshot.hints_reason,
        &snapshot.hints_multipath_mode,
        &snapshot.hints_continuity_policy,
    );
    StatusHintsSummary { source, summary }
}
