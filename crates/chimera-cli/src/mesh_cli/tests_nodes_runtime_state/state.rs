use super::helpers::random_u64;
use crate::mesh_cli::nodes_cmd::mesh_nodes_command;
use crate::mesh_cli::nodes_inventory::{
    load_mesh_nodes_inventory, published_endpoint_updates_from_nodes,
};
use chimera_mesh::{MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshRuntime};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    thread,
};

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

#[test]
fn nodes_advertise_ignores_unreachable_discovery_with_skip_discovery_flag() {
    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "chimera_mesh_discovery_skip_{}.json",
        random_u64()
    ));
    let mut pubkey_path = std::env::temp_dir();
    pubkey_path.push(format!(
        "chimera_mesh_discovery_skip_{}.pub",
        random_u64()
    ));
    let mut keypair_path = std::env::temp_dir();
    keypair_path.push(format!(
        "chimera_mesh_discovery_skip_{}.keypair",
        random_u64()
    ));
    let endpoint = "198.51.100.99:12345";
    let args = vec![
        "advertise".to_string(),
        "--node-id".to_string(),
        "node-skip-1".to_string(),
        "--endpoint".to_string(),
        endpoint.to_string(),
        "--discovery-url".to_string(),
        "http://127.0.0.1:1/mesh_nodes.discovery.json".to_string(),
        "--skip-discovery".to_string(),
        "--out".to_string(),
        out_path.display().to_string(),
        "--pubkey-out".to_string(),
        pubkey_path.display().to_string(),
        "--keypair-path".to_string(),
        keypair_path.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&args), 0);
    let body = fs::read_to_string(&out_path).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(body.contains("\"node_id\":\"node-skip-1\""));
    assert!(body.contains(endpoint));
    let _ = fs::remove_file(out_path);
    let _ = fs::remove_file(pubkey_path);
    let _ = fs::remove_file(keypair_path);
}

#[test]
fn nodes_advertise_writes_invite_token_for_matching_node() {
    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "chimera_mesh_discovery_invite_token_{}.json",
        random_u64()
    ));
    let mut pubkey_path = std::env::temp_dir();
    pubkey_path.push(format!(
        "chimera_mesh_discovery_invite_token_{}.pub",
        random_u64()
    ));
    let mut keypair_path = std::env::temp_dir();
    keypair_path.push(format!(
        "chimera_mesh_discovery_invite_token_{}.keypair",
        random_u64()
    ));
    let mut config_path = std::env::temp_dir();
    config_path.push(format!(
        "chimera_mesh_discovery_invite_token_{}.conf",
        random_u64()
    ));
    let mut update_state_path = std::env::temp_dir();
    update_state_path.push(format!(
        "chimera_mesh_discovery_invite_token_{}.update.json",
        random_u64()
    ));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("system clock failed: {err}"))
        .as_secs();
    fs::write(
        &config_path,
        concat!(
            "mesh.nodes.ids = node-eu-invite\n",
            "mesh.nodes.self_node_id = node-eu-invite\n",
            "mesh.node.node-eu-invite.endpoint = 198.51.100.79:54321\n",
            "mesh.node.node-eu-invite.invite_token = invite-token-123\n",
            "mesh.node.node-eu-invite.status = healthy\n",
            "mesh.node.node-eu-invite.observation_count = 1\n",
        ),
    )
    .unwrap_or_else(|err| unreachable!("write config failed: {err}"));
    fs::write(
        &update_state_path,
        format!(
            "{{\"kind\":\"chimera_peer_update_serve_state\",\"status\":\"ready\",\"listen\":\"127.0.0.1:45680\",\"base_url\":\"http://node.example:45680\",\"update_bootstrap_url\":\"http://node.example:45680/chimera.sh\",\"version\":\"1.2.3\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"endpoint_epoch\":{now},\"endpoint_generation\":8}}"
        ),
    )
    .unwrap_or_else(|err| unreachable!("write update state failed: {err}"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&update_state_path, fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|err| unreachable!("chmod update state failed: {err}"));
    }
    let args = vec![
        "advertise".to_string(),
        "--config".to_string(),
        config_path.display().to_string(),
        "--node-id".to_string(),
        "node-eu-invite".to_string(),
        "--endpoint".to_string(),
        "198.51.100.79:54321".to_string(),
        "--update-state-file".to_string(),
        update_state_path.display().to_string(),
        "--out".to_string(),
        out_path.display().to_string(),
        "--pubkey-out".to_string(),
        pubkey_path.display().to_string(),
        "--keypair-path".to_string(),
        keypair_path.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&args), 0);
    let body = fs::read_to_string(&out_path).unwrap_or_else(|err| unreachable!("{err}"));
    let envelope: serde_json::Value =
        serde_json::from_str(&body).unwrap_or_else(|err| unreachable!("{err}"));
    let nodes = envelope
        .get("nodes")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| unreachable!("nodes array missing"));
    assert_eq!(
        nodes.first()
            .and_then(|node| node.get("invite_token"))
            .and_then(serde_json::Value::as_str),
        Some("invite-token-123")
    );

    let pubkey = fs::read_to_string(&pubkey_path).unwrap_or_else(|err| unreachable!("{err}"));
    let discovery_url = serve_json_once(body);
    let discovery_args = vec![
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--discovery-url".to_string(),
        discovery_url,
        "--discovery-pubkey".to_string(),
        pubkey,
    ];
    let inventory =
        load_mesh_nodes_inventory(&discovery_args).unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(
        inventory.nodes[0].invite_token.as_deref(),
        Some("invite-token-123")
    );

    let _ = fs::remove_file(out_path);
    let _ = fs::remove_file(pubkey_path);
    let _ = fs::remove_file(keypair_path);
    let _ = fs::remove_file(config_path);
    let _ = fs::remove_file(update_state_path);
}

