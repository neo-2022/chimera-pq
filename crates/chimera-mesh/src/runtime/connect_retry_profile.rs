use crate::model::MeshPeerState;

const BACKOFF_INITIAL_MS: u64 = 0;
const BACKOFF_RETRY1_MS: u64 = 250;
const BACKOFF_RETRY2_MS: u64 = 1000;
const RETRY_JITTER_STEP_MS: u64 = 50;

pub(crate) fn build_connect_priority(selected_peers: &[MeshPeerState]) -> String {
    selected_peers
        .iter()
        .enumerate()
        .map(|(idx, peer)| format!("{}:{}@{}", idx + 1, peer.node_id, peer.endpoint))
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
        .map(|(idx, peer)| {
            let fallback_ports = endpoint_fallback_ports(&peer.endpoint, fallback_ports);
            let fallback = selected_peers
                .get(idx.saturating_add(1))
                .map(|next| format!(";fallback:{}@{}", next.node_id, next.endpoint))
                .unwrap_or_default();
            format!(
                "{}@{}:try0(connect)|try1(retry_fast)|try2(retry_slow);ports={}{}",
                peer.node_id, peer.endpoint, fallback_ports, fallback
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

fn endpoint_fallback_ports(endpoint: &str, fallback_ports: &[u16]) -> String {
    let current_port = endpoint_port(endpoint);
    let mut ports = Vec::new();
    if let Some(port) = current_port {
        ports.push(port);
    }
    for port in fallback_ports {
        let port = *port;
        if !ports.contains(&port) {
            ports.push(port);
        }
    }
    ports
        .into_iter()
        .map(|port| port.to_string())
        .collect::<Vec<_>>()
        .join("|")
}

fn endpoint_port(endpoint: &str) -> Option<u16> {
    let port_raw = if endpoint.starts_with('[') {
        let close = endpoint.find(']')?;
        let tail = endpoint.get((close + 1)..)?;
        tail.strip_prefix(':')?
    } else {
        endpoint.rsplit_once(':')?.1
    };
    port_raw.parse::<u16>().ok().filter(|port| *port > 0)
}

#[cfg(test)]
mod tests {
    use super::{build_connect_retry_plan, endpoint_fallback_ports};
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
    fn fallback_ports_preserve_current_port_and_add_known_fallbacks() {
        let ports = [443, 8443];
        assert_eq!(
            endpoint_fallback_ports("198.51.100.10:9443", &ports),
            "9443|443|8443"
        );
        assert_eq!(
            endpoint_fallback_ports("198.51.100.10:443", &ports),
            "443|8443"
        );
        assert_eq!(
            endpoint_fallback_ports("[2001:db8::10]:8443", &ports),
            "8443|443"
        );
    }

    #[test]
    fn retry_plan_includes_port_fallbacks_and_next_peer_chain() {
        let peers = vec![
            peer("node-a", "198.51.100.10:9443"),
            peer("node-b", "198.51.100.11:443"),
        ];
        let plan = build_connect_retry_plan(&peers, &[443, 8443]);
        assert!(plan.contains(
            "node-a@198.51.100.10:9443:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=9443|443|8443;fallback:node-b@198.51.100.11:443"
        ));
        assert!(plan.contains(
            "node-b@198.51.100.11:443:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=443|8443"
        ));
    }
}
