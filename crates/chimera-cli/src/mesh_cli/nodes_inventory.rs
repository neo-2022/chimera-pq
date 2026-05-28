use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    net::{TcpStream, ToSocketAddrs},
    sync::{Mutex, OnceLock},
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use base64::Engine as _;
use chimera_config::RawConfig;
use chimera_mesh::{
    MeshNode, MeshNodeCountry, MeshNodeCountryConfidence, MeshNodeCountrySource, MeshNodeId,
    MeshNodeStatus,
};
use ring::signature::{ED25519, UnparsedPublicKey};

#[derive(Debug, Clone, Default)]
struct MeshIdentityState {
    status: String,
    self_node_id: Option<MeshNodeId>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MeshNodesInventory {
    pub(crate) nodes: Vec<MeshNode>,
    pub(crate) self_node_id: Option<MeshNodeId>,
    pub(crate) current_node: Option<MeshNodeId>,
    pub(crate) pinned_node: Option<MeshNodeId>,
    pub(crate) autoconnect_enabled: Option<bool>,
    pub(crate) restricted_reason: Option<String>,
    pub(crate) last_activation_node_id: Option<MeshNodeId>,
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

pub(crate) fn parse_inventory_config_text(input: &str) -> Result<MeshNodesInventory, String> {
    let raw = RawConfig::parse(input).map_err(|error| error.to_string())?;
    let ids = parse_required_ids(&raw)?;
    validate_config_keys(&raw, &ids)?;

    let mut nodes = Vec::new();
    for id in &ids {
        nodes.push(parse_config_node(&raw, id)?);
    }

    Ok(MeshNodesInventory {
        nodes,
        self_node_id: optional_node_id(&raw, "mesh.nodes.self_node_id")?,
        current_node: optional_node_id(&raw, "mesh.nodes.current")?,
        pinned_node: optional_node_id(&raw, "mesh.nodes.pinned")?,
        autoconnect_enabled: optional_bool(&raw, "mesh.nodes.autoconnect")?,
        restricted_reason: None,
        last_activation_node_id: None,
        last_activation_unix: None,
        source: MeshNodesInventorySource::Config,
    })
}

pub(crate) fn extract_flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].as_str())
}

pub(crate) fn parse_optional_f64(value: &str) -> Result<Option<f64>, String> {
    if value == "-" || value.trim().is_empty() {
        return Ok(None);
    }
    value
        .parse::<f64>()
        .map(Some)
        .map_err(|_| format!("invalid numeric value: {value}"))
}

pub(crate) fn parse_u32(value: &str, label: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("invalid {label}: {value}"))
}

