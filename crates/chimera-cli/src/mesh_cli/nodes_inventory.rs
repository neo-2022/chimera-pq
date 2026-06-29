mod bootstrap;
mod discovery;
mod parse;
mod state;

use std::{collections::BTreeSet, env, fs};

use chimera_config::RawConfig;
use chimera_mesh::{MeshNode, MeshPublishedEndpointUpdate};

use bootstrap::{
    load_upstream_bootstrap_nodes, merge_cli_nodes, retain_reachable_nodes,
    should_bootstrap_from_upstream,
};
pub(crate) use discovery::build_discovery_signature_message;
use discovery::{
    config_discovery_pubkey, config_discovery_url, discovery_pubkey_from_env,
    discovery_url_from_env, fetch_discovery_nodes, parse_discovery_keyring,
};
use parse::parse_cli_nodes;
pub(crate) use parse::parse_inventory_config_text;
pub(crate) use state::default_runtime_state_path;
use state::{load_activation_state, load_identity_state, load_runtime_state, resolve_self_node_id};

#[derive(Debug, Clone, Default)]
pub(crate) struct MeshNodesInventory {
    pub(crate) nodes: Vec<MeshNode>,
    pub(crate) self_node_id: Option<chimera_mesh::MeshNodeId>,
    pub(crate) current_node: Option<chimera_mesh::MeshNodeId>,
    pub(crate) pinned_node: Option<chimera_mesh::MeshNodeId>,
    pub(crate) autoconnect_enabled: Option<bool>,
    pub(crate) restricted_reason: Option<String>,
    pub(crate) last_activation_node_id: Option<chimera_mesh::MeshNodeId>,
    pub(crate) last_activation_unix: Option<u64>,
    pub(crate) source: MeshNodesInventorySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MeshNodesInventorySource {
    #[default]
    Empty,
    Cli,
    Config,
    CliAndConfig,
}

pub(crate) fn load_mesh_nodes_inventory(args: &[String]) -> Result<MeshNodesInventory, String> {
    let mut inventory = MeshNodesInventory::default();
    let self_node_id = resolve_self_node_id(args)?;
    let identity_state = load_identity_state(args)?;
    let activation_state = load_activation_state(args)?;
    let runtime_state = load_runtime_state(args)?;
    let revoked_key_ids = parse_string_set(
        extract_flag_value(args, "--discovery-revoked-key-ids")
            .map(str::to_string)
            .or_else(|| config_string_value(args, "mesh.nodes.discovery_revoked_key_ids"))
            .or_else(|| env::var("CHIMERA_MESH_NODES_DISCOVERY_REVOKED_KEY_IDS").ok()),
    );
    let revoked_node_ids = parse_string_set(
        extract_flag_value(args, "--discovery-revoked-node-ids")
            .map(str::to_string)
            .or_else(|| config_string_value(args, "mesh.nodes.discovery_revoked_node_ids"))
            .or_else(|| env::var("CHIMERA_MESH_NODES_DISCOVERY_REVOKED_NODE_IDS").ok()),
    );
    let config_path = extract_flag_value(args, "--config")
        .map(str::to_string)
        .or_else(config_path_from_env);
    if let Some(path) = config_path.as_deref() {
        let text =
            fs::read_to_string(path).map_err(|error| format!("read config failed: {error}"))?;
        inventory = parse_inventory_config_text(&text)?;
        inventory.source = MeshNodesInventorySource::Config;
    }
    let discovery_url = extract_flag_value(args, "--discovery-url")
        .map(str::to_string)
        .or_else(|| config_discovery_url(args))
        .or_else(discovery_url_from_env);
    if let Some(url) = discovery_url.as_deref() {
        let discovery_pubkey = extract_flag_value(args, "--discovery-pubkey")
            .map(str::to_string)
            .or_else(|| config_discovery_pubkey(args))
            .or_else(discovery_pubkey_from_env)
            .unwrap_or_default();
        let keyring = parse_discovery_keyring(args, &discovery_pubkey)?;
        let discovered = fetch_discovery_nodes(url, &keyring, &revoked_key_ids, &revoked_node_ids)?;
        if !discovered.is_empty() {
            merge_cli_nodes(&mut inventory, discovered)?;
            inventory.source = match inventory.source {
                MeshNodesInventorySource::Config => MeshNodesInventorySource::CliAndConfig,
                MeshNodesInventorySource::Empty => MeshNodesInventorySource::Cli,
                source => source,
            };
        }
    }

    let cli_nodes = parse_cli_nodes(args)?;
    if !cli_nodes.is_empty() {
        merge_cli_nodes(&mut inventory, cli_nodes)?;
        inventory.source = match inventory.source {
            MeshNodesInventorySource::Config => MeshNodesInventorySource::CliAndConfig,
            _ => MeshNodesInventorySource::Cli,
        };
    }
    if inventory.nodes.is_empty() && should_bootstrap_from_upstream(args, config_path.as_deref()) {
        let fallback_nodes = load_upstream_bootstrap_nodes()?;
        if !fallback_nodes.is_empty() {
            merge_cli_nodes(&mut inventory, fallback_nodes)?;
            inventory.source = match inventory.source {
                MeshNodesInventorySource::Config => MeshNodesInventorySource::CliAndConfig,
                _ => MeshNodesInventorySource::Cli,
            };
        }
    }

    let timeout_ms = extract_flag_value(args, "--probe-timeout-ms")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1200);
    inventory.nodes = retain_reachable_nodes(inventory.nodes, timeout_ms);
    inventory.self_node_id = self_node_id.clone().or(identity_state.self_node_id.clone());
    if let Some(self_id) = self_node_id
        && revoked_node_ids.contains(self_id.0.as_str())
    {
        inventory.restricted_reason = Some(format!(
            "self node '{}' is revoked; enter restricted mode until re-enroll",
            self_id
        ));
    }
    let state_self = identity_state.self_node_id.as_ref();
    if identity_state.status == "active"
        && let Some(self_id) = inventory.self_node_id.as_ref()
        && Some(self_id) == state_self
        && !revoked_node_ids.contains(self_id.0.as_str())
    {
        inventory.restricted_reason = None;
    }
    inventory.last_activation_node_id = activation_state.self_node_id;
    inventory.last_activation_unix = activation_state.activated_at_unix;
    if runtime_state.current_node.is_some() {
        inventory.current_node = runtime_state.current_node;
    }
    if runtime_state.pinned_node.is_some() {
        inventory.pinned_node = runtime_state.pinned_node;
    }
    if runtime_state.autoconnect_enabled.is_some() {
        inventory.autoconnect_enabled = runtime_state.autoconnect_enabled;
    }
    Ok(inventory)
}

pub(crate) fn published_endpoint_updates_from_nodes(
    nodes: &[MeshNode],
) -> Result<Vec<MeshPublishedEndpointUpdate>, String> {
    nodes
        .iter()
        .filter_map(|node| {
            node.endpoint_generation
                .map(|endpoint_generation| MeshPublishedEndpointUpdate {
                    node_id: node.node_id.0.clone(),
                    endpoint: node.endpoint.clone(),
                    update_bootstrap_url: node.update_bootstrap_url.clone(),
                    endpoint_generation,
                })
        })
        .map(|update| {
            update.validate()?;
            Ok(update)
        })
        .collect()
}

pub(crate) fn extract_flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].as_str())
}

fn config_path_from_env() -> Option<String> {
    match env::var("CHIMERA_MESH_NODES_CONFIG") {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn config_string_value(args: &[String], key: &str) -> Option<String> {
    let path = extract_flag_value(args, "--config")?;
    let text = fs::read_to_string(path).ok()?;
    let raw = RawConfig::parse(&text).ok()?;
    raw.get(key).map(str::to_string)
}

fn parse_string_set(raw: Option<String>) -> BTreeSet<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}
