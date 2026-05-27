use super::nodes_cmd::mesh_nodes_command;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn nodes_reenroll_writes_request_file() {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut out = std::env::temp_dir();
    out.push(format!("chimera_reenroll_request_{ts}.json"));
    let args = vec![
        "re-enroll".to_string(),
        "--self-node-id".to_string(),
        "self-a".to_string(),
        "--new-node-id".to_string(),
        "self-b".to_string(),
        "--out".to_string(),
        out.display().to_string(),
    ];
    let code = mesh_nodes_command(&args);
    assert_eq!(code, 0);
    let body = fs::read_to_string(&out).unwrap_or_else(|e| unreachable!("{e}"));
    assert!(body.contains("\"kind\":\"mesh_reenroll_request\""));
    assert!(body.contains("\"current_node_id\":\"self-a\""));
    assert!(body.contains("\"new_node_id\":\"self-b\""));
    assert!(body.contains("\"next_step\":\"issue_new_keypair_and_register\""));
    let _ = fs::remove_file(out);
}

#[test]
fn nodes_reenroll_requires_different_node_id() {
    let args = vec![
        "re-enroll".to_string(),
        "--self-node-id".to_string(),
        "self-a".to_string(),
        "--new-node-id".to_string(),
        "self-a".to_string(),
    ];
    let code = mesh_nodes_command(&args);
    assert_eq!(code, 2);
}

#[test]
fn nodes_reenroll_prepare_writes_register_and_key_files() {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut request = std::env::temp_dir();
    request.push(format!("chimera_reenroll_request_in_{ts}.json"));
    let mut out = std::env::temp_dir();
    out.push(format!("chimera_reenroll_register_out_{ts}.json"));
    let mut key_out = std::env::temp_dir();
    key_out.push(format!("chimera_reenroll_key_out_{ts}.json"));
    let request_json = "{\"kind\":\"mesh_reenroll_request\",\"current_node_id\":\"self-a\",\"new_node_id\":\"self-b\",\"restricted_mode\":true}";
    fs::write(&request, request_json).unwrap_or_else(|e| unreachable!("{e}"));

    let args = vec![
        "re-enroll-prepare".to_string(),
        "--request".to_string(),
        request.display().to_string(),
        "--out".to_string(),
        out.display().to_string(),
        "--key-out".to_string(),
        key_out.display().to_string(),
    ];
    let code = mesh_nodes_command(&args);
    assert_eq!(code, 0);

    let out_body = fs::read_to_string(&out).unwrap_or_else(|e| unreachable!("{e}"));
    assert!(out_body.contains("\"kind\":\"mesh_reenroll_register\""));
    assert!(out_body.contains("\"node_id\":\"self-b\""));
    assert!(out_body.contains("\"proof_signature\":\""));

    let key_body = fs::read_to_string(&key_out).unwrap_or_else(|e| unreachable!("{e}"));
    assert!(key_body.contains("\"kind\":\"mesh_reenroll_key_material\""));
    assert!(key_body.contains("\"node_id\":\"self-b\""));
    assert!(key_body.contains("\"pkcs8_base64\":\""));

    let _ = fs::remove_file(request);
    let _ = fs::remove_file(out);
    let _ = fs::remove_file(key_out);
}