fn config_path_from_env() -> Option<String> {
    match env::var("CHIMERA_MESH_NODES_CONFIG") {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn discovery_url_from_env() -> Option<String> {
    match env::var("CHIMERA_MESH_NODES_DISCOVERY_URL") {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn discovery_pubkey_from_env() -> Option<String> {
    match env::var("CHIMERA_MESH_NODES_DISCOVERY_PUBKEY") {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn config_discovery_url(args: &[String]) -> Option<String> {
    let path = extract_flag_value(args, "--config")?;
    let text = fs::read_to_string(path).ok()?;
    let raw = RawConfig::parse(&text).ok()?;
    raw.get("mesh.nodes.discovery_url").map(str::to_string)
}

fn config_discovery_pubkey(args: &[String]) -> Option<String> {
    config_string_value(args, "mesh.nodes.discovery_pubkey")
}

fn fetch_discovery_nodes(
    url: &str,
    discovery_keyring: &BTreeMap<String, String>,
    revoked_key_ids: &BTreeSet<String>,
    revoked_node_ids: &BTreeSet<String>,
) -> Result<Vec<MeshNode>, String> {
    let response = ureq::get(url)
        .call()
        .map_err(|error| format!("mesh discovery request failed: {error}"))?;
    let text = response
        .into_string()
        .map_err(|error| format!("mesh discovery read body failed: {error}"))?;
    parse_discovery_nodes_json(&text, discovery_keyring, revoked_key_ids, revoked_node_ids)
}

fn parse_discovery_nodes_json(
    input: &str,
    discovery_keyring: &BTreeMap<String, String>,
    revoked_key_ids: &BTreeSet<String>,
    revoked_node_ids: &BTreeSet<String>,
) -> Result<Vec<MeshNode>, String> {
    let value: serde_json::Value = serde_json::from_str(input)
        .map_err(|error| format!("mesh discovery json parse failed: {error}"))?;
    let nodes_value = parse_discovery_envelope(&value, discovery_keyring, revoked_key_ids)?;
    let records = nodes_value
        .as_array()
        .ok_or_else(|| "mesh discovery payload must contain 'nodes' array".to_string())?;
    let mut out = Vec::with_capacity(records.len());
    let mut ids = BTreeSet::new();
    for record in records {
        let node = parse_discovery_node_record(record)?;
        if revoked_node_ids.contains(&node.node_id.0) {
            continue;
        }
        if !ids.insert(node.node_id.0.clone()) {
            return Err(format!(
                "duplicate node_id in mesh discovery payload: {}",
                node.node_id
            ));
        }
        out.push(node);
    }
    Ok(out)
}

fn parse_discovery_envelope<'a>(
    value: &'a serde_json::Value,
    discovery_keyring: &BTreeMap<String, String>,
    revoked_key_ids: &BTreeSet<String>,
) -> Result<&'a serde_json::Value, String> {
    const CONTRACT_VERSION: u64 = 1;
    const MAX_CLOCK_SKEW_SEC: u64 = 120;
    let object = value
        .as_object()
        .ok_or_else(|| "mesh discovery payload must be object".to_string())?;
    let contract_version = object
        .get("contract_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "mesh discovery envelope missing 'contract_version'".to_string())?;
    if contract_version != CONTRACT_VERSION {
        return Err(format!(
            "mesh discovery unsupported contract_version: {contract_version}"
        ));
    }
    let issued_at = object
        .get("issued_at_unix")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "mesh discovery envelope missing 'issued_at_unix'".to_string())?;
    let expires_at = object
        .get("expires_at_unix")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "mesh discovery envelope missing 'expires_at_unix'".to_string())?;
    if expires_at <= issued_at {
        return Err("mesh discovery envelope expires_at_unix must be > issued_at_unix".to_string());
    }
    let now = current_unix_seconds()?;
    if issued_at > now.saturating_add(MAX_CLOCK_SKEW_SEC) {
        return Err("mesh discovery envelope issued_at_unix is too far in future".to_string());
    }
    if expires_at < now {
        return Err("mesh discovery envelope is expired".to_string());
    }
    let nonce = object
        .get("nonce")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "mesh discovery envelope missing 'nonce'".to_string())?;
    if nonce.trim().is_empty() {
        return Err("mesh discovery envelope nonce must be non-empty".to_string());
    }
    remember_discovery_nonce(nonce)?;
    let key_id = object
        .get("key_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "mesh discovery envelope missing 'key_id'".to_string())?;
    if key_id.trim().is_empty() {
        return Err("mesh discovery envelope key_id must be non-empty".to_string());
    }
    if revoked_key_ids.contains(key_id) {
        return Err(format!("mesh discovery rejected revoked key_id: {key_id}"));
    }
    let signature = object
        .get("signature")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "mesh discovery envelope missing 'signature'".to_string())?;
    if signature.trim().is_empty() {
        return Err("mesh discovery envelope signature must be non-empty".to_string());
    }
    let nodes = object
        .get("nodes")
        .ok_or_else(|| "mesh discovery envelope missing 'nodes'".to_string())?;
    let message =
        build_discovery_signature_message(contract_version, issued_at, expires_at, nonce, nodes)?;
    let discovery_pubkey = discovery_keyring
        .get(key_id)
        .ok_or_else(|| format!("mesh discovery unknown key_id: {key_id}"))?;
    verify_discovery_signature(discovery_pubkey, signature, &message)?;
    Ok(nodes)
}

fn current_unix_seconds() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("system clock error: {error}"))
}

