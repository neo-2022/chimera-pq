use crate::model::MeshPeerState;

pub(crate) fn standby_target_for_multipath_mode(
    mode: Option<&str>,
    switch_target: &str,
    selected_peers: &[MeshPeerState],
) -> (String, &'static str) {
    let primary = selected_peers
        .first()
        .map(|peer| peer.node_id.as_str())
        .unwrap_or("");
    let secondary = selected_peers
        .get(1)
        .map(|peer| peer.node_id.as_str())
        .unwrap_or("");
    match mode {
        Some("off") | Some("standby_only") => {
            if switch_target != "none" {
                (switch_target.to_string(), "switch_target")
            } else if !primary.is_empty() {
                (primary.to_string(), "selected_primary")
            } else {
                ("none".to_string(), "none")
            }
        }
        Some("flow_shard") | Some("aggregate_buffered") => {
            if !secondary.is_empty() {
                (secondary.to_string(), "selected_secondary")
            } else if switch_target != "none" {
                (switch_target.to_string(), "switch_target")
            } else if !primary.is_empty() {
                (primary.to_string(), "selected_primary")
            } else {
                ("none".to_string(), "none")
            }
        }
        _ => {
            if switch_target != "none" {
                (switch_target.to_string(), "switch_target")
            } else if !secondary.is_empty() {
                (secondary.to_string(), "selected_secondary")
            } else if !primary.is_empty() {
                (primary.to_string(), "selected_primary")
            } else {
                ("none".to_string(), "none")
            }
        }
    }
}
