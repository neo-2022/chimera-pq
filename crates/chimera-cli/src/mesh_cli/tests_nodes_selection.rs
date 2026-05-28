use super::nodes_cmd::mesh_nodes_command;
use super::nodes_cmd::selected_node_endpoint;
use super::nodes_inventory::load_mesh_nodes_inventory;
use super::nodes_selection::{build_selection_entries, render_selection_prompt};
use std::fs;
use std::net::TcpListener;

#[test]
fn nodes_select_pins_and_connects_selected_node() {
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
    let mut state_path = std::env::temp_dir();
    state_path.push(format!("chimera_mesh_select_state_{}.json", random_u64()));
    let args = vec![
        "select".to_string(),
        "--runtime-state".to_string(),
        state_path.display().to_string(),
        "--id".to_string(),
        "nl".to_string(),
        "--node".to_string(),
        format!("de@{}@DE@Germany@healthy@18@2@0.0@99@99@0@10", de_addr),
        "--node".to_string(),
        format!("nl@{}@NL@Netherlands@healthy@12@1@0.0@99@99@0@10", nl_addr),
    ];
    assert_eq!(mesh_nodes_command(&args), 0);
    let body = fs::read_to_string(&state_path).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(body.contains("\"current_node_id\":\"nl\""));
    assert!(body.contains("\"pinned_node_id\":\"nl\""));
    assert!(body.contains("\"autoconnect\":true"));
    let _ = fs::remove_file(state_path);
}

#[test]
fn nodes_select_uses_positional_index_after_filters() {
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
    let mut config_path = std::env::temp_dir();
    config_path.push(format!("chimera_mesh_select_cfg_{}.conf", random_u64()));
    let mut state_path = std::env::temp_dir();
    state_path.push(format!(
        "chimera_mesh_select_state_idx_{}.json",
        random_u64()
    ));
    let config = format!(
        "mesh.nodes.ids = de,nl\nmesh.nodes.runtime_state_path = {}\nmesh.node.de.endpoint = {}\nmesh.node.de.country_code = DE\nmesh.node.de.country_name = Germany\nmesh.node.de.status = healthy\nmesh.node.de.observation_count = 10\nmesh.node.nl.endpoint = {}\nmesh.node.nl.country_code = NL\nmesh.node.nl.country_name = Netherlands\nmesh.node.nl.status = healthy\nmesh.node.nl.observation_count = 10\n",
        state_path.display(),
        de_addr,
        nl_addr
    );
    fs::write(&config_path, config)
        .unwrap_or_else(|err| unreachable!("write config failed: {err}"));
    let args = vec![
        "select".to_string(),
        "--config".to_string(),
        config_path.display().to_string(),
        "2".to_string(),
    ];
    assert_eq!(mesh_nodes_command(&args), 0);
    let body = fs::read_to_string(&state_path).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(body.contains("\"current_node_id\":\"nl\""));
    assert!(body.contains("\"pinned_node_id\":\"nl\""));
    let _ = fs::remove_file(config_path);
    let _ = fs::remove_file(state_path);
}

#[test]
fn nodes_selection_prompt_marks_best_node() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read addr failed: {err}"));
    let args = vec![
        "--node".to_string(),
        format!("de@{}@DE@Germany@healthy@18@2@0.0@99@99@0@10", addr),
    ];
    let inventory = load_mesh_nodes_inventory(&args).unwrap_or_else(|err| unreachable!("{err}"));
    let entries = build_selection_entries(&inventory, &chimera_mesh::MeshNodeListFilter::default());
    let prompt = render_selection_prompt(&entries);
    assert!(prompt.contains("Доступные узлы CHIMERA"));
    assert!(prompt.contains("Enter для лучшего узла"));
    assert!(entries.iter().any(|entry| entry.is_best));
}

#[test]
fn selected_endpoint_prefers_current_then_pinned() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read addr failed: {err}"));
    let endpoint = addr.to_string();
    let text = format!(
        "mesh.nodes.ids = de,nl\nmesh.nodes.current = nl\nmesh.nodes.pinned = de\nmesh.nodes.autoconnect = true\nmesh.node.de.endpoint = 127.0.0.1:1111\nmesh.node.de.country_code = DE\nmesh.node.de.country_name = Germany\nmesh.node.de.status = healthy\nmesh.node.de.observation_count = 10\nmesh.node.nl.endpoint = {}\nmesh.node.nl.country_code = NL\nmesh.node.nl.country_name = Netherlands\nmesh.node.nl.status = healthy\nmesh.node.nl.observation_count = 10\n",
        endpoint
    );
    let inventory = super::nodes_inventory::parse_inventory_config_text(&text)
        .unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(selected_node_endpoint(&inventory), Some(endpoint.as_str()));
}

fn random_u64() -> u64 {
    use rand::RngCore;
    let mut bytes = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut bytes);
    u64::from_le_bytes(bytes)
}