#[test]
fn nodes_advertise_writes_update_bootstrap_url_from_flag() {
    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "chimera_mesh_discovery_update_url_flag_{}.json",
        random_u64()
    ));
    let mut pubkey_path = std::env::temp_dir();
    pubkey_path.push(format!(
        "chimera_mesh_discovery_update_url_flag_{}.pub",
        random_u64()
    ));
    let mut keypair_path = std::env::temp_dir();
    keypair_path.push(format!(
        "chimera_mesh_discovery_update_url_flag_{}.keypair",
        random_u64()
    ));
    let update_url = "http://node.example:45678/chimera.sh";
    let args = vec![
        "advertise".to_string(),
        "--node-id".to_string(),
        "node-eu-flag".to_string(),
        "--endpoint".to_string(),
        "198.51.100.77:54321".to_string(),
        "--update-bootstrap-url".to_string(),
        update_url.to_string(),
        "--out".to_string(),
        out_path.display().to_string(),
        "--pubkey-out".to_string(),
        pubkey_path.display().to_string(),
        "--keypair-path".to_string(),
        keypair_path.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&args), 0);
    let body = fs::read_to_string(&out_path).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(body.contains("\"update_bootstrap_url\":\"http://node.example:45678/chimera.sh\""));
    assert!(!body.contains("host_header/chimera.sh"));
    let _ = fs::remove_file(out_path);
    let _ = fs::remove_file(pubkey_path);
    let _ = fs::remove_file(keypair_path);
}