fn remember_discovery_nonce(nonce: &str) -> Result<(), String> {
    const MAX_TRACKED_NONCES: usize = 4096;
    static NONCES: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
    let cache = NONCES.get_or_init(|| Mutex::new(BTreeSet::new()));
    let mut guard = cache
        .lock()
        .map_err(|_| "mesh discovery nonce cache lock poisoned".to_string())?;
    if guard.contains(nonce) {
        return Err("mesh discovery anti-replay rejected duplicate nonce".to_string());
    }
    if guard.len() >= MAX_TRACKED_NONCES
        && let Some(oldest) = guard.first().cloned()
    {
        guard.remove(&oldest);
    }
    guard.insert(nonce.to_string());
    Ok(())
}

fn verify_discovery_signature(
    discovery_pubkey: &str,
    signature: &str,
    message: &[u8],
) -> Result<(), String> {
    let pubkey_bytes = base64::engine::general_purpose::STANDARD
        .decode(discovery_pubkey)
        .map_err(|error| format!("mesh discovery pubkey base64 decode failed: {error}"))?;
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature)
        .map_err(|error| format!("mesh discovery signature base64 decode failed: {error}"))?;
    let verifier = UnparsedPublicKey::new(&ED25519, pubkey_bytes);
    verifier
        .verify(message, &signature_bytes)
        .map_err(|_| "mesh discovery signature verification failed".to_string())
}

pub(crate) fn build_discovery_signature_message(
    contract_version: u64,
    issued_at_unix: u64,
    expires_at_unix: u64,
    nonce: &str,
    nodes: &serde_json::Value,
) -> Result<Vec<u8>, String> {
    let nodes_compact = serde_json::to_string(nodes)
        .map_err(|error| format!("mesh discovery nodes serialize failed: {error}"))?;
    Ok(format!(
        "contract_version={contract_version}\nissued_at_unix={issued_at_unix}\nexpires_at_unix={expires_at_unix}\nnonce={nonce}\nnodes={nodes_compact}\n"
    )
    .into_bytes())
}

fn parse_discovery_keyring(
    args: &[String],
    fallback_pubkey: &str,
) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    if let Some(raw) = extract_flag_value(args, "--discovery-keyring")
        .map(str::to_string)
        .or_else(|| config_string_value(args, "mesh.nodes.discovery_keyring"))
        .or_else(|| env::var("CHIMERA_MESH_NODES_DISCOVERY_KEYRING").ok())
    {
        for entry in raw.split(',').map(str::trim).filter(|v| !v.is_empty()) {
            let (key_id, pubkey) = entry
                .split_once(':')
                .ok_or_else(|| "discovery keyring entry must be key_id:base64".to_string())?;
            if key_id.trim().is_empty() || pubkey.trim().is_empty() {
                return Err(
                    "discovery keyring entry must have non-empty key_id and pubkey".to_string(),
                );
            }
            out.insert(key_id.trim().to_string(), pubkey.trim().to_string());
        }
    }
    if !fallback_pubkey.trim().is_empty() {
        out.insert("default".to_string(), fallback_pubkey.trim().to_string());
    }
    if out.is_empty() {
        return Err(
            "mesh discovery keyring is required (use --discovery-keyring/--discovery-pubkey, config keys mesh.nodes.discovery_keyring/mesh.nodes.discovery_pubkey, or CHIMERA_MESH_NODES_DISCOVERY_KEYRING/CHIMERA_MESH_NODES_DISCOVERY_PUBKEY)"
                .to_string(),
        );
    }
    Ok(out)
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

fn resolve_self_node_id(args: &[String]) -> Result<Option<MeshNodeId>, String> {
    let value = extract_flag_value(args, "--self-node-id")
        .map(str::to_string)
        .or_else(|| config_string_value(args, "mesh.nodes.self_node_id"))
        .or_else(|| env::var("CHIMERA_MESH_SELF_NODE_ID").ok());
    let Some(value) = value else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    MeshNodeId::new(&value).validate()?;
    Ok(Some(MeshNodeId::new(value)))
}

