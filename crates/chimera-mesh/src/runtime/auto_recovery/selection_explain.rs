use super::*;

#[path = "selection_explain_strategy.rs"]
mod strategy;
#[path = "selection_explain_selected.rs"]
mod selected;
#[path = "selection_explain_stability.rs"]
mod stability;
#[path = "selection_explain_counters.rs"]
mod counters;

pub(crate) use counters::{append_selection_counters_explain, format_candidate_summary};
pub(crate) use selected::{
    append_selected_peer_aggregate_explain, append_selected_peer_identity_explain,
    append_selected_peer_metrics_explain,
};
pub(crate) use stability::{
    append_selected_stability_counters_explain, append_selected_stability_identity_explain,
    append_selected_stability_metrics_explain,
};
pub(crate) use strategy::{
    append_region_selection_diagnostics_explain, append_selection_feasibility_explain,
    append_selection_region_diagnostics_explain, append_selection_strategy_explain,
    append_spread_bonus_explain,
};