#[test]
fn nodes_advertise_writes_update_bootstrap_url_from_state_file() {
    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "chimera_mesh_discovery_update_url_state_{}.json",
        random_u64()
    ));
    let mut pubkey_path = std::env::temp_dir();
    pubkey_path.push(format!(
        "chimera_mesh_discovery_update_url_state_{}.pub",
        random_u64()
    ));
    let mut keypair_path = std::env::temp_dir();
    keypair_path.push(format!(
        "chimera_mesh_discovery_update_url_state_{}.keypair",
        random_u64()
    ));
    let mut update_state_path = std::env::temp_dir();
    update_state_path.push(format!("chimera_peer_update_state_{}.json", random_u64()));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("system clock failed: {err}"))
        .as_secs();
    fs::write(
        &update_state_path,
        format!(
            "{{\"kind\":\"chimera_peer_update_serve_state\",\"status\":\"ready\",\"listen\":\"127.0.0.1:45679\",\"base_url\":\"http://node.example:45679\",\"update_bootstrap_url\":\"http://node.example:45679/chimera.sh\",\"version\":\"1.2.3\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"endpoint_epoch\":{now}}}"
        ),
    )
    .unwrap_or_else(|err| unreachable!("{err}"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&update_state_path, fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|err| unreachable!("chmod state failed: {err}"));
    }
    let args = vec![
        "advertise".to_string(),
        "--node-id".to_string(),
        "node-eu-state".to_string(),
        "--endpoint".to_string(),
        "198.51.100.78:54321".to_string(),
        "--update-state-file".to_string(),
        update_state_path.display().to_string(),
        "--out".to_string(),
        out_path.display().to_string(),
        "--pubkey-out".to_string(),
        pubkey_path.display().to_string(),
        "--keypair-path".to_string(),
        keypair_path.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&args), 0);
    let body = fs::read_to_string(&out_path).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(body.contains("\"update_bootstrap_url\":\"http://node.example:45679/chimera.sh\""));
    assert!(!body.contains("host_header/chimera.sh"));
    let _ = fs::remove_file(out_path);
    let _ = fs::remove_file(pubkey_path);
    let _ = fs::remove_file(keypair_path);
    let _ = fs::remove_file(update_state_path);
}

#[test]
fn nodes_advertise_rewrites_unspecified_state_host_from_update_state() {
    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "chimera_mesh_discovery_unspecified_state_host_{}.json",
        random_u64()
    ));
    let mut pubkey_path = std::env::temp_dir();
    pubkey_path.push(format!(
        "chimera_mesh_discovery_unspecified_state_host_{}.pub",
        random_u64()
    ));
    let mut keypair_path = std::env::temp_dir();
    keypair_path.push(format!(
        "chimera_mesh_discovery_unspecified_state_host_{}.keypair",
        random_u64()
    ));
    let mut peer_egress_state_path = std::env::temp_dir();
    peer_egress_state_path.push(format!(
        "chimera_peer_egress_unspecified_state_{}.state",
        random_u64()
    ));
    let mut update_state_path = std::env::temp_dir();
    update_state_path.push(format!(
        "chimera_peer_update_unspecified_state_{}.json",
        random_u64()
    ));
    let port = 45678;
    fs::write(
        &peer_egress_state_path,
        format!(
            "mode=peer\nresolved_local_listen=127.0.0.1:{port}\nresolved_peer_listen=0.0.0.0:{port}\n"
        ),
    )
    .unwrap_or_else(|err| unreachable!("write peer egress state failed: {err}"));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("system clock failed: {err}"))
        .as_secs();
    fs::write(
        &update_state_path,
        format!(
            "{{\"kind\":\"chimera_peer_update_serve_state\",\"status\":\"ready\",\"listen\":\"127.0.0.1:{port}\",\"base_url\":\"http://node.example:{port}\",\"update_bootstrap_url\":\"http://node.example:{port}/chimera.sh\",\"version\":\"1.2.3\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"endpoint_epoch\":{now},\"endpoint_generation\":4}}"
        ),
    )
    .unwrap_or_else(|err| unreachable!("write peer update state failed: {err}"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&peer_egress_state_path, fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|err| unreachable!("chmod peer egress state failed: {err}"));
        fs::set_permissions(&update_state_path, fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|err| unreachable!("chmod update state failed: {err}"));
    }
    let args = vec![
        "advertise".to_string(),
        "--node-id".to_string(),
        "node-eu-unspecified".to_string(),
        "--state-file".to_string(),
        peer_egress_state_path.display().to_string(),
        "--update-state-file".to_string(),
        update_state_path.display().to_string(),
        "--out".to_string(),
        out_path.display().to_string(),
        "--pubkey-out".to_string(),
        pubkey_path.display().to_string(),
        "--keypair-path".to_string(),
        keypair_path.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&args), 0);
    let body = fs::read_to_string(&out_path).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(body.contains("\"endpoint\":\"node.example:45678\""));
    assert!(!body.contains("\"endpoint\":\"0.0.0.0:45678\""));
    let _ = fs::remove_file(out_path);
    let _ = fs::remove_file(pubkey_path);
    let _ = fs::remove_file(keypair_path);
    let _ = fs::remove_file(peer_egress_state_path);
    let _ = fs::remove_file(update_state_path);
}

