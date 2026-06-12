use super::helpers::random_u64;
use crate::mesh_cli::nodes_cmd::{
    render_nodes_json_error, render_probe_all_json, render_state_view_json,
};
use crate::mesh_cli::nodes_inventory::load_mesh_nodes_inventory;
use chimera_mesh::{MeshConnectAttempt, MeshConnectProbeReport};
use std::{fs, net::TcpListener};

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
