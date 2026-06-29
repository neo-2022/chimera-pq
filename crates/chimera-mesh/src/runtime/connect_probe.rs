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
        let plan = self.plan_path(request, policy)?;
        let mut attempts = Vec::new();
        let mut explain = plan.explain.clone();
        let timeout = Duration::from_millis(timeout_ms.max(1));
        let mut connected_peer = String::new();
        let mut connected_endpoint = String::new();

        let attempt_plan =
            build_connect_attempt_plan(&plan.selected_peers, &policy.connect_fallback_ports)?;
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
                        selected_peers: redacted_peer_labels(&plan.selected_peers),
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
            selected_peers: redacted_peer_labels(&plan.selected_peers),
            connected_peer,
            connected_endpoint,
            success: false,
            attempts,
            explain,
        })
    }

    pub fn connect_probe_from_dps_payload(
        &mut self,
        request: &MeshJoinRequest,
        payload: &str,
        timeout_ms: u64,
    ) -> Result<MeshConnectProbeReport, String> {
        super::payload_utils::ensure_mesh_payload_nonempty(payload)?;
        let policy = MeshPathPolicy::from_dps_payload(payload)?;
        self.connect_probe(request, &policy, timeout_ms)
    }
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
mod tests {
    use super::build_connect_attempt_plan;
    use crate::{
        MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPublishedEndpointUpdate,
        MeshRuntime,
    };

    fn record(node_id: &str, endpoint: &str) -> MeshDiscoveryRecord {
        MeshDiscoveryRecord {
            node_id: node_id.to_string(),
            endpoint: endpoint.to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        }
    }

    fn endpoint_update(endpoint: &str, endpoint_generation: u64) -> MeshPublishedEndpointUpdate {
        MeshPublishedEndpointUpdate {
            node_id: "node-a".to_string(),
            endpoint: endpoint.to_string(),
            update_bootstrap_url: None,
            endpoint_generation,
        }
    }

    fn request() -> MeshJoinRequest {
        MeshJoinRequest {
            namespace: "cef-public".to_string(),
            node_name: "node-client".to_string(),
            invite_token: Some("inv-123".to_string()),
        }
    }

    fn policy() -> MeshPathPolicy {
        MeshPathPolicy::from_dps_payload(
            "allow=mesh;target_region=eu;mesh_max_peers=1;mesh_max_selected_per_region=1;mesh_connect_fallback_ports=443,8443",
        )
        .unwrap_or_else(|e| unreachable!("policy parse should succeed: {e}"))
    }

    fn planned_endpoints(
        runtime: &MeshRuntime,
        policy: &MeshPathPolicy,
    ) -> Result<Vec<String>, String> {
        let plan = runtime.plan_path(&request(), policy)?;
        Ok(
            build_connect_attempt_plan(&plan.selected_peers, &policy.connect_fallback_ports)?
                .into_iter()
                .map(|target| target.endpoint)
                .collect(),
        )
    }

    fn assert_no_endpoint_with_host(endpoints: &[String], host: &str) {
        assert!(
            endpoints.iter().all(|endpoint| !endpoint.starts_with(host)),
            "unexpected stale host in connect attempt plan"
        );
    }

    fn assert_endpoint_with_host(endpoints: &[String], host: &str) {
        assert!(
            endpoints.iter().any(|endpoint| endpoint.starts_with(host)),
            "expected host missing from connect attempt plan"
        );
    }

    fn assert_same_connect_attempt_plan(actual: &[String], expected: &[String]) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "connect attempt plan length changed"
        );
        assert!(
            actual
                .iter()
                .zip(expected.iter())
                .all(|(actual, expected)| actual == expected),
            "connect attempt plan changed"
        );
    }

    #[test]
    fn connect_attempt_plan_uses_fresh_published_endpoint_generation() -> Result<(), String> {
        let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
        runtime.merge_discovery("seed-b", &[record("node-a", "198.51.100.10:9443")])?;
        let _ = runtime.take_pending_multipath_rebuild_signal();
        let policy = policy();

        let before = planned_endpoints(&runtime, &policy)?;
        assert_endpoint_with_host(&before, "198.51.100.10:");

        runtime.merge_published_endpoint_updates(
            "state-publish",
            &[endpoint_update("198.51.100.20:9443", 2)],
        )?;

        let signal = runtime
            .pending_multipath_rebuild_signal()
            .ok_or_else(|| "fresh endpoint update should mark reconnect path dirty".to_string())?;
        assert_eq!(signal.reason(), "published_endpoint_changed");
        assert_eq!(signal.affected_peer_count(), 1);

        let after = planned_endpoints(&runtime, &policy)?;
        assert_endpoint_with_host(&after, "198.51.100.20:");
        assert_no_endpoint_with_host(&after, "198.51.100.10:");
        assert_eq!(after.len(), 3);
        Ok(())
    }

    #[test]
    fn connect_attempt_plan_ignores_stale_and_noop_published_endpoint_updates() -> Result<(), String>
    {
        let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
        runtime.merge_discovery("seed-b", &[record("node-a", "198.51.100.10:9443")])?;
        let _ = runtime.take_pending_multipath_rebuild_signal();
        runtime.merge_published_endpoint_updates(
            "state-publish",
            &[endpoint_update("198.51.100.20:9443", 7)],
        )?;
        let _ = runtime.take_pending_multipath_rebuild_signal();
        let policy = policy();
        let fresh = planned_endpoints(&runtime, &policy)?;

        runtime.merge_published_endpoint_updates(
            "state-publish",
            &[endpoint_update("198.51.100.30:9443", 6)],
        )?;
        assert!(runtime.pending_multipath_rebuild_signal().is_none());
        let after_stale = planned_endpoints(&runtime, &policy)?;
        assert_same_connect_attempt_plan(&after_stale, &fresh);
        assert_no_endpoint_with_host(&after_stale, "198.51.100.30:");

        runtime.merge_published_endpoint_updates(
            "state-publish",
            &[endpoint_update("198.51.100.20:9443", 7)],
        )?;
        assert!(runtime.pending_multipath_rebuild_signal().is_none());
        let after_noop = planned_endpoints(&runtime, &policy)?;
        assert_same_connect_attempt_plan(&after_noop, &fresh);
        Ok(())
    }

    #[test]
    fn connect_attempt_plan_survives_invalid_published_endpoint_update_atomically()
    -> Result<(), String> {
        let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
        runtime.merge_discovery("seed-b", &[record("node-a", "198.51.100.10:9443")])?;
        let _ = runtime.take_pending_multipath_rebuild_signal();
        runtime.merge_published_endpoint_updates(
            "state-publish",
            &[endpoint_update("198.51.100.20:9443", 3)],
        )?;
        let _ = runtime.take_pending_multipath_rebuild_signal();
        let policy = policy();
        let before = planned_endpoints(&runtime, &policy)?;

        let error = match runtime.merge_published_endpoint_updates(
            "state-publish",
            &[endpoint_update("198.51.100.30", 4)],
        ) {
            Ok(_) => return Err("invalid endpoint update must fail".to_string()),
            Err(error) => error,
        };

        assert!(error.contains("endpoint"));
        assert!(runtime.pending_multipath_rebuild_signal().is_none());
        let after = planned_endpoints(&runtime, &policy)?;
        assert_same_connect_attempt_plan(&after, &before);
        assert_no_endpoint_with_host(&after, "198.51.100.30:");
        Ok(())
    }
}
