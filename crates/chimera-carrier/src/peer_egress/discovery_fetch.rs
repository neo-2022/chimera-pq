use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use ring::signature::{ED25519, UnparsedPublicKey};
use serde_json::Value;

/// A node record returned by a verified mesh discovery snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteMeshNode {
    pub node_id: String,
    pub endpoint: String,
    pub update_bootstrap_url: Option<String>,
    pub endpoint_generation: Option<u64>,
    pub country_code: String,
    pub loss_pct: Option<f64>,
    pub success_rate_1h: Option<f64>,
}

impl RemoteMeshNode {
    pub fn region(&self) -> String {
        self.country_code.to_ascii_uppercase()
    }

    pub fn load_score(&self) -> u8 {
        self.loss_pct.unwrap_or(20.0).round().clamp(0.0, 100.0) as u8
    }

    pub fn reliability_score(&self) -> u8 {
        self.success_rate_1h
            .unwrap_or(90.0)
            .round()
            .clamp(0.0, 100.0) as u8
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveryFetchOptions {
    pub urls: Vec<String>,
    pub keyring: BTreeMap<String, String>,
    pub revoked_key_ids: BTreeSet<String>,
    pub revoked_node_ids: BTreeSet<String>,
    pub timeout_ms: u64,
}

pub fn fetch_discovery_nodes(
    options: &DiscoveryFetchOptions,
) -> Result<Vec<RemoteMeshNode>, String> {
    let mut retryable_errors = Vec::new();
    for url in &options.urls {
        match fetch_discovery_nodes_from_url(url, options) {
            Ok(nodes) => return Ok(nodes),
            Err(error) if discovery_fetch_error_is_retryable(&error) => {
                retryable_errors.push(format!("{url}: {error}"));
            }
            Err(error) => return Err(error),
        }
    }
    if retryable_errors.is_empty() {
        return Ok(Vec::new());
    }
    Err(format!(
        "mesh discovery request failed for all sources: {}",
        retryable_errors.join("; ")
    ))
}

fn fetch_discovery_nodes_from_url(
    url: &str,
    options: &DiscoveryFetchOptions,
) -> Result<Vec<RemoteMeshNode>, String> {
    let response = ureq::get(url)
        .timeout(std::time::Duration::from_millis(options.timeout_ms))
        .call()
        .map_err(|error| format!("mesh discovery request failed: {error}"))?;
    let text = response
        .into_string()
        .map_err(|error| format!("mesh discovery read body failed: {error}"))?;
    parse_discovery_nodes_json(
        &text,
        &options.keyring,
        &options.revoked_key_ids,
        &options.revoked_node_ids,
    )
}

fn parse_discovery_nodes_json(
    input: &str,
    discovery_keyring: &BTreeMap<String, String>,
    revoked_key_ids: &BTreeSet<String>,
    revoked_node_ids: &BTreeSet<String>,
) -> Result<Vec<RemoteMeshNode>, String> {
    let value: Value = serde_json::from_str(input)
        .map_err(|error| format!("mesh discovery json parse failed: {error}"))?;
    let nodes_value = parse_discovery_envelope(&value, discovery_keyring, revoked_key_ids)?;
    let records = nodes_value
        .as_array()
        .ok_or_else(|| "mesh discovery payload must contain 'nodes' array".to_string())?;
    let mut out = Vec::with_capacity(records.len());
    let mut ids = BTreeSet::new();
    for record in records {
        let node = parse_discovery_node_record(record)?;
        if revoked_node_ids.contains(&node.node_id) {
            continue;
        }
        if !ids.insert(node.node_id.clone()) {
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
    value: &'a Value,
    discovery_keyring: &BTreeMap<String, String>,
    revoked_key_ids: &BTreeSet<String>,
) -> Result<&'a Value, String> {
    const CONTRACT_VERSION: u64 = 1;
    const MAX_CLOCK_SKEW_SEC: u64 = 120;
    let object = value
        .as_object()
        .ok_or_else(|| "mesh discovery payload must be object".to_string())?;
    let contract_version = object
        .get("contract_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| "mesh discovery envelope missing 'contract_version'".to_string())?;
    if contract_version != CONTRACT_VERSION {
        return Err(format!(
            "mesh discovery unsupported contract_version: {contract_version}"
        ));
    }
    let issued_at = object
        .get("issued_at_unix")
        .and_then(Value::as_u64)
        .ok_or_else(|| "mesh discovery envelope missing 'issued_at_unix'".to_string())?;
    let expires_at = object
        .get("expires_at_unix")
        .and_then(Value::as_u64)
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
        .and_then(Value::as_str)
        .ok_or_else(|| "mesh discovery envelope missing 'nonce'".to_string())?;
    if nonce.trim().is_empty() {
        return Err("mesh discovery envelope nonce must be non-empty".to_string());
    }
    let key_id = object
        .get("key_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "mesh discovery envelope missing 'key_id'".to_string())?;
    if key_id.trim().is_empty() {
        return Err("mesh discovery envelope key_id must be non-empty".to_string());
    }
    if revoked_key_ids.contains(key_id) {
        return Err(format!("mesh discovery rejected revoked key_id: {key_id}"));
    }
    let nodes = object
        .get("nodes")
        .ok_or_else(|| "mesh discovery envelope missing 'nodes'".to_string())?;
    let message =
        build_discovery_signature_message(contract_version, issued_at, expires_at, nonce, nodes)?;
    let discovery_pubkey = discovery_keyring
        .get(key_id)
        .ok_or_else(|| format!("mesh discovery unknown key_id: {key_id}"))?;
    verify_discovery_signature(discovery_pubkey, signature_from_object(object)?, &message)?;
    Ok(nodes)
}

fn signature_from_object(object: &serde_json::Map<String, Value>) -> Result<&str, String> {
    let signature = object
        .get("signature")
        .and_then(Value::as_str)
        .ok_or_else(|| "mesh discovery envelope missing 'signature'".to_string())?;
    if signature.trim().is_empty() {
        return Err("mesh discovery envelope signature must be non-empty".to_string());
    }
    Ok(signature)
}

fn current_unix_seconds() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("system clock error: {error}"))
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

fn build_discovery_signature_message(
    contract_version: u64,
    issued_at_unix: u64,
    expires_at_unix: u64,
    nonce: &str,
    nodes: &Value,
) -> Result<Vec<u8>, String> {
    let nodes_compact = serde_json::to_string(nodes)
        .map_err(|error| format!("mesh discovery nodes serialize failed: {error}"))?;
    Ok(format!(
        "contract_version={contract_version}\nissued_at_unix={issued_at_unix}\nexpires_at_unix={expires_at_unix}\nnonce={nonce}\nnodes={nodes_compact}\n"
    )
    .into_bytes())
}

fn parse_discovery_node_record(record: &Value) -> Result<RemoteMeshNode, String> {
    let node_id = json_string(record, &["node_id", "id"])?;
    let endpoint = json_string(record, &["endpoint"])?;
    let update_bootstrap_url = json_optional_string(record, &["update_bootstrap_url"]);
    let endpoint_generation = json_optional_u64(record, &["endpoint_generation"])?;
    let country_code = json_string_default(record, &["country_code"], "UN");
    let loss_pct = json_optional_f64(record, &["loss_pct"]);
    let success_rate_1h = json_optional_f64(record, &["success_rate_1h"]);
    validate_endpoint_host_port(&endpoint)?;
    if node_id.trim().is_empty() {
        return Err("mesh discovery node_id is empty".to_string());
    }
    Ok(RemoteMeshNode {
        node_id,
        endpoint,
        update_bootstrap_url,
        endpoint_generation,
        country_code,
        loss_pct,
        success_rate_1h,
    })
}

fn validate_endpoint_host_port(endpoint: &str) -> Result<(), String> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return Err("mesh discovery endpoint is empty".to_string());
    }
    let (host, port_raw) = endpoint
        .rsplit_once(':')
        .ok_or_else(|| "mesh discovery endpoint must be host:port".to_string())?;
    if host.trim().is_empty() {
        return Err("mesh discovery endpoint host is empty".to_string());
    }
    let port = port_raw
        .parse::<u16>()
        .map_err(|_| "mesh discovery endpoint port is invalid".to_string())?;
    if port == 0 {
        return Err("mesh discovery endpoint port must be non-zero".to_string());
    }
    Ok(())
}

fn json_string(record: &Value, keys: &[&str]) -> Result<String, String> {
    for key in keys {
        if let Some(value) = record.get(*key).and_then(Value::as_str)
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

fn json_string_default(record: &Value, keys: &[&str], default: &str) -> String {
    json_string(record, keys).unwrap_or_else(|_| default.to_string())
}

fn json_optional_string(record: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| record.get(*key).and_then(Value::as_str).map(str::to_string))
}

fn json_optional_f64(record: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| record.get(*key).and_then(Value::as_f64))
}

fn json_optional_u64(record: &Value, keys: &[&str]) -> Result<Option<u64>, String> {
    for key in keys {
        if let Some(value) = record.get(*key) {
            let generation = value
                .as_u64()
                .ok_or_else(|| format!("mesh discovery record field {key} must be u64"))?;
            if generation == 0 {
                return Err(format!("mesh discovery record field {key} must be > 0"));
            }
            return Ok(Some(generation));
        }
    }
    Ok(None)
}

fn discovery_fetch_error_is_retryable(error: &str) -> bool {
    error.starts_with("mesh discovery request failed:")
        || error.starts_with("mesh discovery read body failed:")
}

#[cfg(test)]
#[path = "discovery_fetch_tests.rs"]
mod tests;
