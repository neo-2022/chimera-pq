use super::nodes_cmd::mesh_nodes_command;
use super::nodes_cmd::proof_pq_strict_enabled;
use super::nodes_cmd::verify_chimera_proof;
use super::nodes_cmd::verify_guard_challenge;
use super::nodes_cmd::{render_nodes_json_error, render_probe_all_json, render_state_view_json};
use super::nodes_inventory::load_mesh_nodes_inventory;
use base64::Engine;
use chimera_mesh::MeshConnectAttempt;
use chimera_mesh::MeshConnectProbeReport;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn nodes_autoconnect_persists_runtime_state_file() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read listener addr failed: {err}"));
    let mut state_path = std::env::temp_dir();
    state_path.push(format!("chimera_mesh_runtime_state_{}.json", random_u64()));
    let args = vec![
        "autoconnect".to_string(),
        "on".to_string(),
        "--runtime-state".to_string(),
        state_path.display().to_string(),
        "--node".to_string(),
        format!("de@{}@DE@Germany@healthy@24@3@0.1@99@99@0@10", addr),
    ];
    assert_eq!(mesh_nodes_command(&args), 0);
    let body = fs::read_to_string(&state_path).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(body.contains("\"kind\":\"mesh_nodes_runtime_state\""));
    assert!(body.contains("\"autoconnect\":true"));
    let _ = fs::remove_file(state_path);
}

#[test]
fn nodes_inventory_overrides_config_with_runtime_state_file() {
    let de_listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind de listener failed: {err}"));
    let nl_listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind nl listener failed: {err}"));
    let de_addr = de_listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read de addr failed: {err}"));
    let nl_addr = nl_listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read nl addr failed: {err}"));
    let mut runtime_state_path = std::env::temp_dir();
    runtime_state_path.push(format!(
        "chimera_mesh_runtime_state_load_{}.json",
        random_u64()
    ));
    fs::write(
        &runtime_state_path,
        "{\"kind\":\"mesh_nodes_runtime_state\",\"current_node_id\":\"nl\",\"pinned_node_id\":\"nl\",\"autoconnect\":true}",
    )
    .unwrap_or_else(|err| unreachable!("write runtime state failed: {err}"));
    let mut config_path = std::env::temp_dir();
    config_path.push(format!("chimera_mesh_runtime_cfg_{}.conf", random_u64()));
    let config = format!(
        "mesh.nodes.ids = de,nl\nmesh.nodes.current = de\nmesh.nodes.pinned = de\nmesh.nodes.autoconnect = false\nmesh.nodes.runtime_state_path = {}\nmesh.node.de.endpoint = {}\nmesh.node.de.country_code = DE\nmesh.node.de.country_name = Germany\nmesh.node.de.status = healthy\nmesh.node.de.observation_count = 10\nmesh.node.nl.endpoint = {}\nmesh.node.nl.country_code = NL\nmesh.node.nl.country_name = Netherlands\nmesh.node.nl.status = healthy\nmesh.node.nl.observation_count = 10\n",
        runtime_state_path.display(),
        de_addr,
        nl_addr
    );
    fs::write(&config_path, config)
        .unwrap_or_else(|err| unreachable!("write config failed: {err}"));
    let args = vec![
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--config".to_string(),
        config_path.display().to_string(),
    ];
    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(
        inventory.current_node.as_ref().map(|id| id.0.as_str()),
        Some("nl")
    );
    assert_eq!(
        inventory.pinned_node.as_ref().map(|id| id.0.as_str()),
        Some("nl")
    );
    assert_eq!(inventory.autoconnect_enabled, Some(true));
    let _ = fs::remove_file(runtime_state_path);
    let _ = fs::remove_file(config_path);
}

