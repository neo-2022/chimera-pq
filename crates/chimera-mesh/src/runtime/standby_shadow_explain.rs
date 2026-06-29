use super::*;

#[path = "standby_shadow_explain_adapt.rs"]
mod adapt;
#[path = "standby_shadow_explain_common.rs"]
mod common;
#[path = "standby_shadow_explain_render.rs"]
mod render;

pub(super) fn append_standby_shadow_explain(
    selected_peers: &[MeshPeerState],
    explain: &mut Vec<String>,
) {
    let snapshot = common::StandbyShadowExplainSnapshot::capture(explain);
    render::append_standby_shadow_explain(selected_peers, explain, &snapshot);
}

pub(super) fn adapt_standby_shadow_from_dps(
    selected_peers: &[MeshPeerState],
    explain: &mut Vec<String>,
    dps_multipath_mode: Option<&str>,
) {
    let snapshot = common::StandbyShadowExplainSnapshot::capture(explain);
    adapt::adapt_standby_shadow_from_dps(selected_peers, explain, &snapshot, dps_multipath_mode);
}
