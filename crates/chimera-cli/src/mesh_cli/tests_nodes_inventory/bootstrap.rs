use crate::mesh_cli::nodes_inventory::{
    bootstrap_env_value_from_text, discovery_urls_from_text, load_mesh_bootstrap_nodes_from_text,
};

#[test]
fn nodes_inventory_mesh_bootstrap_text_supplies_direct_endpoint_and_ignores_legacy_keys() {
    let endpoint = "198.51.100.40:443";
    let legacy_endpoint = "203.0.113.10:8443";
    let nodes = load_mesh_bootstrap_nodes_from_text(&format!(
        "CHIMERA_MESH_REMOTE_ENDPOINT={endpoint}\nCHIMERA_UPSTREAM_ENDPOINTS_CSV={legacy_endpoint}\n"
    ))
    .unwrap_or_else(|err| unreachable!("{err}"));

    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].node_id.0, "bootstrap-1");
    assert_eq!(nodes[0].endpoint, endpoint);
    assert_eq!(nodes[0].explain_reason, "mesh_bootstrap");
}

#[test]
fn mesh_bootstrap_text_exposes_discovery_contract_values() {
    let url = "https://bootstrap.example/discovery.json";
    let pubkey = "BASE64PUBKEY";
    let text = format!(
        "CHIMERA_MESH_NODES_DISCOVERY_URL={url}\nCHIMERA_MESH_NODES_DISCOVERY_PUBKEY={pubkey}\nCHIMERA_UPSTREAM_ENDPOINTS_CSV=203.0.113.55:443\n"
    );

    assert_eq!(
        bootstrap_env_value_from_text(&text, "CHIMERA_MESH_NODES_DISCOVERY_URL").as_deref(),
        Some(url)
    );
    assert_eq!(
        bootstrap_env_value_from_text(&text, "CHIMERA_MESH_NODES_DISCOVERY_PUBKEY").as_deref(),
        Some(pubkey)
    );
}

#[test]
fn mesh_bootstrap_text_exposes_discovery_source_list_contract_values() {
    let text = "CHIMERA_MESH_NODES_DISCOVERY_URL=https://seed-1.example/mesh_nodes.discovery.json\nCHIMERA_MESH_NODES_DISCOVERY_URLS=https://seed-2.example/mesh_nodes.discovery.json,https://seed-3.example/mesh_nodes.discovery.json\n";

    assert_eq!(
        discovery_urls_from_text(text),
        vec![
            "https://seed-1.example/mesh_nodes.discovery.json".to_string(),
            "https://seed-2.example/mesh_nodes.discovery.json".to_string(),
            "https://seed-3.example/mesh_nodes.discovery.json".to_string(),
        ]
    );
}
