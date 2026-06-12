use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use base64::Engine as _;
use chimera_config::RawConfig;
use chimera_mesh::{MeshNode, MeshNodeCountry};
use ring::signature::{ED25519, UnparsedPublicKey};

use super::parse::build_node;
use super::{config_string_value, extract_flag_value};

pub(super) fn discovery_url_from_env() -> Option<String> {
    match env::var("CHIMERA_MESH_NODES_DISCOVERY_URL") {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

pub(super) fn discovery_pubkey_from_env() -> Option<String> {
    match env::var("CHIMERA_MESH_NODES_DISCOVERY_PUBKEY") {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

pub(super) fn config_discovery_url(args: &[String]) -> Option<String> {
    let path = extract_flag_value(args, "--config")?;
    let text = fs::read_to_string(path).ok()?;
    let raw = RawConfig::parse(&text).ok()?;
    raw.get("mesh.nodes.discovery_url").map(str::to_string)
}

pub(super) fn config_discovery_pubkey(args: &[String]) -> Option<String> {
    config_string_value(args, "mesh.nodes.discovery_pubkey")
}

pub(super) fn fetch_discovery_nodes(
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

pub(super) fn parse_discovery_keyring(
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
        if let Some(value) = record.get(*key).and_then(serde_json::Value::as_str)
            && !value.trim().is_empty()
        {
            return Ok(value.to_string());
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