#[test]
fn nodes_advertise_rejects_unspecified_state_host_without_update_state() {
    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "chimera_mesh_discovery_unspecified_state_fail_{}.json",
        random_u64()
    ));
    let mut pubkey_path = std::env::temp_dir();
    pubkey_path.push(format!(
        "chimera_mesh_discovery_unspecified_state_fail_{}.pub",
        random_u64()
    ));
    let mut keypair_path = std::env::temp_dir();
    keypair_path.push(format!(
        "chimera_mesh_discovery_unspecified_state_fail_{}.keypair",
        random_u64()
    ));
    let mut peer_egress_state_path = std::env::temp_dir();
    peer_egress_state_path.push(format!(
        "chimera_peer_egress_unspecified_state_fail_{}.state",
        random_u64()
    ));
    fs::write(
        &peer_egress_state_path,
        "mode=peer\nresolved_local_listen=127.0.0.1:45678\nresolved_peer_listen=0.0.0.0:45678\n",
    )
    .unwrap_or_else(|err| unreachable!("write peer egress state failed: {err}"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&peer_egress_state_path, fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|err| unreachable!("chmod peer egress state failed: {err}"));
    }
    let args = vec![
        "advertise".to_string(),
        "--node-id".to_string(),
        "node-eu-unspecified-fail".to_string(),
        "--state-file".to_string(),
        peer_egress_state_path.display().to_string(),
        "--out".to_string(),
        out_path.display().to_string(),
        "--pubkey-out".to_string(),
        pubkey_path.display().to_string(),
        "--keypair-path".to_string(),
        keypair_path.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&args), 2);
    let _ = fs::remove_file(out_path);
    let _ = fs::remove_file(pubkey_path);
    let _ = fs::remove_file(keypair_path);
    let _ = fs::remove_file(peer_egress_state_path);
}

#[test]
fn nodes_advertise_prefers_peer_egress_state_endpoint_over_inventory() {
    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "chimera_mesh_discovery_peer_egress_state_{}.json",
        random_u64()
    ));
    let mut pubkey_path = std::env::temp_dir();
    pubkey_path.push(format!(
        "chimera_mesh_discovery_peer_egress_state_{}.pub",
        random_u64()
    ));
    let mut keypair_path = std::env::temp_dir();
    keypair_path.push(format!(
        "chimera_mesh_discovery_peer_egress_state_{}.keypair",
        random_u64()
    ));
    let mut config_path = std::env::temp_dir();
    config_path.push(format!(
        "chimera_mesh_discovery_peer_egress_state_{}.conf",
        random_u64()
    ));
    let mut peer_egress_state_path = std::env::temp_dir();
    peer_egress_state_path.push(format!("chimera_peer_egress_state_{}.state", random_u64()));
    fs::write(
        &peer_egress_state_path,
        "mode=peer\nresolved_local_listen=127.0.0.1:11111\nresolved_peer_listen=198.51.100.44:45678\n",
    )
    .unwrap_or_else(|err| unreachable!("write peer egress state failed: {err}"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&peer_egress_state_path, fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|err| unreachable!("chmod peer egress state failed: {err}"));
    }
    fs::write(
        &config_path,
        "mesh.nodes.ids = de\nmesh.nodes.current = de\nmesh.node.de.endpoint = 198.51.100.99:54321\nmesh.node.de.country_code = DE\nmesh.node.de.country_name = Germany\nmesh.node.de.status = healthy\nmesh.node.de.observation_count = 10\n",
    )
    .unwrap_or_else(|err| unreachable!("write config failed: {err}"));
    let args = vec![
        "advertise".to_string(),
        "--config".to_string(),
        config_path.display().to_string(),
        "--node-id".to_string(),
        "node-eu-state-priority".to_string(),
        "--state-file".to_string(),
        peer_egress_state_path.display().to_string(),
        "--out".to_string(),
        out_path.display().to_string(),
        "--pubkey-out".to_string(),
        pubkey_path.display().to_string(),
        "--keypair-path".to_string(),
        keypair_path.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&args), 0);
    let body = fs::read_to_string(&out_path).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(body.contains("\"endpoint\":\"198.51.100.44:45678\""));
    assert!(!body.contains("198.51.100.99:54321"));
    let _ = fs::remove_file(out_path);
    let _ = fs::remove_file(pubkey_path);
    let _ = fs::remove_file(keypair_path);
    let _ = fs::remove_file(config_path);
    let _ = fs::remove_file(peer_egress_state_path);
}

