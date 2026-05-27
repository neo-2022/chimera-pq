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
    render::append_standby_shadow_explain(selected_peers, explain);
}

pub(super) fn adapt_standby_shadow_from_dps(explain: &mut Vec<String>) {
    adapt::adapt_standby_shadow_from_dps(explain);
}
