#[cfg(test)]
use crate::model::MeshPeerState;
#[cfg(test)]
use std::fmt::Write;

const BACKOFF_PROFILE_PREFIX: &str =
    "initial=0ms;retry1=250ms;retry2=1000ms;jitter_step=50ms;fanout=";

#[cfg(test)]
pub(crate) fn build_connect_priority(selected_peers: &[MeshPeerState]) -> String {
    let mut out = String::with_capacity(selected_peers.len().saturating_mul(24));
    for (idx, _) in selected_peers.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        let _ = write!(out, "{}:peer#{}@<redacted>", idx + 1, idx + 1);
    }
    out
}

#[cfg(test)]
pub(crate) fn build_connect_retry_plan(
    selected_peers: &[MeshPeerState],
    fallback_ports: &[u16],
) -> String {
    let mut out = String::with_capacity(selected_peers.len().saturating_mul(96));
    let fallback_port_state = if fallback_ports.is_empty() {
        "none"
    } else {
        "configured"
    };
    for (idx, _) in selected_peers.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "peer#{}@<redacted>:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=<redacted>;fallback_ports={}",
            idx + 1,
            fallback_port_state
        );
        if idx + 1 < selected_peers.len() {
            out.push_str(";fallback:peer#");
            let _ = write!(out, "{}", idx + 2);
            out.push_str("@<redacted>");
        }
    }
    out
}

pub(crate) fn build_connect_backoff_profile(selected_peer_count: usize) -> String {
    let mut out = String::with_capacity(64);
    out.push_str(BACKOFF_PROFILE_PREFIX);
    push_usize_decimal(&mut out, selected_peer_count);
    out
}

fn push_usize_decimal(out: &mut String, mut value: usize) {
    if value == 0 {
        out.push('0');
        return;
    }

    let mut digits = [0u8; 20];
    let mut len = 0usize;
    while value > 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    for digit in digits[..len].iter().rev() {
        out.push(*digit as char);
    }
}

#[cfg(test)]
mod tests {
    use super::{build_connect_backoff_profile, build_connect_priority, build_connect_retry_plan};
    use crate::model::MeshPeerState;

    fn peer(node: &str, endpoint: &str) -> MeshPeerState {
        MeshPeerState {
            node_id: node.to_string(),
            endpoint: endpoint.to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 90,
            latency_ms: None,
            throughput_mbps: None,
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

    #[test]
    fn connect_priority_redacts_and_preserves_order() {
        let peers = vec![
            peer("node-a", "198.51.100.10:9443"),
            peer("node-b", "198.51.100.11:443"),
        ];
        let priority = build_connect_priority(&peers);
        assert_eq!(priority, "1:peer#1@<redacted>,2:peer#2@<redacted>");
        assert!(!priority.contains("node-a"));
        assert!(!priority.contains("node-b"));
        assert!(!priority.contains("198.51.100.10"));
        assert!(!priority.contains("198.51.100.11"));
    }

    #[test]
    fn backoff_profile_preserves_exact_format_for_fanout_values() {
        assert_eq!(
            build_connect_backoff_profile(0),
            "initial=0ms;retry1=250ms;retry2=1000ms;jitter_step=50ms;fanout=0"
        );
        assert_eq!(
            build_connect_backoff_profile(1),
            "initial=0ms;retry1=250ms;retry2=1000ms;jitter_step=50ms;fanout=1"
        );
        assert_eq!(
            build_connect_backoff_profile(9),
            "initial=0ms;retry1=250ms;retry2=1000ms;jitter_step=50ms;fanout=9"
        );
        assert_eq!(
            build_connect_backoff_profile(10),
            "initial=0ms;retry1=250ms;retry2=1000ms;jitter_step=50ms;fanout=10"
        );
        assert_eq!(
            build_connect_backoff_profile(100000),
            "initial=0ms;retry1=250ms;retry2=1000ms;jitter_step=50ms;fanout=100000"
        );
    }
}