#[test]
fn nodes_advertise_publishes_runtime_endpoint_and_update_state_together() {
    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "chimera_mesh_discovery_runtime_publish_{}.json",
        random_u64()
    ));
    let mut pubkey_path = std::env::temp_dir();
    pubkey_path.push(format!(
        "chimera_mesh_discovery_runtime_publish_{}.pub",
        random_u64()
    ));
    let mut keypair_path = std::env::temp_dir();
    keypair_path.push(format!(
        "chimera_mesh_discovery_runtime_publish_{}.keypair",
        random_u64()
    ));
    let mut peer_egress_state_path = std::env::temp_dir();
    peer_egress_state_path.push(format!(
        "chimera_peer_egress_publish_{}.state",
        random_u64()
    ));
    let mut update_state_path = std::env::temp_dir();
    update_state_path.push(format!("chimera_peer_update_publish_{}.json", random_u64()));
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read listener addr failed: {err}"));
    let port = addr.port();
    assert_ne!(port, 0);
    fs::write(
        &peer_egress_state_path,
        format!("mode=peer\nresolved_local_listen=127.0.0.1:{port}\nresolved_peer_listen=198.51.100.44:{port}\n"),
    )
    .unwrap_or_else(|err| unreachable!("write peer egress state failed: {err}"));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("system clock failed: {err}"))
        .as_secs();
    fs::write(
        &update_state_path,
        format!(
            "{{\"kind\":\"chimera_peer_update_serve_state\",\"status\":\"ready\",\"listen\":\"127.0.0.1:{port}\",\"base_url\":\"http://node.example:{port}\",\"update_bootstrap_url\":\"http://node.example:{port}/chimera.sh\",\"version\":\"1.2.3\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"endpoint_epoch\":{now},\"endpoint_generation\":4}}"
        ),
    )
    .unwrap_or_else(|err| unreachable!("write peer update state failed: {err}"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&peer_egress_state_path, fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|err| unreachable!("chmod peer egress state failed: {err}"));
        fs::set_permissions(&update_state_path, fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|err| unreachable!("chmod update state failed: {err}"));
    }
    let args = vec![
        "advertise".to_string(),
        "--node-id".to_string(),
        "node-eu-runtime-publish".to_string(),
        "--state-file".to_string(),
        peer_egress_state_path.display().to_string(),
        "--update-state-file".to_string(),
        update_state_path.display().to_string(),
        "--out".to_string(),
        out_path.display().to_string(),
        "--pubkey-out".to_string(),
        pubkey_path.display().to_string(),
        "--keypair-path".to_string(),
        keypair_path.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&args), 0);
    let body = fs::read_to_string(&out_path).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(body.contains(&format!("\"endpoint\":\"198.51.100.44:{port}\"")));
    assert!(body.contains(&format!(
        "\"update_bootstrap_url\":\"http://node.example:{port}/chimera.sh\""
    )));
    assert!(body.contains("\"endpoint_generation\":4"));
    assert!(!body.contains(":0"));
    assert!(!body.contains("host_header/chimera.sh"));
    let _ = fs::remove_file(out_path);
    let _ = fs::remove_file(pubkey_path);
    let _ = fs::remove_file(keypair_path);
    let _ = fs::remove_file(peer_egress_state_path);
    let _ = fs::remove_file(update_state_path);
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

