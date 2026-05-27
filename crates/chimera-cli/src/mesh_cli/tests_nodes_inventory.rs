use super::nodes_inventory::{
    build_discovery_signature_message, load_mesh_nodes_inventory, parse_inventory_config_text,
};
use super::nodes_render::render_nodes_list;
use base64::Engine as _;
use chimera_mesh::{MeshNodeCountry, MeshNodeListFilter};
use ed25519_dalek::{Signer, SigningKey};
use rand::{RngCore, rngs::OsRng};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn nodes_inventory_config_loads_groupable_nodes() {
    let text = r#"
mesh.nodes.ids = de,nl,x
mesh.nodes.current = nl
mesh.nodes.pinned = none
mesh.nodes.autoconnect = true
mesh.node.de.endpoint = ${CHIMERA_DE_ENDPOINT}
mesh.node.de.country_code = DE
mesh.node.de.country_name = Germany
mesh.node.de.status = healthy
mesh.node.de.latency_ms = 24
mesh.node.de.jitter_ms = 3
mesh.node.de.loss_pct = 0.1
mesh.node.de.success_rate_5m = 99
mesh.node.de.success_rate_1h = 99
mesh.node.de.observation_count = 10
mesh.node.nl.endpoint = ${CHIMERA_NL_ENDPOINT}
mesh.node.nl.country_code = NL
mesh.node.nl.country_name = Netherlands
mesh.node.nl.status = healthy
mesh.node.nl.latency_ms = 31
mesh.node.nl.jitter_ms = 4
mesh.node.nl.loss_pct = 0.0
mesh.node.nl.success_rate_5m = 98
mesh.node.nl.success_rate_1h = 98
mesh.node.nl.observation_count = 10
mesh.node.x.endpoint = ${CHIMERA_X_ENDPOINT}
mesh.node.x.country_code = ZZ
mesh.node.x.status = checking
"#;

    let inventory = parse_inventory_config_text(text).unwrap_or_else(|err| unreachable!("{err}"));

    assert_eq!(inventory.nodes.len(), 3);
    assert_eq!(
        inventory.current_node.as_ref().map(|node| node.0.as_str()),
        Some("nl")
    );
    assert_eq!(inventory.autoconnect_enabled, Some(true));
    assert!(
        inventory
            .nodes
            .iter()
            .any(|node| node.country.country_name == MeshNodeCountry::UNKNOWN_NAME)
    );
}

#[test]
fn nodes_inventory_rejects_unknown_config_key() {
    let text = r#"
mesh.nodes.ids = de
mesh.node.de.endpoint = ${CHIMERA_DE_ENDPOINT}
mesh.node.de.country_code = DE
mesh.node.de.country_name = Germany
mesh.node.de.bad_field = value
"#;

    let error = parse_inventory_config_text(text)
        .err()
        .unwrap_or_else(|| unreachable!("config must fail"));

    assert!(error.contains("unknown mesh node field"));
}

#[test]
fn nodes_inventory_cli_node_overrides_empty_inventory() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind test listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read listener addr failed: {err}"));
    let args = vec![
        "--node".to_string(),
        format!("de@{}@DE@Germany@healthy@24@3@0.1@99@99@0@10", addr),
    ];

    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));

    assert_eq!(inventory.nodes.len(), 1);
    assert_eq!(inventory.nodes[0].node_id.0, "de");
}

#[test]
fn nodes_inventory_render_shows_config_state() {
    let text = r#"
mesh.nodes.ids = de
mesh.nodes.current = de
mesh.nodes.autoconnect = false
mesh.node.de.endpoint = ${CHIMERA_DE_ENDPOINT}
mesh.node.de.country_code = DE
mesh.node.de.country_name = Germany
mesh.node.de.status = healthy
mesh.node.de.observation_count = 10
"#;
    let mut inventory =
        parse_inventory_config_text(text).unwrap_or_else(|err| unreachable!("{err}"));
    chimera_mesh::refresh_mesh_node_scores(
        &mut inventory.nodes,
        &chimera_mesh::MeshNodesPolicy::default(),
    );

    let rendered = render_nodes_list(&inventory, &MeshNodeListFilter::default());

    assert!(rendered.contains("Страна: Germany"));
    assert!(rendered.contains("id: de"));
}

#[test]
fn nodes_inventory_render_shows_last_activation_state() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind test listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read listener addr failed: {err}"));
    let temp_dir = std::env::temp_dir().join(format!("chimera-mesh-activation-{}", random_u64()));
    fs::create_dir_all(&temp_dir)
        .unwrap_or_else(|err| unreachable!("create temp dir failed: {err}"));
    let activation_path = temp_dir.join("activation.json");
    fs::write(
        &activation_path,
        r#"{
  "status":"active",
  "self_node_id":"de",
  "activated_at_unix":1711111111
}"#,
    )
    .unwrap_or_else(|err| unreachable!("write activation file failed: {err}"));
    let args = vec![
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--node".to_string(),
        format!("de@{}@DE@Germany@healthy@20@2@0.0@99@99@0@10", addr),
        "--activation-log".to_string(),
        activation_path.display().to_string(),
    ];
    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));
    let rendered = render_nodes_list(&inventory, &MeshNodeListFilter::default());
    assert!(rendered.contains("id: de"));
    assert!(rendered.contains("Страна: Germany"));
}

