use super::*;
use std::fmt::Write as _;
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
        let mut connected_endpoint_raw = String::new();

        let mut attempt_index = 0usize;
        let connected = visit_connect_attempt_targets(selected_peers, fallback_ports, |target| {
            let peer_label = redacted_peer_label(target.peer_index);
            let endpoint_label = redacted_endpoint_label(attempt_index);
            attempt_index = attempt_index.saturating_add(1);
            let started = Instant::now();
            match connect_endpoint(target.endpoint, timeout) {
                Ok(()) => {
                    let latency_ms = duration_to_latency_ms(started.elapsed());
                    if let Err(error) = self.update_peer_performance(&[MeshPeerPerformance {
                        node_id: target.peer_id.to_string(),
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
                    connected_endpoint_raw = target.endpoint.to_string();
                    explain.push("connect_probe_result=connected".to_string());
                    explain.push(format!("connect_probe_connected_peer={connected_peer}"));
                    explain.push(format!(
                        "connect_probe_connected_endpoint={connected_endpoint}"
                    ));
                    true
                }
                Err(error) => {
                    attempts.push(MeshConnectAttempt {
                        peer_id: peer_label,
                        endpoint: endpoint_label,
                        success: false,
                        error: redacted_connect_error(&error),
                    });
                    false
                }
            }
        })
        .map_err(|error| format!("connect_probe_attempt_plan_error:{error}"))?;

        if connected {
            return Ok(MeshConnectProbeReport {
                namespace: self.namespace.clone(),
                selected_peers: redacted_peer_labels(selected_peers),
                connected_peer,
                connected_endpoint,
                connected_endpoint_raw,
                success: true,
                attempts,
                explain,
            });
        }

        explain.push("connect_probe_result=failed".to_string());
        Ok(MeshConnectProbeReport {
            namespace: self.namespace.clone(),
            selected_peers: redacted_peer_labels(selected_peers),
            connected_peer,
            connected_endpoint,
            connected_endpoint_raw,
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
        redacted_peer_label_csv(selected_peers)
    ));
    explain.push(format!(
        "selected_peer_endpoints={}",
        redacted_endpoint_label_csv(selected_peers)
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

#[cfg(test)]
struct MeshConnectAttemptTarget {
    peer_index: usize,
    peer_id: String,
    endpoint: String,
}

struct MeshConnectAttemptTargetRef<'a> {
    peer_index: usize,
    peer_id: &'a str,
    endpoint: &'a str,
}

#[cfg(test)]
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

fn visit_connect_attempt_targets(
    selected_peers: &[MeshPeerState],
    fallback_ports: &[u16],
    mut visitor: impl FnMut(MeshConnectAttemptTargetRef<'_>) -> bool,
) -> Result<bool, String> {
    for (peer_index, peer) in selected_peers.iter().enumerate() {
        let (host, current_port) = split_host_port(&peer.endpoint)?;
        if visitor(MeshConnectAttemptTargetRef {
            peer_index,
            peer_id: &peer.node_id,
            endpoint: &peer.endpoint,
        }) {
            return Ok(true);
        }
        for (fallback_index, port) in fallback_ports.iter().enumerate() {
            if fallback_port_already_attempted(fallback_ports, fallback_index, current_port, *port)
            {
                continue;
            }
            let endpoint = format_endpoint(host, *port);
            if visitor(MeshConnectAttemptTargetRef {
                peer_index,
                peer_id: &peer.node_id,
                endpoint: &endpoint,
            }) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn fallback_port_already_attempted(
    fallback_ports: &[u16],
    fallback_index: usize,
    current_port: u16,
    port: u16,
) -> bool {
    port == 0 || port == current_port || fallback_ports[..fallback_index].contains(&port)
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

#[cfg(test)]
fn fallback_endpoints_for_peer(
    peer: &MeshPeerState,
    fallback_ports: &[u16],
) -> Result<Vec<String>, String> {
    let (host, current_port) = split_host_port(&peer.endpoint)?;
    let mut ports = Vec::new();
    ports.push(current_port);
    for port in fallback_ports {
        if *port > 0 && !ports.contains(port) {
            ports.push(*port);
        }
    }
    Ok(ports
        .into_iter()
        .map(|port| format_endpoint(host, port))
        .collect())
}

fn split_host_port(endpoint: &str) -> Result<(&str, u16), String> {
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
        return Ok((host, port));
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
    Ok((host, port))
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

fn redacted_peer_label_csv(selected_peers: &[MeshPeerState]) -> String {
    let mut labels = String::with_capacity(selected_peers.len().saturating_mul(8));
    for index in 0..selected_peers.len() {
        if index > 0 {
            labels.push(',');
        }
        append_redacted_peer_label(&mut labels, index);
    }
    labels
}

fn redacted_endpoint_label_csv(selected_peers: &[MeshPeerState]) -> String {
    let mut labels = String::with_capacity(selected_peers.len().saturating_mul(22));
    for index in 0..selected_peers.len() {
        if index > 0 {
            labels.push(',');
        }
        append_redacted_endpoint_label(&mut labels, index);
    }
    labels
}

fn redacted_peer_label(index: usize) -> String {
    format!("peer#{}", index + 1)
}

fn redacted_endpoint_label(index: usize) -> String {
    format!("endpoint#{}:<redacted>", index + 1)
}

fn append_redacted_peer_label(output: &mut String, index: usize) {
    output.push_str("peer#");
    let _ = write!(output, "{}", index + 1);
}

fn append_redacted_endpoint_label(output: &mut String, index: usize) {
    output.push_str("endpoint#");
    let _ = write!(output, "{}", index + 1);
    output.push_str(":<redacted>");
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
