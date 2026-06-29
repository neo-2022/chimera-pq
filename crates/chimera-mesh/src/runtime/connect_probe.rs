use super::*;
use std::collections::BTreeSet;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

impl MeshRuntime {
    pub fn connect_probe(
        &mut self,
        request: &MeshJoinRequest,
        policy: &MeshPathPolicy,
        timeout_ms: u64,
    ) -> Result<MeshConnectProbeReport, String> {
        let plan = self.plan_path_core(request, policy)?;
        self.connect_probe_from_selected_peers(
            &plan.selected_peers,
            &policy.connect_fallback_ports,
            timeout_ms,
        )
    }

    pub fn connect_probe_from_dps_payload(
        &mut self,
        request: &MeshJoinRequest,
        payload: &str,
        timeout_ms: u64,
    ) -> Result<MeshConnectProbeReport, String> {
        super::payload_utils::ensure_mesh_payload_nonempty(payload)?;
        let plan = self.plan_path_core_from_dps_payload(request, payload)?;
        let policy = MeshPathPolicy::from_dps_payload(payload)?;
        self.connect_probe_from_selected_peers(
            &plan.selected_peers,
            &policy.connect_fallback_ports,
            timeout_ms,
        )
    }
}

impl MeshRuntime {
    fn connect_probe_from_selected_peers(
        &mut self,
        selected_peers: &[MeshPeerState],
        fallback_ports: &[u16],
        timeout_ms: u64,
    ) -> Result<MeshConnectProbeReport, String> {
        let mut attempts = Vec::new();
        let mut explain = build_connect_probe_explain(selected_peers, fallback_ports);
        let timeout = Duration::from_millis(timeout_ms.max(1));
        let mut connected_peer = String::new();
        let mut connected_endpoint = String::new();

        let attempt_plan = build_connect_attempt_plan(selected_peers, fallback_ports)?;
        for (attempt_index, target) in attempt_plan.iter().enumerate() {
            let peer_label = redacted_peer_label(target.peer_index);
            let endpoint_label = redacted_endpoint_label(attempt_index);
            let started = Instant::now();
            match connect_endpoint(&target.endpoint, timeout) {
                Ok(()) => {
                    let latency_ms = duration_to_latency_ms(started.elapsed());
                    if let Err(error) = self.update_peer_performance(&[MeshPeerPerformance {
                        node_id: target.peer_id.clone(),
                        latency_ms: Some(latency_ms),
                        throughput_mbps: None,
                    }]) {
                        explain.push(format!("connect_probe_performance_update=failed:{error}"));
                    }
                    attempts.push(MeshConnectAttempt {
                        peer_id: peer_label.clone(),
                        endpoint: endpoint_label.clone(),
                        success: true,
                        error: String::new(),
                    });
                    connected_peer = peer_label;
                    connected_endpoint = endpoint_label;
                    explain.push("connect_probe_result=connected".to_string());
                    explain.push(format!("connect_probe_connected_peer={connected_peer}"));
                    explain.push(format!(
                        "connect_probe_connected_endpoint={connected_endpoint}"
                    ));
                    return Ok(MeshConnectProbeReport {
                        namespace: self.namespace.clone(),
                        selected_peers: redacted_peer_labels(selected_peers),
                        connected_peer,
                        connected_endpoint,
                        success: true,
                        attempts,
                        explain,
                    });
                }
                Err(error) => {
                    attempts.push(MeshConnectAttempt {
                        peer_id: peer_label,
                        endpoint: endpoint_label,
                        success: false,
                        error: redacted_connect_error(&error),
                    });
                }
            }
        }

        explain.push("connect_probe_result=failed".to_string());
        Ok(MeshConnectProbeReport {
            namespace: self.namespace.clone(),
            selected_peers: redacted_peer_labels(selected_peers),
            connected_peer,
            connected_endpoint,
            success: false,
            attempts,
            explain,
        })
    }
}

fn build_connect_probe_explain(
    selected_peers: &[MeshPeerState],
    fallback_ports: &[u16],
) -> Vec<String> {
    let mut explain = Vec::with_capacity(8);
    if selected_peers.is_empty() {
        explain.push("selected_peer_ids=none".to_string());
        explain.push("selected_peer_endpoints=none".to_string());
        explain.push("selected_peer_connect_priority=none".to_string());
        explain.push("selected_peer_connect_retry_plan=none".to_string());
        explain.push("selected_peer_connect_backoff_profile=initial=0ms;retry1=250ms;retry2=1000ms;jitter_step=50ms;fanout=0".to_string());
        return explain;
    }
    explain.push(format!(
        "selected_peer_ids={}",
        redacted_peer_labels(selected_peers).join(",")
    ));
    explain.push(format!(
        "selected_peer_endpoints={}",
        redacted_endpoint_labels(selected_peers).join(",")
    ));
    explain.push(format!(
        "selected_peer_connect_priority={}",
        super::connect_retry_profile::build_connect_priority(selected_peers)
    ));
    explain.push(format!(
        "selected_peer_connect_retry_plan={}",
        super::connect_retry_profile::build_connect_retry_plan(selected_peers, fallback_ports)
    ));
    explain.push(format!(
        "selected_peer_connect_backoff_profile={}",
        super::connect_retry_profile::build_connect_backoff_profile(selected_peers.len())
    ));
    explain
}

