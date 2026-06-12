use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use chimera_mesh::MeshNodeId;
use ring::{
    rand::SystemRandom,
    signature::{ED25519, Ed25519KeyPair, KeyPair, UnparsedPublicKey},
};

use crate::mesh_cli::nodes_inventory::{MeshNodesInventory, extract_flag_value};

pub(super) fn re_enroll_node(args: &[String], inventory: &MeshNodesInventory) -> i32 {
    let Some(current) = inventory.self_node_id.as_ref() else {
        eprintln!("mesh nodes re-enroll error: self node id is not configured");
        return 2;
    };
    let Some(new_node_id) = extract_flag_value(args, "--new-node-id") else {
        eprintln!("mesh nodes re-enroll error: --new-node-id is required");
        return 2;
    };
    if let Err(error) = MeshNodeId::new(new_node_id).validate() {
        eprintln!("mesh nodes re-enroll error: {error}");
        return 2;
    }
    if current.0 == new_node_id {
        eprintln!(
            "mesh nodes re-enroll error: --new-node-id must differ from current self node id"
        );
        return 2;
    }
    let json = match build_re_enroll_request_json(current.0.as_str(), new_node_id, inventory) {
        Ok(json) => json,
        Err(error) => {
            eprintln!("mesh nodes re-enroll error: {error}");
            return 2;
        }
    };
    if let Some(out_path) = extract_flag_value(args, "--out") {
        if let Err(error) = std::fs::write(out_path, &json) {
            eprintln!("mesh nodes re-enroll error: write failed: {error}");
            return 2;
        }
        println!("re_enroll=request_written out={out_path}");
        return 0;
    }
    println!("{json}");
    0
}

fn build_re_enroll_request_json(
    current_node_id: &str,
    new_node_id: &str,
    inventory: &MeshNodesInventory,
) -> Result<String, String> {
    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_secs();
    let restricted = inventory.restricted_reason.is_some();
    let restricted_reason = inventory
        .restricted_reason
        .as_deref()
        .unwrap_or("none")
        .replace('"', "'");
    Ok(format!(
        "{{\"kind\":\"mesh_reenroll_request\",\"status\":\"accepted\",\"current_node_id\":\"{}\",\"new_node_id\":\"{}\",\"restricted_mode\":{},\"restricted_reason\":\"{}\",\"issued_at_unix\":{},\"next_step\":\"issue_new_keypair_and_register\"}}",
        current_node_id,
        new_node_id,
        if restricted { "true" } else { "false" },
        restricted_reason,
        now_unix
    ))
}