#[test]
fn nodes_probe_all_uses_connect_probe_backend() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read listener addr failed: {err}"));
    let args = vec![
        "probe".to_string(),
        "--all".to_string(),
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--node".to_string(),
        format!("de@{}@DE@Germany@healthy@24@3@0.1@99@99@0@10", addr),
    ];
    assert_eq!(mesh_nodes_command(&args), 0);
}

#[test]
fn nodes_state_clear_removes_runtime_state_file() {
    let mut state_path = std::env::temp_dir();
    state_path.push(format!(
        "chimera_mesh_runtime_state_clear_{}.json",
        random_u64()
    ));
    fs::write(
        &state_path,
        "{\"kind\":\"mesh_nodes_runtime_state\",\"current_node_id\":\"de\",\"pinned_node_id\":\"de\",\"autoconnect\":true}",
    )
    .unwrap_or_else(|err| unreachable!("write runtime state failed: {err}"));
    let args = vec![
        "state".to_string(),
        "clear".to_string(),
        "--runtime-state".to_string(),
        state_path.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&args), 0);
    assert!(!state_path.exists());
}

#[test]
fn nodes_probe_all_json_contract_has_common_fields() {
    let report = MeshConnectProbeReport {
        namespace: "ns-a".to_string(),
        selected_peers: vec!["de".to_string()],
        connected_peer: "de".to_string(),
        connected_endpoint: "127.0.0.1:443".to_string(),
        success: true,
        attempts: vec![MeshConnectAttempt {
            peer_id: "de".to_string(),
            endpoint: "127.0.0.1:443".to_string(),
            success: true,
            error: String::new(),
        }],
        explain: Vec::new(),
    };
    let json = render_probe_all_json(&report);
    assert!(json.contains("\"kind\":\"mesh_nodes_probe_all\""));
    assert!(json.contains("\"status\":\"ok\""));
    assert!(json.contains("\"contract_version\":1"));
    assert!(json.contains("\"network_state\":\"not_modified\""));
}

#[test]
fn nodes_state_view_json_contract_has_common_fields() {
    let inventory = load_mesh_nodes_inventory(&[]).unwrap_or_else(|err| unreachable!("{err}"));
    let json = render_state_view_json(&inventory);
    assert!(json.contains("\"kind\":\"mesh_nodes_runtime_state_view\""));
    assert!(json.contains("\"status\":\"ok\""));
    assert!(json.contains("\"contract_version\":1"));
    assert!(json.contains("\"network_state\":\"not_modified\""));
}

#[test]
fn nodes_json_error_contract_has_common_fields() {
    let json = render_nodes_json_error(
        "mesh_nodes_probe_all",
        "probe_input",
        "inspect_inventory",
        "no nodes available for probe",
    );
    assert!(json.contains("\"kind\":\"mesh_nodes_probe_all\""));
    assert!(json.contains("\"status\":\"error\""));
    assert!(json.contains("\"contract_family\":\"mesh_nodes_contract\""));
    assert!(json.contains("\"contract_version\":1"));
    assert!(json.contains("\"network_state\":\"not_modified\""));
    assert!(json.contains("\"stage\":\"probe_input\""));
    assert!(json.contains("\"action\":\"inspect_inventory\""));
    assert!(json.contains("\"error_signature\":\"probe_input:inspect_inventory\""));
    assert!(json.contains("\"error_route_key\":\"mesh_nodes_probe_all:inspect_inventory\""));
}

#[test]
fn nodes_probe_all_json_snapshot_stable() {
    let report = MeshConnectProbeReport {
        namespace: "ns-snap".to_string(),
        selected_peers: vec!["de".to_string()],
        connected_peer: "de".to_string(),
        connected_endpoint: "127.0.0.1:443".to_string(),
        success: true,
        attempts: vec![MeshConnectAttempt {
            peer_id: "de".to_string(),
            endpoint: "127.0.0.1:443".to_string(),
            success: true,
            error: String::new(),
        }],
        explain: Vec::new(),
    };
    let json = render_probe_all_json(&report);
    let expected = "{\"kind\":\"mesh_nodes_probe_all\",\"status\":\"ok\",\"contract_version\":1,\"network_state\":\"not_modified\",\"success\":true,\"selected\":1,\"attempts_count\":1,\"connected_peer\":\"de\",\"connected_endpoint\":\"127.0.0.1:443\",\"attempts\":[{\"peer_id\":\"de\",\"endpoint\":\"127.0.0.1:443\",\"success\":true,\"error\":\"\"}]}";
    assert_eq!(json, expected);
}

