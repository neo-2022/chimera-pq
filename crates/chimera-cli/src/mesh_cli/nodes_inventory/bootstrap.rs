use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use chimera_config::RawConfig;
use chimera_mesh::{MeshNode, MeshNodeCountry};

use super::MeshNodesInventory;
use super::parse::build_node;

const BOOTSTRAP_ENDPOINT_KEYS: [&str; 3] = [
    "CHIMERA_NODE_ENDPOINT",
    "CHIMERA_PEER_ENDPOINT",
    "CHIMERA_MESH_REMOTE_ENDPOINT",
];

pub(super) fn load_mesh_bootstrap_nodes() -> Result<Vec<MeshNode>, String> {
    let endpoints = read_mesh_bootstrap_endpoints();
    mesh_bootstrap_nodes_from_endpoints(endpoints)
}

#[cfg(test)]
pub(crate) fn load_mesh_bootstrap_nodes_from_text(input: &str) -> Result<Vec<MeshNode>, String> {
    let raw = RawConfig::parse(input).map_err(|error| error.to_string())?;
    mesh_bootstrap_nodes_from_endpoints(read_mesh_bootstrap_endpoints_from_raw(&raw))
}

#[cfg(test)]
pub(crate) fn bootstrap_env_value_from_text(input: &str, key: &str) -> Option<String> {
    let raw = RawConfig::parse(input).ok()?;
    raw.get(key).and_then(non_empty_env_value)
}

fn mesh_bootstrap_nodes_from_endpoints(endpoints: Vec<String>) -> Result<Vec<MeshNode>, String> {
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
            &format!("bootstrap-{}", index + 1),
            &endpoint,
            MeshNodeCountry::UNKNOWN_CODE,
            MeshNodeCountry::UNKNOWN_NAME,
            "operator_override",
            "low",
            "mesh_bootstrap",
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
            None,
            "mesh_bootstrap",
        )?;
        out.push(node);
    }
    Ok(out)
}

pub(super) fn bootstrap_env_value(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .and_then(|value| non_empty_env_value(&value))
        .or_else(|| {
            let raw = read_bootstrap_env_config()?;
            raw.get(key).and_then(non_empty_env_value)
        })
}

pub(super) fn read_bootstrap_env_config() -> Option<RawConfig> {
    let path = bootstrap_env_path()?;
    let text = fs::read_to_string(path).ok()?;
    RawConfig::parse(&text).ok()
}

fn read_mesh_bootstrap_endpoints() -> Vec<String> {
    let env_endpoints = BOOTSTRAP_ENDPOINT_KEYS
        .iter()
        .filter_map(|key| env::var(key).ok())
        .filter_map(|value| normalize_endpoint(&value))
        .collect::<Vec<_>>();
    if !env_endpoints.is_empty() {
        return env_endpoints;
    }
    let Some(raw) = read_bootstrap_env_config() else {
        return Vec::new();
    };
    read_mesh_bootstrap_endpoints_from_raw(&raw)
}

fn read_mesh_bootstrap_endpoints_from_raw(raw: &RawConfig) -> Vec<String> {
    BOOTSTRAP_ENDPOINT_KEYS
        .iter()
        .filter_map(|key| raw.get(key))
        .filter_map(normalize_endpoint)
        .collect()
}

fn bootstrap_env_path() -> Option<String> {
    if let Ok(path) = env::var("CHIMERA_BOOTSTRAP_ENV_FILE")
        && !path.trim().is_empty()
    {
        return Some(path);
    }
    let home = env::var("HOME").ok()?;
    let config_home = env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("{home}/.config"));
    Some(format!("{config_home}/chimera/mesh_bootstrap.env"))
}

fn non_empty_env_value(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('"');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
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

pub(super) fn should_bootstrap_from_mesh_env(args: &[String], config_path: Option<&str>) -> bool {
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