pub(super) fn re_enroll_prepare(args: &[String]) -> i32 {
    let Some(request_path) = extract_flag_value(args, "--request") else {
        eprintln!("mesh nodes re-enroll-prepare error: --request is required");
        return 2;
    };
    let Some(out_path) = extract_flag_value(args, "--out") else {
        eprintln!("mesh nodes re-enroll-prepare error: --out is required");
        return 2;
    };
    let Some(key_out_path) = extract_flag_value(args, "--key-out") else {
        eprintln!("mesh nodes re-enroll-prepare error: --key-out is required");
        return 2;
    };
    let request_text = match std::fs::read_to_string(request_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh nodes re-enroll-prepare error: read request failed: {error}");
            return 2;
        }
    };
    let request: serde_json::Value = match serde_json::from_str(&request_text) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh nodes re-enroll-prepare error: invalid request json: {error}");
            return 2;
        }
    };
    let Some(kind) = request.get("kind").and_then(serde_json::Value::as_str) else {
        eprintln!("mesh nodes re-enroll-prepare error: request missing kind");
        return 2;
    };
    if kind != "mesh_reenroll_request" {
        eprintln!("mesh nodes re-enroll-prepare error: unsupported kind '{kind}'");
        return 2;
    }
    let Some(new_node_id) = request
        .get("new_node_id")
        .and_then(serde_json::Value::as_str)
    else {
        eprintln!("mesh nodes re-enroll-prepare error: request missing new_node_id");
        return 2;
    };
    if let Err(error) = MeshNodeId::new(new_node_id).validate() {
        eprintln!("mesh nodes re-enroll-prepare error: {error}");
        return 2;
    }
    let now_unix = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_secs(),
        Err(error) => {
            eprintln!("mesh nodes re-enroll-prepare error: system clock error: {error}");
            return 2;
        }
    };
    let nonce = format!("{new_node_id}-{now_unix}");
    let keypair_pkcs8 = match Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("mesh nodes re-enroll-prepare error: keypair generation failed");
            return 2;
        }
    };
    let keypair = match Ed25519KeyPair::from_pkcs8(keypair_pkcs8.as_ref()) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("mesh nodes re-enroll-prepare error: keypair parse failed");
            return 2;
        }
    };
    let message = format!(
        "kind=mesh_reenroll_register\nnode_id={new_node_id}\nissued_at_unix={now_unix}\nnonce={nonce}\n"
    );
    let signature = keypair.sign(message.as_bytes());
    let pubkey_b64 =
        base64::engine::general_purpose::STANDARD.encode(keypair.public_key().as_ref());
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.as_ref());
    let key_pkcs8_b64 = base64::engine::general_purpose::STANDARD.encode(keypair_pkcs8.as_ref());
    let register_json = format!(
        "{{\"kind\":\"mesh_reenroll_register\",\"node_id\":\"{}\",\"pubkey\":\"{}\",\"issued_at_unix\":{},\"nonce\":\"{}\",\"proof_signature\":\"{}\",\"proof_message\":\"{}\"}}",
        new_node_id,
        pubkey_b64,
        now_unix,
        nonce,
        signature_b64,
        message.replace('\n', "\\n")
    );
    let key_json = format!(
        "{{\"kind\":\"mesh_reenroll_key_material\",\"node_id\":\"{}\",\"algorithm\":\"ed25519\",\"pkcs8_base64\":\"{}\"}}",
        new_node_id, key_pkcs8_b64
    );
    if let Err(error) = std::fs::write(out_path, register_json) {
        eprintln!("mesh nodes re-enroll-prepare error: write --out failed: {error}");
        return 2;
    }
    if let Err(error) = std::fs::write(key_out_path, key_json) {
        eprintln!("mesh nodes re-enroll-prepare error: write --key-out failed: {error}");
        return 2;
    }
    println!(
        "re_enroll_prepare=ok out={} key_out={}",
        out_path, key_out_path
    );
    0
}

