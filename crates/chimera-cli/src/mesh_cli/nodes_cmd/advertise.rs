use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use chimera_mesh::{MeshNodeId, validate_update_bootstrap_url};
use ring::{
    rand::SystemRandom,
    signature::{Ed25519KeyPair, KeyPair},
};

use crate::mesh_cli::nodes_inventory::{self, MeshNodesInventory, extract_flag_value};

use super::basic::{selected_node_endpoint, selected_node_invite_token};

pub(super) fn advertise_node(args: &[String], inventory: &MeshNodesInventory) -> i32 {
    let Some(out_path) = extract_flag_value(args, "--out") else {
        eprintln!("mesh nodes advertise error: --out is required");
        return 2;
    };
    let Some(discovery_pubkey_out) = extract_flag_value(args, "--pubkey-out") else {
        eprintln!("mesh nodes advertise error: --pubkey-out is required");
        return 2;
    };
    let node_id = match resolve_advertise_node_id(args, inventory) {
        Ok(node_id) => node_id,
        Err(error) => {
            eprintln!("mesh nodes advertise error: {error}");
            return 2;
        }
    };
    let endpoint = match resolve_advertise_endpoint(args, inventory) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            eprintln!("mesh nodes advertise error: {error}");
            return 2;
        }
    };
    let update_bootstrap_url =
        match resolve_advertise_update_bootstrap_url(args, inventory, &node_id) {
            Ok(update_bootstrap_url) => update_bootstrap_url,
            Err(error) => {
                eprintln!("mesh nodes advertise error: {error}");
                return 2;
            }
        };
    let country_code = extract_flag_value(args, "--country-code")
        .map(str::to_string)
        .or_else(|| {
            inventory
                .nodes
                .iter()
                .find(|node| node.node_id.0 == node_id)
                .map(|node| node.country.country_code.clone())
        })
        .unwrap_or_else(|| "ZZ".to_string());
    let country_name = extract_flag_value(args, "--country-name")
        .map(str::to_string)
        .or_else(|| {
            inventory
                .nodes
                .iter()
                .find(|node| node.node_id.0 == node_id)
                .map(|node| node.country.country_name.clone())
        })
        .unwrap_or_else(|| "Unknown".to_string());
    let region = extract_flag_value(args, "--region")
        .map(str::to_string)
        .or_else(|| {
            inventory
                .nodes
                .iter()
                .find(|node| node.node_id.0 == node_id)
                .map(|node| node.country.country_code.to_ascii_lowercase())
        })
        .unwrap_or_else(|| "global".to_string());
    let topic = extract_flag_value(args, "--topic")
        .map(str::to_string)
        .unwrap_or_else(|| "mesh-node".to_string());
    let ttl_sec = extract_flag_value(args, "--ttl-sec")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(900)
        .max(1);
    let discovery_keypair_path = resolve_discovery_keypair_path(args);
    let keypair = match load_or_create_discovery_signing_key(&discovery_keypair_path) {
        Ok(keypair) => keypair,
        Err(error) => {
            eprintln!("mesh nodes advertise error: {error}");
            return 2;
        }
    };
    let pubkey_b64 =
        base64::engine::general_purpose::STANDARD.encode(keypair.public_key().as_ref());
    let now_unix = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_secs(),
        Err(error) => {
            eprintln!("mesh nodes advertise error: system clock error: {error}");
            return 2;
        }
    };
    let expires_at_unix = now_unix.saturating_add(ttl_sec);
    let nonce = format!("advertise-{node_id}-{now_unix}");
    let node = serde_json::json!({
        "node_id": &node_id,
        "endpoint": &endpoint,
        "country_code": &country_code,
        "country_name": &country_name,
        "status": "healthy",
        "country_source": "operator_override",
        "country_confidence": "high",
        "country_updated_at": "auto",
        "country_ttl_sec": ttl_sec,
        "country_conflict": false,
        "country_conflict_reason": null,
        "region": &region,
        "topic": &topic,
        "invite_token": selected_node_invite_token(inventory),
        "update_bootstrap_url": update_bootstrap_url.as_deref(),
        "freshness_unix": now_unix,
        "ttl_sec": ttl_sec,
        "capabilities": ["node", "transit", "mesh"],
    });
    let nodes = serde_json::json!([node]);
    let message = match nodes_inventory::build_discovery_signature_message(
        1,
        now_unix,
        expires_at_unix,
        &nonce,
        &nodes,
    ) {
        Ok(message) => message,
        Err(error) => {
            eprintln!("mesh nodes advertise error: {error}");
            return 2;
        }
    };
    let signature = keypair.sign(&message);
    let envelope = serde_json::json!({
        "contract_version": 1,
        "issued_at_unix": now_unix,
        "expires_at_unix": expires_at_unix,
        "key_id": "default",
        "nonce": nonce,
        "nodes": nodes,
        "signature": base64::engine::general_purpose::STANDARD.encode(signature.as_ref()),
    });
    if let Err(error) = write_discovery_artifacts(
        out_path,
        discovery_pubkey_out,
        &pubkey_b64,
        &envelope.to_string(),
    ) {
        eprintln!("mesh nodes advertise error: {error}");
        return 2;
    }
    println!("mesh_nodes_advertise=ok out={out_path} pubkey_out={discovery_pubkey_out}");
    println!("mesh_nodes_advertise_endpoint={endpoint}");
    if update_bootstrap_url.is_some() {
        println!("mesh_nodes_advertise_update_bootstrap_url=present");
    }
    println!("mesh_nodes_advertise_node_id={node_id}");
    0
}

