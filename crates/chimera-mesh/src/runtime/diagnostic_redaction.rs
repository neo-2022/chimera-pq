use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt::Write as _;

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
    let mut labels = String::new();
    for peer_id in peer_ids {
        if !labels.is_empty() {
            labels.push(',');
        }
        append_peer_label_from_table(&mut labels, peer_id, peers);
    }
    labels
}

fn append_peer_label_from_table(
    output: &mut String,
    peer_id: &str,
    peers: &BTreeMap<String, MeshPeerState>,
) {
    if peer_id == "none" {
        output.push_str("none");
        return;
    }
    match peers.keys().position(|node_id| node_id == peer_id) {
        Some(index) => {
            output.push_str("peer#");
            let _ = write!(output, "{}", index + 1);
        }
        None => output.push_str("<redacted>"),
    }
}