pub(super) fn re_enroll_submit(args: &[String]) -> i32 {
    let Some(register_path) = extract_flag_value(args, "--register") else {
        eprintln!("mesh nodes re-enroll-submit error: --register is required");
        return 2;
    };
    let Some(key_path) = extract_flag_value(args, "--key") else {
        eprintln!("mesh nodes re-enroll-submit error: --key is required");
        return 2;
    };
    let Some(state_out_path) = resolve_identity_state_out_path(args) else {
        eprintln!(
            "mesh nodes re-enroll-submit error: --state-out (or config/env identity_state_path) is required"
        );
        return 2;
    };
    let register_text = match std::fs::read_to_string(register_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh nodes re-enroll-submit error: read register failed: {error}");
            return 2;
        }
    };
    let key_text = match std::fs::read_to_string(key_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh nodes re-enroll-submit error: read key failed: {error}");
            return 2;
        }
    };
    let register: serde_json::Value = match serde_json::from_str(&register_text) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh nodes re-enroll-submit error: invalid register json: {error}");
            return 2;
        }
    };
    let key_material: serde_json::Value = match serde_json::from_str(&key_text) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh nodes re-enroll-submit error: invalid key json: {error}");
            return 2;
        }
    };
    let Some(node_id) = register.get("node_id").and_then(serde_json::Value::as_str) else {
        eprintln!("mesh nodes re-enroll-submit error: register missing node_id");
        return 2;
    };
    let Some(pubkey_b64) = register.get("pubkey").and_then(serde_json::Value::as_str) else {
        eprintln!("mesh nodes re-enroll-submit error: register missing pubkey");
        return 2;
    };
    let Some(proof_signature_b64) = register
        .get("proof_signature")
        .and_then(serde_json::Value::as_str)
    else {
        eprintln!("mesh nodes re-enroll-submit error: register missing proof_signature");
        return 2;
    };
    let Some(proof_message) = register
        .get("proof_message")
        .and_then(serde_json::Value::as_str)
    else {
        eprintln!("mesh nodes re-enroll-submit error: register missing proof_message");
        return 2;
    };
    let Some(key_node_id) = key_material
        .get("node_id")
        .and_then(serde_json::Value::as_str)
    else {
        eprintln!("mesh nodes re-enroll-submit error: key material missing node_id");
        return 2;
    };
    if key_node_id != node_id {
        eprintln!(
            "mesh nodes re-enroll-submit error: node_id mismatch between register and key material"
        );
        return 2;
    }
    let Some(pkcs8_b64) = key_material
        .get("pkcs8_base64")
        .and_then(serde_json::Value::as_str)
    else {
        eprintln!("mesh nodes re-enroll-submit error: key material missing pkcs8_base64");
        return 2;
    };
    let pkcs8 = match base64::engine::general_purpose::STANDARD.decode(pkcs8_b64) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "mesh nodes re-enroll-submit error: key material base64 decode failed: {error}"
            );
            return 2;
        }
    };
    let keypair = match Ed25519KeyPair::from_pkcs8(&pkcs8) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("mesh nodes re-enroll-submit error: key material parse failed");
            return 2;
        }
    };
    let derived_pubkey_b64 =
        base64::engine::general_purpose::STANDARD.encode(keypair.public_key().as_ref());
    if derived_pubkey_b64 != pubkey_b64 {
        eprintln!("mesh nodes re-enroll-submit error: pubkey mismatch with key material");
        return 2;
    }
    let proof_signature = match base64::engine::general_purpose::STANDARD
        .decode(proof_signature_b64)
    {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "mesh nodes re-enroll-submit error: proof signature base64 decode failed: {error}"
            );
            return 2;
        }
    };
    let verifier = UnparsedPublicKey::new(&ED25519, keypair.public_key().as_ref());
    if verifier
        .verify(proof_message.as_bytes(), &proof_signature)
        .is_err()
    {
        eprintln!("mesh nodes re-enroll-submit error: proof signature verification failed");
        return 2;
    }
    let now_unix = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_secs(),
        Err(error) => {
            eprintln!("mesh nodes re-enroll-submit error: system clock error: {error}");
            return 2;
        }
    };
    let state_json = format!(
        "{{\"kind\":\"mesh_identity_state\",\"status\":\"active\",\"self_node_id\":\"{}\",\"pubkey\":\"{}\",\"activated_at_unix\":{},\"restricted_mode\":false}}",
        node_id, pubkey_b64, now_unix
    );
    if let Err(error) = std::fs::write(&state_out_path, state_json) {
        eprintln!("mesh nodes re-enroll-submit error: write state failed: {error}");
        return 2;
    }
    if let Some(activation_out_path) = resolve_activation_out_path(args) {
        let activation_json = format!(
            "{{\"kind\":\"mesh_reenroll_activation\",\"status\":\"active\",\"self_node_id\":\"{}\",\"activated_at_unix\":{},\"source\":\"re_enroll_submit\"}}",
            node_id, now_unix
        );
        if let Err(error) = std::fs::write(&activation_out_path, activation_json) {
            eprintln!("mesh nodes re-enroll-submit error: write activation failed: {error}");
            return 2;
        }
        println!(
            "re_enroll_submit=ok state_out={} activation_out={}",
            state_out_path, activation_out_path
        );
        return 0;
    }
    println!("re_enroll_submit=ok state_out={state_out_path}");
    0
}

fn resolve_identity_state_out_path(args: &[String]) -> Option<String> {
    extract_flag_value(args, "--state-out")
        .map(str::to_string)
        .or_else(|| {
            let config_path = extract_flag_value(args, "--config")?;
            let text = std::fs::read_to_string(config_path).ok()?;
            let raw = chimera_config::RawConfig::parse(&text).ok()?;
            raw.get("mesh.nodes.identity_state_path")
                .map(str::to_string)
        })
        .or_else(|| std::env::var("CHIMERA_MESH_IDENTITY_STATE_PATH").ok())
}

fn resolve_activation_out_path(args: &[String]) -> Option<String> {
    extract_flag_value(args, "--activation-out")
        .map(str::to_string)
        .or_else(|| {
            let config_path = extract_flag_value(args, "--config")?;
            let text = std::fs::read_to_string(config_path).ok()?;
            let raw = chimera_config::RawConfig::parse(&text).ok()?;
            raw.get("mesh.nodes.activation_log_path")
                .map(str::to_string)
        })
        .or_else(|| std::env::var("CHIMERA_MESH_ACTIVATION_LOG_PATH").ok())
}