fn load_identity_state(args: &[String]) -> Result<MeshIdentityState, String> {
    let state_path = extract_flag_value(args, "--identity-state")
        .map(str::to_string)
        .or_else(|| config_string_value(args, "mesh.nodes.identity_state_path"))
        .or_else(|| env::var("CHIMERA_MESH_IDENTITY_STATE_PATH").ok());
    let Some(path) = state_path else {
        return Ok(MeshIdentityState::default());
    };
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(MeshIdentityState::default());
        }
        Err(error) => {
            return Err(format!("read identity state failed: {error}"));
        }
    };
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("identity state json parse failed: {error}"))?;
    let status = value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let self_node_id = value
        .get("self_node_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .filter(|id| !id.trim().is_empty())
        .map(MeshNodeId::new);
    if let Some(id) = self_node_id.as_ref() {
        id.validate()?;
    }
    Ok(MeshIdentityState {
        status,
        self_node_id,
    })
}

fn load_activation_state(args: &[String]) -> Result<MeshActivationState, String> {
    let activation_path = extract_flag_value(args, "--activation-log")
        .map(str::to_string)
        .or_else(|| config_string_value(args, "mesh.nodes.activation_log_path"))
        .or_else(|| env::var("CHIMERA_MESH_ACTIVATION_LOG_PATH").ok());
    let Some(path) = activation_path else {
        return Ok(MeshActivationState::default());
    };
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(MeshActivationState::default());
        }
        Err(error) => {
            return Err(format!("read activation log failed: {error}"));
        }
    };
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("activation log json parse failed: {error}"))?;
    let self_node_id = value
        .get("self_node_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .filter(|id| !id.trim().is_empty())
        .map(MeshNodeId::new);
    if let Some(id) = self_node_id.as_ref() {
        id.validate()?;
    }
    let activated_at_unix = value
        .get("activated_at_unix")
        .and_then(serde_json::Value::as_u64);
    Ok(MeshActivationState {
        self_node_id,
        activated_at_unix,
    })
}

#[derive(Debug, Clone, Default)]
struct MeshActivationState {
    self_node_id: Option<MeshNodeId>,
    activated_at_unix: Option<u64>,
}

#[derive(Debug, Clone, Default)]
struct MeshRuntimeState {
    current_node: Option<MeshNodeId>,
    pinned_node: Option<MeshNodeId>,
    autoconnect_enabled: Option<bool>,
}

pub(crate) fn default_runtime_state_path() -> String {
    if let Ok(xdg_state_home) = env::var("XDG_STATE_HOME")
        && !xdg_state_home.trim().is_empty()
    {
        return format!(
            "{}/chimera/mesh_nodes_runtime_state.json",
            xdg_state_home.trim_end_matches('/')
        );
    }
    format!(
        "{}/.local/state/chimera/mesh_nodes_runtime_state.json",
        env::var("HOME").unwrap_or_default()
    )
}