#[test]
fn nodes_state_view_json_snapshot_stable() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read listener addr failed: {err}"));
    let mut config_path = std::env::temp_dir();
    config_path.push(format!("chimera_mesh_state_view_cfg_{}.conf", random_u64()));
    let config = format!(
        "mesh.nodes.ids = de\nmesh.nodes.current = de\nmesh.nodes.autoconnect = true\nmesh.node.de.endpoint = {}\nmesh.node.de.country_code = DE\nmesh.node.de.country_name = Germany\nmesh.node.de.status = healthy\nmesh.node.de.observation_count = 10\n",
        addr
    );
    fs::write(&config_path, config)
        .unwrap_or_else(|err| unreachable!("write config failed: {err}"));
    let args = vec!["--config".to_string(), config_path.display().to_string()];
    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));
    let json = render_state_view_json(&inventory);
    let expected = "{\"kind\":\"mesh_nodes_runtime_state_view\",\"status\":\"ok\",\"contract_version\":1,\"network_state\":\"not_modified\",\"current_node_id\":\"de\",\"pinned_node_id\":\"\",\"autoconnect\":true,\"restricted_mode\":false,\"restricted_reason\":\"\"}";
    assert_eq!(json, expected);
    let _ = fs::remove_file(config_path);
}

#[test]
fn nodes_advertise_writes_signed_discovery_snapshot() {
    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "chimera_mesh_discovery_snapshot_{}.json",
        random_u64()
    ));
    let mut pubkey_path = std::env::temp_dir();
    pubkey_path.push(format!(
        "chimera_mesh_discovery_snapshot_{}.pub",
        random_u64()
    ));
    let mut keypair_path = std::env::temp_dir();
    keypair_path.push(format!(
        "chimera_mesh_discovery_snapshot_{}.keypair",
        random_u64()
    ));
    let endpoint = "198.51.100.77:54321";
    let args = vec![
        "advertise".to_string(),
        "--node-id".to_string(),
        "node-eu-1".to_string(),
        "--endpoint".to_string(),
        endpoint.to_string(),
        "--out".to_string(),
        out_path.display().to_string(),
        "--pubkey-out".to_string(),
        pubkey_path.display().to_string(),
        "--keypair-path".to_string(),
        keypair_path.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&args), 0);
    let body = fs::read_to_string(&out_path).unwrap_or_else(|err| unreachable!("{err}"));
    let pubkey = fs::read_to_string(&pubkey_path).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(body.contains("\"node_id\":\"node-eu-1\""));
    assert!(body.contains(endpoint));
    assert!(body.contains("\"contract_version\":1"));
    assert!(!pubkey.trim().is_empty());
    let _ = fs::remove_file(out_path);
    let _ = fs::remove_file(pubkey_path);
    let _ = fs::remove_file(keypair_path);
}

#[test]
fn nodes_json_error_snapshot_stable() {
    let json = render_nodes_json_error(
        "mesh_nodes_probe_all",
        "probe_input",
        "inspect_inventory",
        "no nodes available for probe",
    );
    let expected = "{\"kind\":\"mesh_nodes_probe_all\",\"status\":\"error\",\"contract_family\":\"mesh_nodes_contract\",\"contract_version\":1,\"network_state\":\"not_modified\",\"stage\":\"probe_input\",\"action\":\"inspect_inventory\",\"message\":\"no nodes available for probe\",\"error_signature\":\"probe_input:inspect_inventory\",\"error_route_key\":\"mesh_nodes_probe_all:inspect_inventory\"}";
    assert_eq!(json, expected);
}