#[test]
fn nodes_reenroll_submit_writes_identity_state() {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut request = std::env::temp_dir();
    request.push(format!("chimera_reenroll_submit_request_{ts}.json"));
    let mut register_out = std::env::temp_dir();
    register_out.push(format!("chimera_reenroll_submit_register_{ts}.json"));
    let mut key_out = std::env::temp_dir();
    key_out.push(format!("chimera_reenroll_submit_key_{ts}.json"));
    let mut state_out = std::env::temp_dir();
    state_out.push(format!("chimera_reenroll_submit_state_{ts}.json"));
    let request_json = "{\"kind\":\"mesh_reenroll_request\",\"current_node_id\":\"self-a\",\"new_node_id\":\"self-c\",\"restricted_mode\":true}";
    fs::write(&request, request_json).unwrap_or_else(|e| unreachable!("{e}"));

    let prepare_args = vec![
        "re-enroll-prepare".to_string(),
        "--request".to_string(),
        request.display().to_string(),
        "--out".to_string(),
        register_out.display().to_string(),
        "--key-out".to_string(),
        key_out.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&prepare_args), 0);

    let submit_args = vec![
        "re-enroll-submit".to_string(),
        "--register".to_string(),
        register_out.display().to_string(),
        "--key".to_string(),
        key_out.display().to_string(),
        "--state-out".to_string(),
        state_out.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&submit_args), 0);

    let state = fs::read_to_string(&state_out).unwrap_or_else(|e| unreachable!("{e}"));
    assert!(state.contains("\"kind\":\"mesh_identity_state\""));
    assert!(state.contains("\"self_node_id\":\"self-c\""));
    assert!(state.contains("\"restricted_mode\":false"));

    let _ = fs::remove_file(request);
    let _ = fs::remove_file(register_out);
    let _ = fs::remove_file(key_out);
    let _ = fs::remove_file(state_out);
}

#[test]
fn nodes_reenroll_submit_uses_config_identity_state_path() {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut request = std::env::temp_dir();
    request.push(format!("chimera_reenroll_env_request_{ts}.json"));
    let mut register_out = std::env::temp_dir();
    register_out.push(format!("chimera_reenroll_env_register_{ts}.json"));
    let mut key_out = std::env::temp_dir();
    key_out.push(format!("chimera_reenroll_env_key_{ts}.json"));
    let mut state_out = std::env::temp_dir();
    state_out.push(format!("chimera_reenroll_cfg_state_{ts}.json"));
    let mut config_path = std::env::temp_dir();
    config_path.push(format!("chimera_reenroll_cfg_{ts}.conf"));
    let mut activation_out = std::env::temp_dir();
    activation_out.push(format!("chimera_reenroll_activation_{ts}.json"));
    let request_json = "{\"kind\":\"mesh_reenroll_request\",\"current_node_id\":\"self-a\",\"new_node_id\":\"self-d\",\"restricted_mode\":true}";
    fs::write(&request, request_json).unwrap_or_else(|e| unreachable!("{e}"));

    let prepare_args = vec![
        "re-enroll-prepare".to_string(),
        "--request".to_string(),
        request.display().to_string(),
        "--out".to_string(),
        register_out.display().to_string(),
        "--key-out".to_string(),
        key_out.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&prepare_args), 0);

    let config_body = format!(
        "mesh.nodes.ids = self-d\nmesh.node.self-d.endpoint = 127.0.0.1:443\nmesh.nodes.identity_state_path = {}\nmesh.nodes.activation_log_path = {}\n",
        state_out.display(),
        activation_out.display()
    );
    fs::write(&config_path, config_body).unwrap_or_else(|e| unreachable!("{e}"));
    let submit_args = vec![
        "re-enroll-submit".to_string(),
        "--config".to_string(),
        config_path.display().to_string(),
        "--register".to_string(),
        register_out.display().to_string(),
        "--key".to_string(),
        key_out.display().to_string(),
    ];
    assert_eq!(mesh_nodes_command(&submit_args), 0);

    let state = fs::read_to_string(&state_out).unwrap_or_else(|e| unreachable!("{e}"));
    assert!(state.contains("\"self_node_id\":\"self-d\""));
    let activation = fs::read_to_string(&activation_out).unwrap_or_else(|e| unreachable!("{e}"));
    assert!(activation.contains("\"kind\":\"mesh_reenroll_activation\""));
    assert!(activation.contains("\"self_node_id\":\"self-d\""));

    let _ = fs::remove_file(request);
    let _ = fs::remove_file(register_out);
    let _ = fs::remove_file(key_out);
    let _ = fs::remove_file(state_out);
    let _ = fs::remove_file(activation_out);
    let _ = fs::remove_file(config_path);
}