fn load_runtime_state(args: &[String]) -> Result<MeshRuntimeState, String> {
    let state_path = extract_flag_value(args, "--runtime-state")
        .map(str::to_string)
        .or_else(|| config_string_value(args, "mesh.nodes.runtime_state_path"))
        .or_else(|| env::var("CHIMERA_MESH_NODES_RUNTIME_STATE_PATH").ok())
        .unwrap_or_else(default_runtime_state_path);
    let path = state_path;
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(MeshRuntimeState::default());
        }
        Err(error) => {
            return Err(format!("read mesh runtime state failed: {error}"));
        }
    };
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("mesh runtime state json parse failed: {error}"))?;
    let current_node = value
        .get("current_node_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .filter(|id| !id.trim().is_empty())
        .map(MeshNodeId::new);
    if let Some(id) = current_node.as_ref() {
        id.validate()?;
    }
    let pinned_node = value
        .get("pinned_node_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .filter(|id| !id.trim().is_empty())
        .map(MeshNodeId::new);
    if let Some(id) = pinned_node.as_ref() {
        id.validate()?;
    }
    let autoconnect_enabled = value
        .get("autoconnect")
        .and_then(serde_json::Value::as_bool);
    Ok(MeshRuntimeState {
        current_node,
        pinned_node,
        autoconnect_enabled,
    })
}

fn parse_discovery_node_record(record: &serde_json::Value) -> Result<MeshNode, String> {
    let node_id = json_string(record, &["node_id", "id"])?;
    let endpoint = json_string(record, &["endpoint"])?;
    let invite_token = json_optional_string(record, &["invite_token"]);
    let country_code =
        json_string_default(record, &["country_code"], MeshNodeCountry::UNKNOWN_CODE);
    let country_name =
        json_string_default(record, &["country_name"], MeshNodeCountry::UNKNOWN_NAME);
    let country_source = json_string_default(record, &["country_source"], "geoip");
    let country_confidence = json_string_default(record, &["country_confidence"], "low");
    let country_updated_at = json_string_default(record, &["country_updated_at"], "discovery");
    let country_ttl_sec = json_u64_default(record, &["country_ttl_sec"], 86400).to_string();
    let country_conflict = json_bool_default(record, &["country_conflict"], false).to_string();
    let country_conflict_reason = json_optional_string(record, &["country_conflict_reason"]);
    let status = json_string_default(record, &["status"], "checking");
    let latency_ms = json_optional_number_string(record, &["latency_ms"]);
    let jitter_ms = json_optional_number_string(record, &["jitter_ms"]);
    let loss_pct = json_optional_number_string(record, &["loss_pct"]);
    let success_rate_5m = json_optional_number_string(record, &["success_rate_5m"]);
    let success_rate_1h = json_optional_number_string(record, &["success_rate_1h"]);
    let consecutive_failures = json_u64_default(record, &["consecutive_failures"], 0).to_string();
    let observation_count = json_u64_default(record, &["observation_count"], 0).to_string();
    let explain_reason = json_string_default(record, &["explain_reason"], "discovery_node_record");
    build_node(
        &node_id,
        &endpoint,
        &country_code,
        &country_name,
        &country_source,
        &country_confidence,
        &country_updated_at,
        &country_ttl_sec,
        &country_conflict,
        country_conflict_reason.as_deref(),
        &status,
        &latency_ms,
        &jitter_ms,
        &loss_pct,
        &success_rate_5m,
        &success_rate_1h,
        &consecutive_failures,
        &observation_count,
        invite_token.as_deref(),
        &explain_reason,
    )
}

fn json_string(record: &serde_json::Value, keys: &[&str]) -> Result<String, String> {
    for key in keys {
        if let Some(value) = record.get(*key).and_then(serde_json::Value::as_str) {
            if !value.trim().is_empty() {
                return Ok(value.to_string());
            }
        }
    }
    Err(format!(
        "mesh discovery record missing string field: {}",
        keys.join("|")
    ))
}

fn json_string_default(record: &serde_json::Value, keys: &[&str], default: &str) -> String {
    json_string(record, keys).unwrap_or_else(|_| default.to_string())
}

fn json_optional_string(record: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        record
            .get(*key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    })
}

fn json_u64_default(record: &serde_json::Value, keys: &[&str], default: u64) -> u64 {
    for key in keys {
        if let Some(value) = record.get(*key).and_then(serde_json::Value::as_u64) {
            return value;
        }
    }
    default
}

fn json_bool_default(record: &serde_json::Value, keys: &[&str], default: bool) -> bool {
    for key in keys {
        if let Some(value) = record.get(*key).and_then(serde_json::Value::as_bool) {
            return value;
        }
    }
    default
}

fn json_optional_number_string(record: &serde_json::Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(value) = record.get(*key).and_then(serde_json::Value::as_f64) {
            return value.to_string();
        }
    }
    "-".to_string()
}

