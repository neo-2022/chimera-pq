use super::helpers::{build_signed_payload, generate_signing_key, now_unix, serve_json_once};
use crate::mesh_cli::nodes_inventory::load_mesh_nodes_inventory;
use base64::Engine as _;
use std::net::TcpListener;

#[test]
fn nodes_inventory_discovery_contract_accepts_valid_envelope() {
    let endpoint_listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind endpoint listener failed: {err}"));
    let endpoint_addr = endpoint_listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read endpoint listener addr failed: {err}"));
    let now = now_unix();
    let signing_key = generate_signing_key();
    let pubkey_b64 =
        base64::engine::general_purpose::STANDARD.encode(signing_key.verifying_key().as_bytes());
    let nodes = format!(
        "[{{\"node_id\":\"de1\",\"endpoint\":\"{}\",\"country_code\":\"DE\",\"country_name\":\"Germany\",\"status\":\"healthy\"}}]",
        endpoint_addr
    );
    let payload = build_signed_payload(
        &signing_key,
        "default",
        "n-valid-1",
        now.saturating_sub(1),
        now.saturating_add(60),
        &nodes,
    );
    let url = serve_json_once(payload);
    let args = vec![
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--discovery-url".to_string(),
        url,
        "--discovery-pubkey".to_string(),
        pubkey_b64,
    ];
    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(inventory.nodes.len(), 1);
    assert_eq!(inventory.nodes[0].node_id.0, "de1");
}

#[test]
fn nodes_inventory_discovery_contract_preserves_update_bootstrap_url() {
    let endpoint_listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind endpoint listener failed: {err}"));
    let endpoint_addr = endpoint_listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read endpoint listener addr failed: {err}"));
    let now = now_unix();
    let signing_key = generate_signing_key();
    let pubkey_b64 =
        base64::engine::general_purpose::STANDARD.encode(signing_key.verifying_key().as_bytes());
    let nodes = format!(
        "[{{\"node_id\":\"de-update\",\"endpoint\":\"{}\",\"country_code\":\"DE\",\"country_name\":\"Germany\",\"status\":\"healthy\",\"update_bootstrap_url\":\"http://node.example:45678/chimera.sh\"}}]",
        endpoint_addr
    );
    let payload = build_signed_payload(
        &signing_key,
        "default",
        "n-update-url-1",
        now.saturating_sub(1),
        now.saturating_add(60),
        &nodes,
    );
    let url = serve_json_once(payload);
    let args = vec![
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--discovery-url".to_string(),
        url,
        "--discovery-pubkey".to_string(),
        pubkey_b64,
    ];
    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(inventory.nodes.len(), 1);
    assert_eq!(
        inventory.nodes[0].update_bootstrap_url.as_deref(),
        Some("http://node.example:45678/chimera.sh")
    );
    assert_eq!(inventory.nodes[0].endpoint_generation, None);
}

#[test]
fn nodes_inventory_discovery_contract_preserves_endpoint_generation() {
    let endpoint_listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind endpoint listener failed: {err}"));
    let endpoint_addr = endpoint_listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read endpoint listener addr failed: {err}"));
    let now = now_unix();
    let signing_key = generate_signing_key();
    let pubkey_b64 =
        base64::engine::general_purpose::STANDARD.encode(signing_key.verifying_key().as_bytes());
    let nodes = format!(
        "[{{\"node_id\":\"de-update-generation\",\"endpoint\":\"{}\",\"country_code\":\"DE\",\"country_name\":\"Germany\",\"status\":\"healthy\",\"update_bootstrap_url\":\"http://node.example:45678/chimera.sh\",\"endpoint_generation\":11}}]",
        endpoint_addr
    );
    let payload = build_signed_payload(
        &signing_key,
        "default",
        "n-update-generation-1",
        now.saturating_sub(1),
        now.saturating_add(60),
        &nodes,
    );
    let url = serve_json_once(payload);
    let args = vec![
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--discovery-url".to_string(),
        url,
        "--discovery-pubkey".to_string(),
        pubkey_b64,
    ];
    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(inventory.nodes.len(), 1);
    assert_eq!(inventory.nodes[0].endpoint_generation, Some(11));
}