fn resolve_advertise_node_id(
    args: &[String],
    inventory: &MeshNodesInventory,
) -> Result<String, String> {
    if let Some(id) = extract_flag_value(args, "--node-id") {
        let id = id.trim();
        if id.is_empty() {
            return Err("mesh nodes advertise error: --node-id is empty".to_string());
        }
        MeshNodeId::new(id).validate()?;
        return Ok(id.to_string());
    }
    if let Some(id) = inventory.self_node_id.as_ref() {
        return Ok(id.0.clone());
    }
    if let Ok(host) = std::env::var("HOSTNAME") {
        let host = host.trim();
        if !host.is_empty() {
            let sanitized = sanitize_node_id(host);
            if !sanitized.is_empty() {
                return Ok(sanitized);
            }
        }
    }
    Err("mesh nodes advertise error: cannot resolve node id (use --node-id or mesh.nodes.self_node_id)".to_string())
}

fn sanitize_node_id(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

fn resolve_advertise_endpoint(
    args: &[String],
    inventory: &MeshNodesInventory,
) -> Result<String, String> {
    if let Some(endpoint) = extract_flag_value(args, "--endpoint") {
        let endpoint = endpoint.trim();
        if endpoint.is_empty() {
            return Err("mesh nodes advertise error: --endpoint is empty".to_string());
        }
        return Ok(endpoint.to_string());
    }
    if let Some(endpoint) = selected_node_endpoint(inventory) {
        let endpoint = endpoint.trim();
        if !endpoint.is_empty() {
            return Ok(endpoint.to_string());
        }
    }
    let state_path = extract_flag_value(args, "--state-file")
        .map(str::to_string)
        .or_else(|| std::env::var("CHIMERA_MESH_PEER_EGRESS_STATE_PATH").ok());
    if let Some(state_path) = state_path
        && let Ok(text) = std::fs::read_to_string(state_path)
    {
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("resolved_peer_listen=") {
                let endpoint = rest.trim();
                if !endpoint.is_empty() {
                    return Ok(endpoint.to_string());
                }
            }
        }
    }
    Err("mesh nodes advertise error: cannot resolve endpoint (use --endpoint or current selected endpoint)".to_string())
}

fn resolve_advertise_update_bootstrap_url(
    args: &[String],
    inventory: &MeshNodesInventory,
    node_id: &str,
) -> Result<Option<String>, String> {
    if let Some(url) = extract_flag_value(args, "--update-bootstrap-url") {
        return validate_advertise_update_bootstrap_url(url).map(Some);
    }
    if let Ok(url) = std::env::var("CHIMERA_MESH_UPDATE_BOOTSTRAP_URL")
        && !url.trim().is_empty()
    {
        return validate_advertise_update_bootstrap_url(&url).map(Some);
    }
    for path in resolve_update_state_paths(args) {
        if let Some(url) = read_update_bootstrap_url_from_state(&path)? {
            return validate_advertise_update_bootstrap_url(&url).map(Some);
        }
    }
    if let Some(node) = inventory
        .nodes
        .iter()
        .find(|node| node.node_id.0 == node_id)
        && let Some(url) = node.update_bootstrap_url.as_deref()
    {
        return validate_advertise_update_bootstrap_url(url).map(Some);
    }
    Ok(None)
}

fn resolve_update_state_paths(args: &[String]) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(path) = extract_flag_value(args, "--update-state-file")
        && !path.trim().is_empty()
    {
        paths.push(path.to_string());
    }
    if let Ok(path) = std::env::var("CHIMERA_PEER_UPDATE_STATE_FILE")
        && !path.trim().is_empty()
    {
        paths.push(path.trim().to_string());
    }
    if let Some(path) = extract_flag_value(args, "--state-file")
        && !path.trim().is_empty()
    {
        paths.push(path.to_string());
    }
    paths
}

