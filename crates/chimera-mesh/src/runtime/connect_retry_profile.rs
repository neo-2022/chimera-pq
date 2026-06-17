use crate::model::MeshPeerState;

const BACKOFF_INITIAL_MS: u64 = 0;
const BACKOFF_RETRY1_MS: u64 = 250;
const BACKOFF_RETRY2_MS: u64 = 1000;
const RETRY_JITTER_STEP_MS: u64 = 50;

pub(crate) fn build_connect_priority(selected_peers: &[MeshPeerState]) -> String {
    selected_peers
        .iter()
        .enumerate()
        .map(|(idx, _)| format!("{}:peer#{}@<redacted>", idx + 1, idx + 1))
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn build_connect_retry_plan(
    selected_peers: &[MeshPeerState],
    fallback_ports: &[u16],
) -> String {
    selected_peers
        .iter()
        .enumerate()
        .map(|(idx, _)| {
            let fallback_port_state = if fallback_ports.is_empty() {
                "none"
            } else {
                "configured"
            };
            let fallback = selected_peers
                .get(idx.saturating_add(1))
                .map(|_| format!(";fallback:peer#{}@<redacted>", idx + 2))
                .unwrap_or_default();
            format!(
                "peer#{}@<redacted>:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=<redacted>;fallback_ports={}{}",
                idx + 1,
                fallback_port_state,
                fallback
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn build_connect_backoff_profile(selected_peer_count: usize) -> String {
    format!(
        "initial={}ms;retry1={}ms;retry2={}ms;jitter_step={}ms;fanout={}",
        BACKOFF_INITIAL_MS,
        BACKOFF_RETRY1_MS,
        BACKOFF_RETRY2_MS,
        RETRY_JITTER_STEP_MS,
        selected_peer_count
    )
}

#[cfg(test)]
mod tests {
    use super::build_connect_retry_plan;
    use crate::model::MeshPeerState;

    fn peer(node: &str, endpoint: &str) -> MeshPeerState {
        MeshPeerState {
            node_id: node.to_string(),
            endpoint: endpoint.to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 90,
            selection_score: 180,
        }
    }

    #[test]
    fn retry_plan_redacts_ports_and_keeps_next_peer_chain() {
        let peers = vec![
            peer("node-a", "198.51.100.10:9443"),
            peer("node-b", "198.51.100.11:443"),
        ];
        let plan = build_connect_retry_plan(&peers, &[443, 8443]);
        assert!(plan.contains(
            "peer#1@<redacted>:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=<redacted>;fallback_ports=configured;fallback:peer#2@<redacted>"
        ));
        assert!(plan.contains(
            "peer#2@<redacted>:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=<redacted>;fallback_ports=configured"
        ));
        assert!(!plan.contains("9443"));
        assert!(!plan.contains("443"));
        assert!(!plan.contains("8443"));
        assert!(!plan.contains("node-a"));
        assert!(!plan.contains("node-b"));
        assert!(!plan.contains("198.51.100.10"));
        assert!(!plan.contains("198.51.100.11"));
    }
}