#[test]
fn nodes_verify_chimera_proof_accepts_valid_guard_response() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read addr failed: {err}"));
    let token = "tok-proof-a".to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .unwrap_or_else(|err| unreachable!("accept failed: {err}"));
        let mut line = String::new();
        let mut reader = BufReader::new(
            stream
                .try_clone()
                .unwrap_or_else(|err| unreachable!("clone failed: {err}")),
        );
        reader
            .read_line(&mut line)
            .unwrap_or_else(|err| unreachable!("read_line failed: {err}"));
        let parsed: serde_json::Value =
            serde_json::from_str(line.trim()).unwrap_or_else(|err| unreachable!("{err}"));
        assert_eq!(parsed["kind"], "mesh_guard_challenge_v1");
        assert_eq!(parsed["key_id"], "mesh-shared-v1");
        assert_eq!(parsed["pq_key_id"], "mesh-pq-shared-v1");
        assert_eq!(parsed["classic_alg"], "hmac-sha256-v1");
        assert_eq!(parsed["pq_alg"], "hmac-sha256-v1-placeholder");
        assert!(parsed["classic_sig"].as_str().unwrap_or("").len() > 10);
        assert!(parsed["pq_sig"].as_str().unwrap_or("").len() > 10);
        stream
            .write_all(b"{\"kind\":\"mesh_guard_ack_v1\",\"status\":\"ok\"}\n")
            .unwrap_or_else(|err| unreachable!("write failed: {err}"));
    });
    let endpoint = format!("{addr}");
    let result = verify_chimera_proof(
        &endpoint,
        &token,
        &token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
        500,
    );
    handle
        .join()
        .unwrap_or_else(|_| unreachable!("join failed"));
    assert!(result.is_ok());
}

#[test]
fn nodes_verify_chimera_proof_sends_custom_key_ids() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read addr failed: {err}"));
    let token = "tok-proof-custom".to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .unwrap_or_else(|err| unreachable!("accept failed: {err}"));
        let mut line = String::new();
        let mut reader = BufReader::new(
            stream
                .try_clone()
                .unwrap_or_else(|err| unreachable!("clone failed: {err}")),
        );
        reader
            .read_line(&mut line)
            .unwrap_or_else(|err| unreachable!("read_line failed: {err}"));
        let parsed: serde_json::Value =
            serde_json::from_str(line.trim()).unwrap_or_else(|err| unreachable!("{err}"));
        assert_eq!(parsed["key_id"], "mesh-shared-v9");
        assert_eq!(parsed["pq_key_id"], "mesh-pq-shared-v9");
        stream
            .write_all(b"{\"kind\":\"mesh_guard_ack_v1\",\"status\":\"ok\"}\n")
            .unwrap_or_else(|err| unreachable!("write failed: {err}"));
    });
    let endpoint = format!("{addr}");
    let result = verify_chimera_proof(
        &endpoint,
        &token,
        &token,
        "mesh-shared-v9",
        "mesh-pq-shared-v9",
        500,
    );
    handle
        .join()
        .unwrap_or_else(|_| unreachable!("join failed"));
    assert!(result.is_ok());
}

#[test]
fn nodes_verify_chimera_proof_rejects_invalid_guard_response() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read addr failed: {err}"));
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .unwrap_or_else(|err| unreachable!("accept failed: {err}"));
        let mut line = String::new();
        let mut reader = BufReader::new(
            stream
                .try_clone()
                .unwrap_or_else(|err| unreachable!("clone failed: {err}")),
        );
        reader
            .read_line(&mut line)
            .unwrap_or_else(|err| unreachable!("read_line failed: {err}"));
        stream
            .write_all(b"NOPE\n")
            .unwrap_or_else(|err| unreachable!("write failed: {err}"));
    });
    let endpoint = format!("{addr}");
    let result = verify_chimera_proof(
        &endpoint,
        "tok-proof-b",
        "tok-proof-b",
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
        500,
    );
    handle
        .join()
        .unwrap_or_else(|_| unreachable!("join failed"));
    assert!(result.is_err());
}

