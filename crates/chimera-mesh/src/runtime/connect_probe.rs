use super::*;
use std::collections::BTreeSet;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

impl MeshRuntime {
    pub fn connect_probe(
        &self,
        request: &MeshJoinRequest,
        policy: &MeshPathPolicy,
        timeout_ms: u64,
    ) -> Result<MeshConnectProbeReport, String> {
        let plan = self.plan_path(request, policy)?;
        let mut attempts = Vec::new();
        let mut explain = plan.explain.clone();
        let timeout = Duration::from_millis(timeout_ms.max(1));
        let mut connected_peer = String::new();
        let mut connected_endpoint = String::new();

        for peer in &plan.selected_peers {
            let fallback_endpoints =
                fallback_endpoints_for_peer(peer, &policy.connect_fallback_ports)?;
            for endpoint in fallback_endpoints {
                match connect_endpoint(&endpoint, timeout) {
                    Ok(()) => {
                        attempts.push(MeshConnectAttempt {
                            peer_id: peer.node_id.clone(),
                            endpoint: endpoint.clone(),
                            success: true,
                            error: String::new(),
                        });
                        connected_peer = peer.node_id.clone();
                        connected_endpoint = endpoint;
                        explain.push("connect_probe_result=connected".to_string());
                        explain.push(format!("connect_probe_connected_peer={connected_peer}"));
                        explain.push(format!(
                            "connect_probe_connected_endpoint={connected_endpoint}"
                        ));
                        return Ok(MeshConnectProbeReport {
                            namespace: self.namespace.clone(),
                            selected_peers: plan
                                .selected_peers
                                .iter()
                                .map(|p| p.node_id.clone())
                                .collect(),
                            connected_peer,
                            connected_endpoint,
                            success: true,
                            attempts,
                            explain,
                        });
                    }
                    Err(error) => {
                        attempts.push(MeshConnectAttempt {
                            peer_id: peer.node_id.clone(),
                            endpoint,
                            success: false,
                            error,
                        });
                    }
                }
            }
        }

        explain.push("connect_probe_result=failed".to_string());
        Ok(MeshConnectProbeReport {
            namespace: self.namespace.clone(),
            selected_peers: plan
                .selected_peers
                .iter()
                .map(|p| p.node_id.clone())
                .collect(),
            connected_peer,
            connected_endpoint,
            success: false,
            attempts,
            explain,
        })
    }

    pub fn connect_probe_from_dps_payload(
        &self,
        request: &MeshJoinRequest,
        payload: &str,
        timeout_ms: u64,
    ) -> Result<MeshConnectProbeReport, String> {
        super::payload_utils::ensure_mesh_payload_nonempty(payload)?;
        let policy = MeshPathPolicy::from_dps_payload(payload)?;
        self.connect_probe(request, &policy, timeout_ms)
    }
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