#[test]
fn nodes_private_state_advertise_discovery_update_reaches_runtime_planner() {
    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "chimera_mesh_discovery_causality_{}.json",
        random_u64()
    ));
    let mut pubkey_path = std::env::temp_dir();
    pubkey_path.push(format!(
        "chimera_mesh_discovery_causality_{}.pub",
        random_u64()
    ));
    let mut keypair_path = std::env::temp_dir();
    keypair_path.push(format!(
        "chimera_mesh_discovery_causality_{}.keypair",
        random_u64()
    ));
    let mut peer_egress_state_path = std::env::temp_dir();
    peer_egress_state_path.push(format!(
        "chimera_peer_egress_causality_{}.state",
        random_u64()
    ));
    let mut update_state_path = std::env::temp_dir();
    update_state_path.push(format!(
        "chimera_peer_update_causality_{}.json",
        random_u64()
    ));
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read listener addr failed: {err}"));
    let port = addr.port();
    assert_ne!(port, 0);
    let old_endpoint = "198.51.100.40:443";
    let new_endpoint = format!("198.51.100.44:{port}");
    let update_url = format!("http://node.example:{port}/chimera.sh");
    fs::write(
        &peer_egress_state_path,
        format!(
            "mode=peer\nresolved_local_listen=127.0.0.1:{port}\nresolved_peer_listen={new_endpoint}\n"
        ),
    )
    .unwrap_or_else(|err| unreachable!("write peer egress state failed: {err}"));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("system clock failed: {err}"))
        .as_secs();
    fs::write(
        &update_state_path,
        format!(
            "{{\"kind\":\"chimera_peer_update_serve_state\",\"status\":\"ready\",\"listen\":\"127.0.0.1:{port}\",\"base_url\":\"http://node.example:{port}\",\"update_bootstrap_url\":\"{update_url}\",\"version\":\"1.2.3\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"endpoint_epoch\":{now},\"endpoint_generation\":12}}"
        ),
    )
    .unwrap_or_else(|err| unreachable!("write peer update state failed: {err}"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&peer_egress_state_path, fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|err| unreachable!("chmod peer egress state failed: {err}"));
        fs::set_permissions(&update_state_path, fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|err| unreachable!("chmod update state failed: {err}"));
    }
    let advertise_args = vec![
        "advertise".to_string(),
        "--node-id".to_string(),
        "node-eu-causality".to_string(),
        "--state-file".to_string(),
        peer_egress_state_path.display().to_string(),
        "--update-state-file".to_string(),
        update_state_path.display().to_string(),
        "--out".to_string(),
        out_path.display().to_string(),
        "--pubkey-out".to_string(),
        pubkey_path.display().to_string(),
        "--keypair-path".to_string(),
        keypair_path.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&advertise_args), 0);
    let artifact = fs::read_to_string(&out_path).unwrap_or_else(|err| unreachable!("{err}"));
    assert!(artifact.contains(&format!("\"endpoint\":\"{new_endpoint}\"")));
    assert!(artifact.contains(&format!("\"update_bootstrap_url\":\"{update_url}\"")));
    assert!(artifact.contains("\"endpoint_generation\":12"));

    let pubkey = fs::read_to_string(&pubkey_path).unwrap_or_else(|err| unreachable!("{err}"));
    let discovery_url = serve_json_once(artifact);
    let discovery_args = vec![
        "--probe-timeout-ms".to_string(),
        "200".to_string(),
        "--discovery-url".to_string(),
        discovery_url,
        "--discovery-pubkey".to_string(),
        pubkey,
    ];
    let inventory =
        load_mesh_nodes_inventory(&discovery_args).unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(inventory.nodes.len(), 1);
    assert_eq!(inventory.nodes[0].endpoint, new_endpoint);
    assert_eq!(
        inventory.nodes[0].update_bootstrap_url.as_deref(),
        Some(update_url.as_str())
    );
    assert_eq!(inventory.nodes[0].endpoint_generation, Some(12));
    let endpoint_updates =
        published_endpoint_updates_from_nodes(&inventory.nodes).unwrap_or_else(|err| {
            unreachable!("published endpoint updates should build from discovery inventory: {err}")
        });
    assert_eq!(endpoint_updates.len(), 1);
    assert_eq!(endpoint_updates[0].endpoint, new_endpoint);
    assert_eq!(endpoint_updates[0].endpoint_generation, 12);

    let mut runtime =
        MeshRuntime::bootstrap("mesh-nodes", "seed-a").unwrap_or_else(|err| unreachable!("{err}"));
    runtime
        .merge_discovery(
            "seed-b",
            &[MeshDiscoveryRecord {
                node_id: "node-eu-causality".to_string(),
                endpoint: old_endpoint.to_string(),
                region: "DE".to_string(),
                load_score: 1,
                reliability_score: 99,
            }],
        )
        .unwrap_or_else(|err| unreachable!("{err}"));
    let request = MeshJoinRequest {
        namespace: "mesh-nodes".to_string(),
        node_name: "probe".to_string(),
        invite_token: None,
    };
    let policy = MeshPathPolicy::default_auto();
    let before_plan = runtime
        .plan_path(&request, &policy)
        .unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(before_plan.selected_peers[0].endpoint, old_endpoint);

    runtime
        .merge_published_endpoint_updates("discovery-causality", &endpoint_updates)
        .unwrap_or_else(|err| unreachable!("{err}"));
    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("endpoint update must mark planner rebuild"));
    assert_eq!(signal.reason(), "published_endpoint_changed");
    assert_eq!(signal.affected_peer_count(), 1);
    let after_plan = runtime
        .plan_path(&request, &policy)
        .unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(after_plan.selected_peers[0].endpoint, new_endpoint);

    runtime
        .merge_published_endpoint_updates("discovery-causality", &endpoint_updates)
        .unwrap_or_else(|err| unreachable!("identical update must be no-op: {err}"));
    let identical_plan = runtime
        .plan_path(&request, &policy)
        .unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(identical_plan.selected_peers[0].endpoint, new_endpoint);

    let stale_update = chimera_mesh::MeshPublishedEndpointUpdate {
        node_id: "node-eu-causality".to_string(),
        endpoint: old_endpoint.to_string(),
        update_bootstrap_url: None,
        endpoint_generation: 11,
    };
    runtime
        .merge_published_endpoint_updates("discovery-causality", &[stale_update])
        .unwrap_or_else(|err| unreachable!("stale update should be ignored: {err}"));
    let stale_plan = runtime
        .plan_path(&request, &policy)
        .unwrap_or_else(|err| unreachable!("{err}"));
    assert_eq!(stale_plan.selected_peers[0].endpoint, new_endpoint);

    let _ = fs::remove_file(out_path);
    let _ = fs::remove_file(pubkey_path);
    let _ = fs::remove_file(keypair_path);
    let _ = fs::remove_file(peer_egress_state_path);
    let _ = fs::remove_file(update_state_path);
}

#[test]
fn nodes_advertise_rejects_loopback_update_bootstrap_url() {
    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "chimera_mesh_discovery_bad_update_url_{}.json",
        random_u64()
    ));
    let mut pubkey_path = std::env::temp_dir();
    pubkey_path.push(format!(
        "chimera_mesh_discovery_bad_update_url_{}.pub",
        random_u64()
    ));
    let mut keypair_path = std::env::temp_dir();
    keypair_path.push(format!(
        "chimera_mesh_discovery_bad_update_url_{}.keypair",
        random_u64()
    ));
    let args = vec![
        "advertise".to_string(),
        "--node-id".to_string(),
        "node-eu-bad".to_string(),
        "--endpoint".to_string(),
        "198.51.100.79:54321".to_string(),
        "--update-bootstrap-url".to_string(),
        "http://127.0.0.1:45678/chimera.sh".to_string(),
        "--out".to_string(),
        out_path.display().to_string(),
        "--pubkey-out".to_string(),
        pubkey_path.display().to_string(),
        "--keypair-path".to_string(),
        keypair_path.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&args), 2);
    let _ = fs::remove_file(out_path);
    let _ = fs::remove_file(pubkey_path);
    let _ = fs::remove_file(keypair_path);
}