#[test]
fn nodes_inventory_filters_unreachable_nodes() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind test listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read listener addr failed: {err}"));
    let reachable = format!("ok@{}@DE@Germany@healthy@24@3@0.1@99@99@0@10", addr);
    let unreachable = "bad@127.0.0.1:1@DE@Germany@healthy@24@3@0.1@99@99@0@10".to_string();
    let args = vec![
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--node".to_string(),
        reachable,
        "--node".to_string(),
        unreachable,
    ];
    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(inventory.nodes.len(), 1);
    assert_eq!(inventory.nodes[0].node_id.0, "ok");
}

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

#[test]
fn nodes_inventory_enters_restricted_mode_when_self_is_revoked() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind test listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read listener addr failed: {err}"));
    let args = vec![
        "--self-node-id".to_string(),
        "self-1".to_string(),
        "--discovery-revoked-node-ids".to_string(),
        "self-1".to_string(),
        "--node".to_string(),
        format!("de@{}@DE@Germany@healthy@24@3@0.1@99@99@0@10", addr),
    ];
    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(
        inventory.self_node_id.as_ref().map(|id| id.0.as_str()),
        Some("self-1")
    );
    assert!(inventory.restricted_reason.is_some());
}

#[test]
fn nodes_inventory_lifts_restricted_mode_from_active_identity_state() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind test listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read listener addr failed: {err}"));
    let ts = now_unix();
    let mut state_path = std::env::temp_dir();
    state_path.push(format!("chimera_identity_state_{ts}.json"));
    fs::write(
        &state_path,
        "{\"kind\":\"mesh_identity_state\",\"status\":\"active\",\"self_node_id\":\"self-2\",\"restricted_mode\":false}",
    )
    .unwrap_or_else(|err| unreachable!("write state failed: {err}"));
    let args = vec![
        "--self-node-id".to_string(),
        "self-2".to_string(),
        "--identity-state".to_string(),
        state_path.display().to_string(),
        "--node".to_string(),
        format!("de@{}@DE@Germany@healthy@24@3@0.1@99@99@0@10", addr),
    ];
    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(inventory.restricted_reason.is_none());
    let _ = fs::remove_file(state_path);
}

#[test]
fn nodes_inventory_keeps_restricted_mode_if_revoked_even_with_active_state() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind test listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read listener addr failed: {err}"));
    let ts = now_unix();
    let mut state_path = std::env::temp_dir();
    state_path.push(format!("chimera_identity_state_revoked_{ts}.json"));
    fs::write(
        &state_path,
        "{\"kind\":\"mesh_identity_state\",\"status\":\"active\",\"self_node_id\":\"self-3\",\"restricted_mode\":false}",
    )
    .unwrap_or_else(|err| unreachable!("write state failed: {err}"));
    let args = vec![
        "--self-node-id".to_string(),
        "self-3".to_string(),
        "--identity-state".to_string(),
        state_path.display().to_string(),
        "--discovery-revoked-node-ids".to_string(),
        "self-3".to_string(),
        "--node".to_string(),
        format!("de@{}@DE@Germany@healthy@24@3@0.1@99@99@0@10", addr),
    ];
    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(inventory.restricted_reason.is_some());
    let _ = fs::remove_file(state_path);
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_else(|err| unreachable!("system time error: {err}"))
}

fn random_u64() -> u64 {
    let mut bytes = [0_u8; 8];
    OsRng.fill_bytes(&mut bytes);
    u64::from_le_bytes(bytes)
}

fn serve_json_once(body: String) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind http listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read http listener addr failed: {err}"));
    thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .unwrap_or_else(|err| unreachable!("accept failed: {err}"));
        let mut buffer = [0u8; 1024];
        let _ = stream.read(&mut buffer);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .unwrap_or_else(|err| unreachable!("write response failed: {err}"));
    });
    format!("http://{addr}/nodes")
}

fn build_signed_payload(
    signing_key: &SigningKey,
    key_id: &str,
    nonce: &str,
    issued_at_unix: u64,
    expires_at_unix: u64,
    nodes_json: &str,
) -> String {
    let nodes_value: serde_json::Value = serde_json::from_str(nodes_json)
        .unwrap_or_else(|err| unreachable!("valid nodes json: {err}"));
    let msg =
        build_discovery_signature_message(1, issued_at_unix, expires_at_unix, nonce, &nodes_value)
            .unwrap_or_else(|err| unreachable!("build message failed: {err}"));
    let signature = signing_key.sign(&msg);
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
    format!(
        "{{\"contract_version\":1,\"key_id\":\"{}\",\"issued_at_unix\":{},\"expires_at_unix\":{},\"nonce\":\"{}\",\"signature\":\"{}\",\"nodes\":{}}}",
        key_id, issued_at_unix, expires_at_unix, nonce, signature_b64, nodes_json
    )
}

fn generate_signing_key() -> SigningKey {
    let mut rng = OsRng;
    let mut secret = [0u8; 32];
    rng.fill_bytes(&mut secret);
    SigningKey::from_bytes(&secret)
}