fn parse_cli_nodes(args: &[String]) -> Result<Vec<MeshNode>, String> {
    let mut nodes = Vec::new();
    let mut ids = BTreeSet::new();
    let mut index = 0usize;
    while index < args.len() {
        if args[index] == "--node" {
            let raw = args
                .get(index + 1)
                .ok_or_else(|| "--node requires a value".to_string())?;
            let node = parse_cli_node(raw)?;
            if !ids.insert(node.node_id.0.clone()) {
                return Err(format!("duplicate node_id: {}", node.node_id));
            }
            nodes.push(node);
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(nodes)
}

fn parse_cli_node(raw: &str) -> Result<MeshNode, String> {
    let parts = raw.split('@').collect::<Vec<_>>();
    if parts.len() != 12 {
        return Err("node record must have 12 @-separated fields".to_string());
    }
    build_node(
        parts[0],
        parts[1],
        parts[2],
        parts[3],
        "node_claim",
        "low",
        "cli",
        "86400",
        "false",
        None,
        parts[4],
        parts[5],
        parts[6],
        parts[7],
        parts[8],
        parts[9],
        parts[10],
        parts[11],
        None,
        "cli_node_record",
    )
}

fn parse_config_node(raw: &RawConfig, id: &str) -> Result<MeshNode, String> {
    let prefix = format!("mesh.node.{id}.");
    let invite_token = raw.get(&format!("{prefix}invite_token"));
    build_node(
        id,
        required(raw, &format!("{prefix}endpoint"))?,
        raw.get(&format!("{prefix}country_code"))
            .unwrap_or(MeshNodeCountry::UNKNOWN_CODE),
        raw.get(&format!("{prefix}country_name"))
            .unwrap_or(MeshNodeCountry::UNKNOWN_NAME),
        raw.get(&format!("{prefix}country_source"))
            .unwrap_or("node_claim"),
        raw.get(&format!("{prefix}country_confidence"))
            .unwrap_or("low"),
        raw.get(&format!("{prefix}country_updated_at"))
            .unwrap_or("config"),
        raw.get(&format!("{prefix}country_ttl_sec"))
            .unwrap_or("86400"),
        raw.get(&format!("{prefix}country_conflict"))
            .unwrap_or("false"),
        raw.get(&format!("{prefix}country_conflict_reason")),
        raw.get(&format!("{prefix}status")).unwrap_or("checking"),
        raw.get(&format!("{prefix}latency_ms")).unwrap_or("-"),
        raw.get(&format!("{prefix}jitter_ms")).unwrap_or("-"),
        raw.get(&format!("{prefix}loss_pct")).unwrap_or("-"),
        raw.get(&format!("{prefix}success_rate_5m")).unwrap_or("-"),
        raw.get(&format!("{prefix}success_rate_1h")).unwrap_or("-"),
        raw.get(&format!("{prefix}consecutive_failures"))
            .unwrap_or("0"),
        raw.get(&format!("{prefix}observation_count"))
            .unwrap_or("0"),
        invite_token,
        raw.get(&format!("{prefix}explain_reason"))
            .unwrap_or("config_node_record"),
    )
}

#[allow(clippy::too_many_arguments)]
fn build_node(
    id: &str,
    endpoint: &str,
    country_code_raw: &str,
    country_name: &str,
    country_source: &str,
    country_confidence: &str,
    country_updated_at: &str,
    country_ttl_sec: &str,
    country_conflict: &str,
    country_conflict_reason: Option<&str>,
    status: &str,
    latency_ms: &str,
    jitter_ms: &str,
    loss_pct: &str,
    success_rate_5m: &str,
    success_rate_1h: &str,
    consecutive_failures: &str,
    observation_count: &str,
    invite_token: Option<&str>,
    explain_reason: &str,
) -> Result<MeshNode, String> {
    let country_code = country_code_raw.to_ascii_uppercase();
    let ttl = country_ttl_sec
        .parse::<u64>()
        .map_err(|_| format!("invalid country_ttl_sec: {country_ttl_sec}"))?;
    let conflict = parse_bool_value(country_conflict, "country_conflict")?;
    let country = if country_code == MeshNodeCountry::UNKNOWN_CODE {
        MeshNodeCountry::unknown(country_updated_at, ttl)
    } else {
        MeshNodeCountry {
            country_code,
            country_name: country_name.to_string(),
            country_source: MeshNodeCountrySource::parse(country_source)?,
            country_confidence: MeshNodeCountryConfidence::parse(country_confidence)?,
            country_updated_at: country_updated_at.to_string(),
            country_ttl_sec: ttl,
            country_conflict: conflict,
            country_conflict_reason: country_conflict_reason.map(str::to_string),
        }
    };
    let node = MeshNode {
        node_id: MeshNodeId::new(id),
        endpoint: endpoint.to_string(),
        invite_token: invite_token
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        country,
        status: MeshNodeStatus::parse(status)?,
        latency_ms: parse_optional_f64(latency_ms)?,
        jitter_ms: parse_optional_f64(jitter_ms)?,
        loss_pct: parse_optional_f64(loss_pct)?,
        success_rate_5m: parse_optional_f64(success_rate_5m)?,
        success_rate_1h: parse_optional_f64(success_rate_1h)?,
        consecutive_failures: parse_u32(consecutive_failures, "consecutive_failures")?,
        observation_count: parse_u32(observation_count, "observation_count")?,
        score: 0.0,
        explain_reason: explain_reason.to_string(),
    };
    node.validate()?;
    Ok(node)
}

fn parse_required_ids(raw: &RawConfig) -> Result<Vec<String>, String> {
    let ids = required(raw, "mesh.nodes.ids")?
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Err("mesh.nodes.ids must contain at least one node id".to_string());
    }
    let mut seen = BTreeSet::new();
    for id in &ids {
        validate_config_id(id)?;
        if !seen.insert(id.clone()) {
            return Err(format!("duplicate node_id in mesh.nodes.ids: {id}"));
        }
    }
    Ok(ids)
}

fn validate_config_keys(raw: &RawConfig, ids: &[String]) -> Result<(), String> {
    let ids = ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for key in raw.keys() {
        if matches!(
            key,
            "mesh.nodes.ids"
                | "mesh.nodes.self_node_id"
                | "mesh.nodes.identity_state_path"
                | "mesh.nodes.activation_log_path"
                | "mesh.nodes.runtime_state_path"
                | "mesh.nodes.current"
                | "mesh.nodes.pinned"
                | "mesh.nodes.autoconnect"
        ) {
            continue;
        }
        let Some(rest) = key.strip_prefix("mesh.node.") else {
            return Err(format!("unknown mesh nodes config key: {key}"));
        };
        let Some((id, field)) = rest.rsplit_once('.') else {
            return Err(format!("invalid mesh node config key: {key}"));
        };
        if !ids.contains(id) {
            return Err(format!(
                "mesh node key references id not listed in mesh.nodes.ids: {id}"
            ));
        }
        if !is_allowed_node_field(field) {
            return Err(format!("unknown mesh node field '{field}' in key {key}"));
        }
    }
    Ok(())
}

fn is_allowed_node_field(field: &str) -> bool {
    matches!(
        field,
        "endpoint"
            | "country_code"
            | "country_name"
            | "country_source"
            | "country_confidence"
            | "country_updated_at"
            | "country_ttl_sec"
            | "country_conflict"
            | "country_conflict_reason"
            | "status"
            | "latency_ms"
            | "jitter_ms"
            | "loss_pct"
            | "success_rate_5m"
            | "success_rate_1h"
            | "consecutive_failures"
            | "observation_count"
            | "invite_token"
            | "explain_reason"
    )
}

fn validate_config_id(id: &str) -> Result<(), String> {
    if id.trim().is_empty() || id.contains('.') || id.chars().any(|ch| ch.is_whitespace()) {
        return Err(format!("invalid mesh node id for config key syntax: {id}"));
    }
    MeshNodeId::new(id).validate()
}

fn required<'a>(raw: &'a RawConfig, key: &str) -> Result<&'a str, String> {
    raw.get(key)
        .ok_or_else(|| format!("missing required mesh nodes config key: {key}"))
}

fn optional_node_id(raw: &RawConfig, key: &str) -> Result<Option<MeshNodeId>, String> {
    let Some(value) = raw.get(key) else {
        return Ok(None);
    };
    if value.trim().is_empty() || value == "none" {
        return Ok(None);
    }
    validate_config_id(value)?;
    Ok(Some(MeshNodeId::new(value)))
}

fn optional_bool(raw: &RawConfig, key: &str) -> Result<Option<bool>, String> {
    raw.get(key)
        .map(|value| parse_bool_value(value, key))
        .transpose()
}

fn parse_bool_value(value: &str, label: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("{label} must be true or false")),
    }
}

fn load_upstream_bootstrap_nodes() -> Result<Vec<MeshNode>, String> {
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

fn should_bootstrap_from_upstream(args: &[String], config_path: Option<&str>) -> bool {
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

fn merge_cli_nodes(
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

fn retain_reachable_nodes(nodes: Vec<MeshNode>, timeout_ms: u64) -> Vec<MeshNode> {
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