struct MeshConnectAttemptTarget {
    peer_index: usize,
    peer_id: String,
    endpoint: String,
}

fn build_connect_attempt_plan(
    selected_peers: &[MeshPeerState],
    fallback_ports: &[u16],
) -> Result<Vec<MeshConnectAttemptTarget>, String> {
    let candidate_count = selected_peers
        .len()
        .saturating_mul(fallback_ports.len().saturating_add(1));
    let mut targets = Vec::with_capacity(candidate_count);
    for (peer_index, peer) in selected_peers.iter().enumerate() {
        for endpoint in fallback_endpoints_for_peer(peer, fallback_ports)? {
            targets.push(MeshConnectAttemptTarget {
                peer_index,
                peer_id: peer.node_id.clone(),
                endpoint,
            });
        }
    }
    Ok(targets)
}

fn connect_endpoint(endpoint: &str, timeout: Duration) -> Result<(), String> {
    let addrs = endpoint
        .to_socket_addrs()
        .map_err(|e| format!("resolve_error:{e}"))?;
    let socket_addrs: Vec<SocketAddr> = addrs.collect();
    if socket_addrs.is_empty() {
        return Err("resolve_error:no_socket_addrs".to_string());
    }
    let mut last_error = String::new();
    for addr in socket_addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(_) => return Ok(()),
            Err(e) => last_error = format!("connect_error:{e}"),
        }
    }
    if last_error.is_empty() {
        Err("connect_error:unknown".to_string())
    } else {
        Err(last_error)
    }
}

fn duration_to_latency_ms(duration: Duration) -> u32 {
    duration.as_millis().min(u128::from(u32::MAX)).max(1) as u32
}

fn fallback_endpoints_for_peer(
    peer: &MeshPeerState,
    fallback_ports: &[u16],
) -> Result<Vec<String>, String> {
    let (host, current_port) = split_host_port(&peer.endpoint)?;
    let mut ports = Vec::new();
    let mut seen = BTreeSet::new();
    if seen.insert(current_port) {
        ports.push(current_port);
    }
    for port in fallback_ports {
        if *port > 0 && seen.insert(*port) {
            ports.push(*port);
        }
    }
    Ok(ports
        .into_iter()
        .map(|port| format_endpoint(&host, port))
        .collect())
}

fn split_host_port(endpoint: &str) -> Result<(String, u16), String> {
    if endpoint.starts_with('[') {
        let close = endpoint
            .find(']')
            .ok_or_else(|| "invalid_endpoint:missing_ipv6_bracket".to_string())?;
        let host = endpoint
            .get(1..close)
            .ok_or_else(|| "invalid_endpoint:host_slice".to_string())?;
        let tail = endpoint
            .get((close + 1)..)
            .ok_or_else(|| "invalid_endpoint:tail_slice".to_string())?;
        let port_raw = tail
            .strip_prefix(':')
            .ok_or_else(|| "invalid_endpoint:missing_port".to_string())?;
        let port = port_raw
            .parse::<u16>()
            .map_err(|_| "invalid_endpoint:bad_port".to_string())?;
        if port == 0 {
            return Err("invalid_endpoint:zero_port".to_string());
        }
        return Ok((host.to_string(), port));
    }
    let (host, port_raw) = endpoint
        .rsplit_once(':')
        .ok_or_else(|| "invalid_endpoint:missing_host_port_sep".to_string())?;
    if host.contains(':') {
        return Err("invalid_endpoint:ipv6_must_be_bracketed".to_string());
    }
    let port = port_raw
        .parse::<u16>()
        .map_err(|_| "invalid_endpoint:bad_port".to_string())?;
    if port == 0 {
        return Err("invalid_endpoint:zero_port".to_string());
    }
    Ok((host.to_string(), port))
}

fn format_endpoint(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn redacted_peer_labels(selected_peers: &[MeshPeerState]) -> Vec<String> {
    selected_peers
        .iter()
        .enumerate()
        .map(|(index, _)| redacted_peer_label(index))
        .collect()
}

fn redacted_endpoint_labels(selected_peers: &[MeshPeerState]) -> Vec<String> {
    selected_peers
        .iter()
        .enumerate()
        .map(|(index, _)| redacted_endpoint_label(index))
        .collect()
}

fn redacted_peer_label(index: usize) -> String {
    format!("peer#{}", index + 1)
}

fn redacted_endpoint_label(index: usize) -> String {
    format!("endpoint#{}:<redacted>", index + 1)
}

fn redacted_connect_error(error: &str) -> String {
    if error.starts_with("resolve_error:") {
        "resolve_error".to_string()
    } else if error.starts_with("connect_error:") {
        "connect_error".to_string()
    } else if error.starts_with("invalid_endpoint:") {
        error.to_string()
    } else {
        "connect_error".to_string()
    }
}

#[cfg(test)]
#[path = "connect_probe_tests.rs"]
mod tests;