#[test]
fn nodes_inventory_discovery_contract_rejects_zero_endpoint_generation() {
    let endpoint_listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind endpoint listener failed: {err}"));
    let endpoint_addr = endpoint_listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read endpoint listener addr failed: {err}"));
    let now = now_unix();
    let signing_key = generate_signing_key();
    let pubkey_b64 =
        base64::engine::general_purpose::STANDARD.encode(signing_key.verifying_key().as_bytes());
    let nodes = format!(
        "[{{\"node_id\":\"de-zero-generation\",\"endpoint\":\"{}\",\"country_code\":\"DE\",\"country_name\":\"Germany\",\"status\":\"healthy\",\"endpoint_generation\":0}}]",
        endpoint_addr
    );
    let payload = build_signed_payload(
        &signing_key,
        "default",
        "n-zero-generation-1",
        now.saturating_sub(1),
        now.saturating_add(60),
        &nodes,
    );
    let url = serve_json_once(payload);
    let args = vec![
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--discovery-url".to_string(),
        url,
        "--discovery-pubkey".to_string(),
        pubkey_b64,
    ];
    let error = load_mesh_nodes_inventory(&args)
        .err()
        .unwrap_or_else(|| unreachable!("zero endpoint_generation must fail"));
    assert!(error.contains("endpoint_generation"));
}

#[test]
fn nodes_inventory_discovery_contract_rejects_expired_envelope() {
    let now = now_unix();
    let signing_key = generate_signing_key();
    let pubkey_b64 =
        base64::engine::general_purpose::STANDARD.encode(signing_key.verifying_key().as_bytes());
    let payload = build_signed_payload(
        &signing_key,
        "default",
        "n-expired-1",
        now.saturating_sub(120),
        now.saturating_sub(60),
        "[]",
    );
    let url = serve_json_once(payload);
    let args = vec![
        "--discovery-url".to_string(),
        url,
        "--discovery-pubkey".to_string(),
        pubkey_b64,
    ];
    let error = load_mesh_nodes_inventory(&args)
        .err()
        .unwrap_or_else(|| unreachable!("expired payload must fail"));
    assert!(error.contains("expired"));
}

#[test]
fn nodes_inventory_discovery_contract_rejects_replay_nonce() {
    let endpoint_listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind endpoint listener failed: {err}"));
    let endpoint_addr = endpoint_listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read endpoint listener addr failed: {err}"));
    let now = now_unix();
    let signing_key = generate_signing_key();
    let pubkey_b64 =
        base64::engine::general_purpose::STANDARD.encode(signing_key.verifying_key().as_bytes());
    let nodes = format!(
        "[{{\"node_id\":\"de1\",\"endpoint\":\"{}\",\"country_code\":\"DE\",\"country_name\":\"Germany\",\"status\":\"healthy\"}}]",
        endpoint_addr
    );
    let payload = build_signed_payload(
        &signing_key,
        "default",
        "n-replay-1",
        now.saturating_sub(1),
        now.saturating_add(60),
        &nodes,
    );
    let url1 = serve_json_once(payload.clone());
    let url2 = serve_json_once(payload);
    let args1 = vec![
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--discovery-url".to_string(),
        url1,
        "--discovery-pubkey".to_string(),
        pubkey_b64.clone(),
    ];
    let args2 = vec![
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--discovery-url".to_string(),
        url2,
        "--discovery-pubkey".to_string(),
        pubkey_b64,
    ];
    let first = load_mesh_nodes_inventory(&args1);
    assert!(first.is_ok());
    let second = load_mesh_nodes_inventory(&args2)
        .err()
        .unwrap_or_else(|| unreachable!("replay nonce must fail"));
    assert!(second.contains("anti-replay"));
}