fn read_update_bootstrap_url_from_state(path: &str) -> Result<Option<String>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("read update state failed: {error}")),
    };
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(url) = value
            .get("update_bootstrap_url")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
    {
        return Ok(Some(url.to_string()));
    }
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("update_bootstrap_url=") {
            let url = rest.trim();
            if !url.is_empty() {
                return Ok(Some(url.to_string()));
            }
        }
    }
    Ok(None)
}

fn validate_advertise_update_bootstrap_url(url: &str) -> Result<String, String> {
    validate_update_bootstrap_url(url)?;
    let host = update_url_host(url)?;
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "localhost"
        || host_lower == "0.0.0.0"
        || host_lower == "::"
        || host_lower == "::1"
        || host_lower.starts_with("127.")
    {
        return Err(
            "update_bootstrap_url must be externally reachable, not loopback or unspecified"
                .to_string(),
        );
    }
    Ok(url.to_string())
}

fn update_url_host(url: &str) -> Result<&str, String> {
    let without_scheme = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .ok_or_else(|| "update_bootstrap_url must be http(s)".to_string())?;
    let authority = without_scheme
        .split('/')
        .next()
        .ok_or_else(|| "update_bootstrap_url missing host".to_string())?;
    if authority.is_empty() {
        return Err("update_bootstrap_url missing host".to_string());
    }
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, _) = rest
            .split_once(']')
            .ok_or_else(|| "update_bootstrap_url invalid IPv6 host".to_string())?;
        if host.is_empty() {
            return Err("update_bootstrap_url missing host".to_string());
        }
        return Ok(host);
    }
    let host = authority.split(':').next().unwrap_or(authority);
    if host.is_empty() {
        return Err("update_bootstrap_url missing host".to_string());
    }
    Ok(host)
}

fn resolve_discovery_keypair_path(args: &[String]) -> PathBuf {
    if let Some(path) = extract_flag_value(args, "--keypair-path") {
        return PathBuf::from(path);
    }
    if let Ok(path) = std::env::var("CHIMERA_MESH_DISCOVERY_KEYPAIR_PATH")
        && !path.trim().is_empty()
    {
        return PathBuf::from(path);
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|home| format!("{home}/.config"))
                .unwrap_or_else(|| ".config".to_string())
        });
    PathBuf::from(base).join("chimera/discovery_signing.keypair")
}

fn load_or_create_discovery_signing_key(path: &Path) -> Result<Ed25519KeyPair, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create key dir failed: {error}"))?;
    }
    if path.exists() {
        let raw = std::fs::read_to_string(path)
            .map_err(|error| format!("read discovery keypair failed: {error}"))?;
        let pkcs8_b64 = raw
            .lines()
            .find_map(|line| line.strip_prefix("pkcs8_base64="))
            .ok_or_else(|| "discovery keypair file missing pkcs8_base64".to_string())?;
        let pkcs8 = base64::engine::general_purpose::STANDARD
            .decode(pkcs8_b64.trim())
            .map_err(|error| format!("decode discovery keypair failed: {error}"))?;
        return Ed25519KeyPair::from_pkcs8(&pkcs8)
            .map_err(|_| "parse discovery keypair failed".to_string());
    }
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
        .map_err(|_| "discovery keypair generation failed".to_string())?;
    let keypair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())
        .map_err(|_| "discovery keypair parse failed".to_string())?;
    let material = format!(
        "kind=mesh_discovery_keypair\nalgorithm=ed25519\npkcs8_base64={}\n",
        base64::engine::general_purpose::STANDARD.encode(pkcs8.as_ref())
    );
    std::fs::write(path, material)
        .map_err(|error| format!("write discovery keypair failed: {error}"))?;
    Ok(keypair)
}

fn write_discovery_artifacts(
    out_path: &str,
    pubkey_out_path: &str,
    pubkey_b64: &str,
    json: &str,
) -> Result<(), String> {
    if let Some(parent) = Path::new(out_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create discovery dir failed: {error}"))?;
    }
    if let Some(parent) = Path::new(pubkey_out_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create pubkey dir failed: {error}"))?;
    }
    std::fs::write(out_path, json)
        .map_err(|error| format!("write discovery out failed: {error}"))?;
    std::fs::write(pubkey_out_path, format!("{pubkey_b64}\n"))
        .map_err(|error| format!("write pubkey out failed: {error}"))?;
    Ok(())
}