#[test]
fn nodes_verify_guard_challenge_rejects_unexpected_key_id() {
    let token = "tok-proof-c";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let expires = now.saturating_add(30);
    let nonce = format!("n-{}-test", random_u64());
    let message = format!(
        "nonce={}\nissued_at_unix={}\nexpires_at_unix={}\n",
        nonce, now, expires
    );
    let classic_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let classic_payload = format!("chimera-classic-v1\n{message}");
    let classic_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&classic_key, classic_payload.as_bytes()).as_ref());
    let pq_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let pq_payload = format!("chimera-pq-v1\n{message}");
    let pq_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&pq_key, pq_payload.as_bytes()).as_ref());
    let challenge = serde_json::json!({
        "kind":"mesh_guard_challenge_v1",
        "key_id":"mesh-shared-v2",
        "pq_key_id":"mesh-pq-shared-v1",
        "classic_alg":"hmac-sha256-v1",
        "pq_alg":"hmac-sha256-v1-placeholder",
        "nonce":nonce,
        "issued_at_unix":now,
        "expires_at_unix":expires,
        "classic_sig":classic_sig,
        "pq_sig":pq_sig
    });
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert_eq!(result.unwrap_err(), "unexpected_key_id");
}

#[test]
fn nodes_verify_guard_challenge_rejects_unexpected_pq_key_id() {
    let token = "tok-proof-d";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let expires = now.saturating_add(30);
    let nonce = format!("n-{}-test", random_u64());
    let message = format!(
        "nonce={}\nissued_at_unix={}\nexpires_at_unix={}\n",
        nonce, now, expires
    );
    let classic_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let classic_payload = format!("chimera-classic-v1\n{message}");
    let classic_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&classic_key, classic_payload.as_bytes()).as_ref());
    let pq_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let pq_payload = format!("chimera-pq-v1\n{message}");
    let pq_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&pq_key, pq_payload.as_bytes()).as_ref());
    let challenge = serde_json::json!({
        "kind":"mesh_guard_challenge_v1",
        "key_id":"mesh-shared-v1",
        "pq_key_id":"mesh-pq-shared-v2",
        "classic_alg":"hmac-sha256-v1",
        "pq_alg":"hmac-sha256-v1-placeholder",
        "nonce":nonce,
        "issued_at_unix":now,
        "expires_at_unix":expires,
        "classic_sig":classic_sig,
        "pq_sig":pq_sig
    });
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert_eq!(result.unwrap_err(), "unexpected_pq_key_id");
}

#[test]
fn nodes_verify_guard_challenge_rejects_missing_key_id() {
    let token = "tok-proof-e";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let expires = now.saturating_add(30);
    let nonce = format!("n-{}-test", random_u64());
    let message = format!(
        "nonce={}\nissued_at_unix={}\nexpires_at_unix={}\n",
        nonce, now, expires
    );
    let classic_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let classic_payload = format!("chimera-classic-v1\n{message}");
    let classic_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&classic_key, classic_payload.as_bytes()).as_ref());
    let pq_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let pq_payload = format!("chimera-pq-v1\n{message}");
    let pq_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&pq_key, pq_payload.as_bytes()).as_ref());
    let challenge = serde_json::json!({
        "kind":"mesh_guard_challenge_v1",
        "pq_key_id":"mesh-pq-shared-v1",
        "classic_alg":"hmac-sha256-v1",
        "pq_alg":"hmac-sha256-v1-placeholder",
        "nonce":nonce,
        "issued_at_unix":now,
        "expires_at_unix":expires,
        "classic_sig":classic_sig,
        "pq_sig":pq_sig
    });
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert_eq!(result.unwrap_err(), "missing_key_id");
}

