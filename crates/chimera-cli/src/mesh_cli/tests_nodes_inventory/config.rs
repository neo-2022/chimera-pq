use super::helpers::random_u64;
use crate::mesh_cli::nodes_inventory::{load_mesh_nodes_inventory, parse_inventory_config_text};
use crate::mesh_cli::nodes_render::render_nodes_list;
use chimera_mesh::{MeshNodeCountry, MeshNodeListFilter};
use std::{fs, net::TcpListener};

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
mesh.node.nl.update_bootstrap_url = http://node-nl.example:18179/chimera.sh
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
    assert_eq!(
        inventory
            .nodes
            .iter()
            .find(|node| node.node_id.0 == "nl")
            .and_then(|node| node.update_bootstrap_url.as_deref()),
        Some("http://node-nl.example:18179/chimera.sh")
    );
    assert!(
        inventory
            .nodes
            .iter()
            .any(|node| node.country.country_name == MeshNodeCountry::UNKNOWN_NAME)
    );
}

#[test]
fn nodes_inventory_rejects_bad_update_bootstrap_url() {
    let text = r#"
mesh.nodes.ids = de
mesh.node.de.endpoint = ${CHIMERA_DE_ENDPOINT}
mesh.node.de.country_code = DE
mesh.node.de.country_name = Germany
mesh.node.de.status = healthy
mesh.node.de.update_bootstrap_url = file:///tmp/chimera.sh
"#;

    let error = parse_inventory_config_text(text)
        .err()
        .unwrap_or_else(|| unreachable!("bad update_bootstrap_url must fail"));

    assert!(error.contains("update_bootstrap_url"));
}

#[test]
fn nodes_inventory_rejects_update_bootstrap_url_userinfo() {
    let text = "\
mesh.nodes.ids = de
mesh.node.de.endpoint = 127.0.0.1:1111
mesh.node.de.country_code = DE
mesh.node.de.country_name = Germany
mesh.node.de.status = healthy
mesh.node.de.observation_count = 10
mesh.node.de.update_bootstrap_url = http://user@node-de.example:18179/chimera.sh
";

    let error = parse_inventory_config_text(text)
        .err()
        .unwrap_or_else(|| unreachable!("userinfo update_bootstrap_url must fail"));

    assert!(error.contains("update_bootstrap_url"));
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
