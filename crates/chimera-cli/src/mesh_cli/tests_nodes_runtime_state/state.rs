use super::helpers::random_u64;
use crate::mesh_cli::nodes_cmd::mesh_nodes_command;
use crate::mesh_cli::nodes_inventory::load_mesh_nodes_inventory;
use std::{fs, net::TcpListener};

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