#[test]
fn nodes_verify_guard_challenge_rejects_missing_pq_key_id() {
    let token = "tok-proof-f";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let expires = now.saturating_add(30);
    let nonce = format!("n-{}-test", random_u64());
    let message = format!(
        "nonce={}\nissued_at_unix={}\nexpires_at_unix={}\n",
        nonce, now, expires
    );
    let classic_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let classic_payload = format!("chimera-classic-v1\n{message}");
    let classic_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&classic_key, classic_payload.as_bytes()).as_ref());
    let pq_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let pq_payload = format!("chimera-pq-v1\n{message}");
    let pq_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&pq_key, pq_payload.as_bytes()).as_ref());
    let challenge = serde_json::json!({
        "kind":"mesh_guard_challenge_v1",
        "key_id":"mesh-shared-v1",
        "classic_alg":"hmac-sha256-v1",
        "pq_alg":"hmac-sha256-v1-placeholder",
        "nonce":nonce,
        "issued_at_unix":now,
        "expires_at_unix":expires,
        "classic_sig":classic_sig,
        "pq_sig":pq_sig
    });
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert_eq!(result.unwrap_err(), "missing_pq_key_id");
}

#[test]
fn nodes_verify_guard_challenge_rejects_invalid_ttl_window() {
    let token = "tok-proof-g";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let nonce = format!("n-{}-test", random_u64());
    let challenge = build_guard_challenge(token, &nonce, now, now);
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert_eq!(result.unwrap_err(), "invalid_ttl_window");
}

#[test]
fn nodes_verify_guard_challenge_rejects_issued_at_too_far_in_future() {
    let token = "tok-proof-h";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let nonce = format!("n-{}-test", random_u64());
    let issued_at = now.saturating_add(3600);
    let challenge = build_guard_challenge(token, &nonce, issued_at, issued_at.saturating_add(30));
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert_eq!(result.unwrap_err(), "issued_at_too_far_in_future");
}

#[test]
fn nodes_verify_guard_challenge_rejects_expired_challenge() {
    let token = "tok-proof-i";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let nonce = format!("n-{}-test", random_u64());
    let issued_at = now.saturating_sub(120);
    let challenge = build_guard_challenge(token, &nonce, issued_at, now.saturating_sub(1));
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert_eq!(result.unwrap_err(), "challenge_expired");
}

#[test]
fn nodes_verify_guard_challenge_rejects_replay_nonce() {
    let token = "tok-proof-j";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let nonce = format!("n-{}-test", random_u64());
    let challenge = build_guard_challenge(token, &nonce, now, now.saturating_add(30));
    let first = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert!(first.is_ok());
    let second = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert_eq!(second.unwrap_err(), "guard_replay_nonce");
}

#[test]
fn nodes_json_error_snapshot_proof_verify_stage_stable() {
    let json = render_nodes_json_error(
        "mesh_nodes_probe_all",
        "proof_verify",
        "verify_chimera_proof",
        "connect_error:connection refused",
    );
    let expected = "{\"kind\":\"mesh_nodes_probe_all\",\"status\":\"error\",\"contract_family\":\"mesh_nodes_contract\",\"contract_version\":1,\"network_state\":\"not_modified\",\"stage\":\"proof_verify\",\"action\":\"verify_chimera_proof\",\"message\":\"connect_error:connection refused\",\"error_signature\":\"proof_verify:verify_chimera_proof\",\"error_route_key\":\"mesh_nodes_probe_all:verify_chimera_proof\"}";
    assert_eq!(json, expected);
}

