use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::model::MeshPeerState;

pub(super) fn peer_label_from_table(
    peer_id: &str,
    peers: &BTreeMap<String, MeshPeerState>,
) -> String {
    if peer_id == "none" {
        return "none".to_string();
    }
    peers
        .keys()
        .position(|node_id| node_id == peer_id)
        .map(|index| format!("peer#{}", index + 1))
        .unwrap_or_else(|| "<redacted>".to_string())
}

pub(super) fn peer_labels_from_table(
    peer_ids: &BTreeSet<String>,
    peers: &BTreeMap<String, MeshPeerState>,
) -> String {
    if peer_ids.is_empty() {
        return "none".to_string();
    }
    peer_ids
        .iter()
        .map(|peer_id| peer_label_from_table(peer_id, peers))
        .collect::<Vec<_>>()
        .join(",")
}
