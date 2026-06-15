use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use chimera_mesh::{MeshNode, MeshNodeCountry};

use super::MeshNodesInventory;
use super::parse::build_node;

pub(super) fn load_upstream_bootstrap_nodes() -> Result<Vec<MeshNode>, String> {
    let last_endpoint = read_last_upstream_endpoint();
    let mut endpoints = Vec::new();
    if let Some(endpoint) = last_endpoint.clone() {
        endpoints.push(endpoint);
    }
    endpoints.extend(read_upstream_endpoints_csv());
    if endpoints.is_empty() {
        return Ok(Vec::new());
    }
    let mut host_to_endpoint = BTreeMap::<String, String>::new();
    for endpoint in endpoints {
        let Some(host) = endpoint_host(&endpoint) else {
            continue;
        };
        host_to_endpoint.entry(host).or_insert(endpoint);
    }
    if host_to_endpoint.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for (index, (_host, endpoint)) in host_to_endpoint.into_iter().enumerate() {
        let node = build_node(
            &format!("upstream-{}", index + 1),
            &endpoint,
            MeshNodeCountry::UNKNOWN_CODE,
            MeshNodeCountry::UNKNOWN_NAME,
            "geoip",
            "low",
            "upstream_bootstrap",
            "86400",
            "false",
            None,
            "healthy",
            "-",
            "-",
            "-",
            "99",
            "99",
            "0",
            "1",
            None,
            None,
            "upstream_bootstrap",
        )?;
        out.push(node);
    }
    Ok(out)
}

fn read_last_upstream_endpoint() -> Option<String> {
    let path = format!(
        "{}/chimera/last_upstream_endpoint",
        env::var("XDG_CACHE_HOME")
            .ok()
            .unwrap_or_else(|| format!("{}/.cache", env::var("HOME").unwrap_or_default()))
    );
    let text = fs::read_to_string(path).ok()?;
    let endpoint = text
        .lines()
        .next()
        .unwrap_or_default()
        .split('|')
        .next()
        .unwrap_or_default()
        .trim();
    normalize_endpoint(endpoint)
}

fn read_upstream_endpoints_csv() -> Vec<String> {
    let path = format!(
        "{}/chimera/upstream_proxy.env",
        env::var("XDG_CONFIG_HOME")
            .ok()
            .unwrap_or_else(|| format!("{}/.config", env::var("HOME").unwrap_or_default()))
    );
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("CHIMERA_UPSTREAM_ENDPOINTS_CSV="))
        .flat_map(|value| value.split(','))
        .filter_map(normalize_endpoint)
        .collect()
}

fn normalize_endpoint(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || !value.contains(':') || value.contains('/') {
        return None;
    }
    Some(value.to_string())
}

fn endpoint_host(endpoint: &str) -> Option<String> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return None;
    }
    if let Some(stripped) = endpoint.strip_prefix('[') {
        let host = stripped.split(']').next()?.trim();
        if host.is_empty() {
            return None;
        }
        return Some(host.to_string());
    }
    let (host, _port) = endpoint.rsplit_once(':')?;
    let host = host.trim();
    if host.is_empty() {
        return None;
    }
    Some(host.to_string())
}

pub(super) fn should_bootstrap_from_upstream(args: &[String], config_path: Option<&str>) -> bool {
    if config_path.is_some() {
        return false;
    }
    let guard_flags = [
        "--node",
        "--discovery-url",
        "--discovery-pubkey",
        "--discovery-keyring",
        "--discovery-revoked-key-ids",
        "--discovery-revoked-node-ids",
    ];
    !args.iter().any(|arg| guard_flags.contains(&arg.as_str()))
}

pub(super) fn merge_cli_nodes(
    inventory: &mut MeshNodesInventory,
    cli_nodes: Vec<MeshNode>,
) -> Result<(), String> {
    let mut ids = inventory
        .nodes
        .iter()
        .map(|node| node.node_id.0.clone())
        .collect::<BTreeSet<_>>();
    for node in cli_nodes {
        if !ids.insert(node.node_id.0.clone()) {
            return Err(format!(
                "duplicate node_id across config and --node: {}",
                node.node_id
            ));
        }
        inventory.nodes.push(node);
    }
    Ok(())
}

pub(super) fn retain_reachable_nodes(nodes: Vec<MeshNode>, timeout_ms: u64) -> Vec<MeshNode> {
    let timeout = Duration::from_millis(timeout_ms);
    nodes
        .into_iter()
        .filter(|node| is_reachable_endpoint(&node.endpoint, timeout))
        .collect()
}

fn is_reachable_endpoint(endpoint: &str, timeout: Duration) -> bool {
    let addrs = match endpoint.to_socket_addrs() {
        Ok(value) => value.collect::<Vec<_>>(),
        Err(_) => return false,
    };
    for addr in addrs {
        if TcpStream::connect_timeout(&addr, timeout).is_ok() {
            return true;
        }
    }
    false
}