#[test]
fn nodes_json_error_snapshot_state_path_stage_stable() {
    let json = render_nodes_json_error(
        "mesh_nodes_runtime_state",
        "state_path",
        "resolve_runtime_state_path",
        "runtime-state path is not configured",
    );
    let expected = "{\"kind\":\"mesh_nodes_runtime_state\",\"status\":\"error\",\"contract_family\":\"mesh_nodes_contract\",\"contract_version\":1,\"network_state\":\"not_modified\",\"stage\":\"state_path\",\"action\":\"resolve_runtime_state_path\",\"message\":\"runtime-state path is not configured\",\"error_signature\":\"state_path:resolve_runtime_state_path\",\"error_route_key\":\"mesh_nodes_runtime_state:resolve_runtime_state_path\"}";
    assert_eq!(json, expected);
}

#[test]
fn nodes_json_error_snapshot_state_clear_stage_stable() {
    let json = render_nodes_json_error(
        "mesh_nodes_runtime_state",
        "state_clear",
        "remove_runtime_state_file",
        "is a directory",
    );
    let expected = "{\"kind\":\"mesh_nodes_runtime_state\",\"status\":\"error\",\"contract_family\":\"mesh_nodes_contract\",\"contract_version\":1,\"network_state\":\"not_modified\",\"stage\":\"state_clear\",\"action\":\"remove_runtime_state_file\",\"message\":\"is a directory\",\"error_signature\":\"state_clear:remove_runtime_state_file\",\"error_route_key\":\"mesh_nodes_runtime_state:remove_runtime_state_file\"}";
    assert_eq!(json, expected);
}

#[test]
fn nodes_json_error_snapshot_state_options_parse_stage_stable() {
    let json = render_nodes_json_error(
        "mesh_nodes_runtime_state",
        "options_parse",
        "parse_state_subcommand",
        "unknown subcommand 'bad'",
    );
    let expected = "{\"kind\":\"mesh_nodes_runtime_state\",\"status\":\"error\",\"contract_family\":\"mesh_nodes_contract\",\"contract_version\":1,\"network_state\":\"not_modified\",\"stage\":\"options_parse\",\"action\":\"parse_state_subcommand\",\"message\":\"unknown subcommand 'bad'\",\"error_signature\":\"options_parse:parse_state_subcommand\",\"error_route_key\":\"mesh_nodes_runtime_state:parse_state_subcommand\"}";
    assert_eq!(json, expected);
}

#[test]
fn nodes_proof_pq_strict_defaults_to_enabled() {
    let args: Vec<String> = Vec::new();
    assert!(proof_pq_strict_enabled(&args));
}

#[test]
fn nodes_proof_pq_strict_can_be_disabled_by_flag() {
    let args = vec!["--no-pq-strict".to_string()];
    assert!(!proof_pq_strict_enabled(&args));
}

fn random_u64() -> u64 {
    use rand::RngCore;
    let mut bytes = [0_u8; 8];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    u64::from_le_bytes(bytes)
}

fn build_guard_challenge(
    token: &str,
    nonce: &str,
    issued_at_unix: u64,
    expires_at_unix: u64,
) -> serde_json::Value {
    let message = format!(
        "nonce={}\nissued_at_unix={}\nexpires_at_unix={}\n",
        nonce, issued_at_unix, expires_at_unix
    );
    let classic_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let classic_payload = format!("chimera-classic-v1\n{message}");
    let classic_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&classic_key, classic_payload.as_bytes()).as_ref());
    let pq_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let pq_payload = format!("chimera-pq-v1\n{message}");
    let pq_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&pq_key, pq_payload.as_bytes()).as_ref());
    serde_json::json!({
        "kind":"mesh_guard_challenge_v1",
        "key_id":"mesh-shared-v1",
        "pq_key_id":"mesh-pq-shared-v1",
        "classic_alg":"hmac-sha256-v1",
        "pq_alg":"hmac-sha256-v1-placeholder",
        "nonce":nonce,
        "issued_at_unix":issued_at_unix,
        "expires_at_unix":expires_at_unix,
        "classic_sig":classic_sig,
        "pq_sig":pq_sig
    })
}
