use super::helpers::now_unix;
use crate::mesh_cli::nodes_inventory::load_mesh_nodes_inventory;
use std::{fs, net::TcpListener};

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