#[test]
fn nodes_inventory_discovery_contract_rejects_invalid_signature() {
    let endpoint_listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind endpoint listener failed: {err}"));
    let endpoint_addr = endpoint_listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read endpoint listener addr failed: {err}"));
    let now = now_unix();
    let signing_key = generate_signing_key();
    let wrong_key = generate_signing_key();
    let pubkey_b64 =
        base64::engine::general_purpose::STANDARD.encode(wrong_key.verifying_key().as_bytes());
    let nodes = format!(
        "[{{\"node_id\":\"de1\",\"endpoint\":\"{}\",\"country_code\":\"DE\",\"country_name\":\"Germany\",\"status\":\"healthy\"}}]",
        endpoint_addr
    );
    let payload = build_signed_payload(
        &signing_key,
        "default",
        "n-badsig-1",
        now.saturating_sub(1),
        now.saturating_add(60),
        &nodes,
    );
    let url = serve_json_once(payload);
    let args = vec![
        "--discovery-url".to_string(),
        url,
        "--discovery-pubkey".to_string(),
        pubkey_b64,
    ];
    let error = load_mesh_nodes_inventory(&args)
        .err()
        .unwrap_or_else(|| unreachable!("invalid signature must fail"));
    assert!(error.contains("signature verification failed"));
}

#[test]
fn nodes_inventory_discovery_contract_accepts_keyring_rotation() {
    let endpoint_listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind endpoint listener failed: {err}"));
    let endpoint_addr = endpoint_listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read endpoint listener addr failed: {err}"));
    let now = now_unix();
    let key1 = generate_signing_key();
    let key2 = generate_signing_key();
    let keyring = format!(
        "k1:{},k2:{}",
        base64::engine::general_purpose::STANDARD.encode(key1.verifying_key().as_bytes()),
        base64::engine::general_purpose::STANDARD.encode(key2.verifying_key().as_bytes())
    );
    let nodes = format!(
        "[{{\"node_id\":\"rot1\",\"endpoint\":\"{}\",\"country_code\":\"DE\",\"country_name\":\"Germany\",\"status\":\"healthy\"}}]",
        endpoint_addr
    );
    let payload = build_signed_payload(
        &key2,
        "k2",
        "n-rot-1",
        now.saturating_sub(1),
        now.saturating_add(60),
        &nodes,
    );
    let url = serve_json_once(payload);
    let args = vec![
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--discovery-url".to_string(),
        url,
        "--discovery-keyring".to_string(),
        keyring,
    ];
    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(inventory.nodes.len(), 1);
    assert_eq!(inventory.nodes[0].node_id.0, "rot1");
}

#[test]
fn nodes_inventory_discovery_contract_rejects_revoked_key_id() {
    let now = now_unix();
    let key = generate_signing_key();
    let keyring = format!(
        "k1:{}",
        base64::engine::general_purpose::STANDARD.encode(key.verifying_key().as_bytes())
    );
    let payload = build_signed_payload(
        &key,
        "k1",
        "n-revkey-1",
        now.saturating_sub(1),
        now.saturating_add(60),
        "[]",
    );
    let url = serve_json_once(payload);
    let args = vec![
        "--discovery-url".to_string(),
        url,
        "--discovery-keyring".to_string(),
        keyring,
        "--discovery-revoked-key-ids".to_string(),
        "k1".to_string(),
    ];
    let error = load_mesh_nodes_inventory(&args)
        .err()
        .unwrap_or_else(|| unreachable!("revoked key must fail"));
    assert!(error.contains("revoked key_id"));
}

#[test]
fn nodes_inventory_discovery_contract_rejects_revoked_node_id() {
    let endpoint_listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind endpoint listener failed: {err}"));
    let endpoint_addr = endpoint_listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read endpoint listener addr failed: {err}"));
    let now = now_unix();
    let key = generate_signing_key();
    let keyring = format!(
        "k1:{}",
        base64::engine::general_purpose::STANDARD.encode(key.verifying_key().as_bytes())
    );
    let nodes = format!(
        "[{{\"node_id\":\"banme\",\"endpoint\":\"{}\",\"country_code\":\"DE\",\"country_name\":\"Germany\",\"status\":\"healthy\"}}]",
        endpoint_addr
    );
    let payload = build_signed_payload(
        &key,
        "k1",
        "n-revnode-1",
        now.saturating_sub(1),
        now.saturating_add(60),
        &nodes,
    );
    let url = serve_json_once(payload);
    let args = vec![
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--discovery-url".to_string(),
        url,
        "--discovery-keyring".to_string(),
        keyring,
        "--discovery-revoked-node-ids".to_string(),
        "banme".to_string(),
    ];
    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(inventory.nodes.is_empty());
}
