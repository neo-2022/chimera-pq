use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use chimera_mesh::{
    MeshConnectProbeReport, MeshDiscoveryRecord, MeshJoinRequest, MeshNodeId, MeshNodeListFilter,
    MeshNodeRuntime, MeshNodeStatus, MeshNodesPolicy, MeshPathPolicy, MeshRuntime,
    build_mesh_node_explain, group_mesh_nodes_by_country, refresh_mesh_node_scores,
    render_mesh_node_explain,
};
use ring::hmac;
use ring::{
    rand::SystemRandom,
    signature::{ED25519, Ed25519KeyPair, KeyPair, UnparsedPublicKey},
};

use super::nodes_inventory::{
    MeshNodesInventory, default_runtime_state_path, extract_flag_value, load_mesh_nodes_inventory,
};
use super::nodes_render::{render_best, render_nodes_list};
use super::nodes_selection::{choose_node_id, resolve_node_id_selector};

pub(crate) fn mesh_nodes_command(args: &[String]) -> i32 {
    let Some(subcommand) = args.first().map(String::as_str) else {
        eprintln!("{}", usage());
        return 2;
    };
    let rest = &args[1..];
    let mut inventory = match load_mesh_nodes_inventory(rest) {
        Ok(inventory) => inventory,
        Err(error) => {
            eprintln!("mesh nodes error: {error}");
            return 2;
        }
    };
    let policy = MeshNodesPolicy::default();
    refresh_mesh_node_scores(&mut inventory.nodes, &policy);

    match subcommand {
        "list" => match parse_filter(rest) {
            Ok(filter) => {
                println!("{}", render_nodes_list(&inventory, &filter));
                print_list_next_step_hint();
                0
            }
            Err(error) => {
                eprintln!("mesh nodes list error: {error}");
                2
            }
        },
        "best" => {
            println!("{}", render_best(&inventory.nodes));
            0
        }
        "explain" => explain_node(rest, &inventory.nodes, &policy),
        "connect" => connect_node(rest, &inventory, &policy),
        "select" => select_node(rest, &inventory, &policy),
        "selected-endpoint" => selected_endpoint(rest, &inventory),
        "selected-invite-token" => selected_invite_token(rest, &inventory),
        "pin" => pin_node(rest, &inventory, &policy),
        "unpin" => unpin_node(rest, &inventory, &policy),
        "autoconnect" => autoconnect(rest, &inventory, &policy),
        "auto-unblock" => auto_unblock(rest, &inventory),
        "guard-listen" => guard_listen(rest),
        "state" => state_cmd(rest, &inventory),
        "advertise" => advertise_node(rest, &inventory),
        "re-enroll" => re_enroll_node(rest, &inventory),
        "re-enroll-prepare" => re_enroll_prepare(rest),
        "re-enroll-submit" => re_enroll_submit(rest),
        "probe" if rest.first().map(String::as_str) == Some("--all") => probe_all(rest, &inventory),
        _ => {
            eprintln!("{}", usage());
            2
        }
    }
}

fn usage() -> &'static str {
    "usage: chimera mesh nodes <list|best|explain|connect|select|selected-endpoint|selected-invite-token|pin|unpin|autoconnect|auto-unblock|guard-listen|state|advertise|re-enroll|re-enroll-prepare|re-enroll-submit|probe> [--config path] [--self-node-id <id>] [--runtime-state <file>] [--namespace <name>] [--json] [--proof-token <token>] [--proof-token-classic <token>] [--proof-token-pq <token>] [--proof-key-id <id>] [--proof-pq-key-id <id>] [--bind <host:port>] [--once] [--discovery-url http(s)://...] [--probe-timeout-ms n] [--node <id@endpoint@country_code@country_name@status@latency_ms@jitter_ms@loss_pct@success5m@success1h@failures@observations>] [--country DE,NL] [--status healthy,checking] [--available-only] [--search text] [--id node_id] [--new-node-id <id>] [--request <file>] [--out <file>] [--key-out <file>] [--register <file>] [--key <file>] [--state-out <file>] [--activation-out <file>]"
}

fn explain_node(
    args: &[String],
    nodes: &[chimera_mesh::MeshNode],
    policy: &MeshNodesPolicy,
) -> i32 {
    let Some(id) = extract_flag_value(args, "--id") else {
        eprintln!("mesh nodes explain error: --id is required");
        return 2;
    };
    let Some(node) = nodes.iter().find(|node| node.node_id.0 == id) else {
        eprintln!("mesh nodes explain error: node not found");
        return 2;
    };
    println!(
        "{}",
        render_mesh_node_explain(&build_mesh_node_explain(node, policy))
    );
    0
}

fn connect_node(args: &[String], inventory: &MeshNodesInventory, policy: &MeshNodesPolicy) -> i32 {
    if let Some(reason) = inventory.restricted_reason.as_deref() {
        eprintln!("mesh nodes connect error: restricted mode ({reason})");
        return 2;
    }
    let id = match resolve_node_id_selector(args, inventory) {
        Ok(id) => id,
        Err(error) => {
            eprintln!("mesh nodes connect error: {error}");
            eprintln!(
                "mesh nodes connect hint: use 'chimera mesh nodes select' for interactive choice"
            );
            return 2;
        }
    };
    let mut runtime = match build_runtime_from_inventory(inventory, policy, "connect") {
        Ok(runtime) => runtime,
        Err(code) => return code,
    };
    let decision = runtime.manual_connect(&inventory.nodes, &MeshNodeId::new(&id), 0);
    let selected_node = decision
        .candidate_node
        .as_ref()
        .map(|node| node.0.as_str())
        .unwrap_or(id.as_str());
    if decision.allowed
        && let Err(error) = persist_runtime_state(args, &runtime)
    {
        eprintln!("mesh nodes connect error: persist runtime state failed: {error}");
        return 1;
    }
    if decision.allowed {
        println!("Подключение: выполнено");
        println!("Узел: {selected_node}");
        print_connect_next_step_hint();
        0
    } else {
        eprintln!("Подключение не выполнено: {}", decision.reason);
        eprintln!("Проверьте список узлов: chimera mesh nodes list");
        2
    }
}

fn select_node(args: &[String], inventory: &MeshNodesInventory, policy: &MeshNodesPolicy) -> i32 {
    if let Some(reason) = inventory.restricted_reason.as_deref() {
        eprintln!("mesh nodes select error: restricted mode ({reason})");
        return 2;
    }
    let id = match choose_node_id(args, inventory) {
        Ok(id) => id,
        Err(error) => {
            eprintln!("mesh nodes select error: {error}");
            return 2;
        }
    };
    let mut runtime = match build_runtime_from_inventory(inventory, policy, "select") {
        Ok(runtime) => runtime,
        Err(code) => return code,
    };
    let decision = runtime.manual_connect(&inventory.nodes, &MeshNodeId::new(&id), 0);
    if !decision.allowed {
        eprintln!("mesh nodes select error: {}", decision.reason);
        return 2;
    }
    let pin_decision = runtime.pin(MeshNodeId::new(&id));
    if !pin_decision.allowed {
        eprintln!("mesh nodes select error: {}", pin_decision.reason);
        return 2;
    }
    runtime.set_autoconnect(true);
    if let Err(error) = persist_runtime_state(args, &runtime) {
        eprintln!("mesh nodes select error: persist runtime state failed: {error}");
        return 1;
    }
    println!("Выбран узел: {id}");
    println!("Подключение: выполнено");
    println!("Закрепление: выполнено");
    println!("Автоподключение: включено");
    println!("Режим: mesh peer");
    println!("next: chimera mesh nodes state");
    0
}

pub(crate) fn selected_node_endpoint(inventory: &MeshNodesInventory) -> Option<&str> {
    let selected_id = inventory
        .current_node
        .as_ref()
        .or(inventory.pinned_node.as_ref())?;
    inventory
        .nodes
        .iter()
        .find(|node| node.node_id == *selected_id)
        .map(|node| node.endpoint.as_str())
}

pub(crate) fn selected_node_invite_token(inventory: &MeshNodesInventory) -> Option<&str> {
    let selected_id = inventory
        .current_node
        .as_ref()
        .or(inventory.pinned_node.as_ref())?;
    inventory
        .nodes
        .iter()
        .find(|node| node.node_id == *selected_id)
        .and_then(|node| node.invite_token.as_deref())
}

fn selected_endpoint(_args: &[String], inventory: &MeshNodesInventory) -> i32 {
    if let Some(reason) = inventory.restricted_reason.as_deref() {
        eprintln!("mesh nodes selected-endpoint error: restricted mode ({reason})");
        return 2;
    }
    match selected_node_endpoint(inventory) {
        Some(endpoint) => {
            println!("{endpoint}");
            0
        }
        None => {
            eprintln!("mesh nodes selected-endpoint error: no selected node");
            2
        }
    }
}

fn selected_invite_token(_args: &[String], inventory: &MeshNodesInventory) -> i32 {
    if let Some(reason) = inventory.restricted_reason.as_deref() {
        eprintln!("mesh nodes selected-invite-token error: restricted mode ({reason})");
        return 2;
    }
    match selected_node_invite_token(inventory) {
        Some(token) => {
            println!("{token}");
            0
        }
        None => {
            eprintln!("mesh nodes selected-invite-token error: no selected node invite token");
            2
        }
    }
}

fn pin_node(args: &[String], inventory: &MeshNodesInventory, policy: &MeshNodesPolicy) -> i32 {
    if let Some(reason) = inventory.restricted_reason.as_deref() {
        eprintln!("mesh nodes pin error: restricted mode ({reason})");
        return 2;
    }
    let id = match resolve_node_id_selector(args, inventory) {
        Ok(id) => id,
        Err(error) => {
            eprintln!("mesh nodes pin error: {error}");
            return 2;
        }
    };
    let mut runtime = match build_runtime_from_inventory(inventory, policy, "pin") {
        Ok(runtime) => runtime,
        Err(code) => return code,
    };
    let decision = runtime.pin(MeshNodeId::new(&id));
    println!(
        "action={:?} allowed={} reason={}",
        decision.action, decision.allowed, decision.reason
    );
    if decision.allowed
        && let Err(error) = persist_runtime_state(args, &runtime)
    {
        eprintln!("mesh nodes pin error: persist runtime state failed: {error}");
        return 1;
    }
    print_pin_next_step_hint();
    0
}

fn unpin_node(args: &[String], inventory: &MeshNodesInventory, policy: &MeshNodesPolicy) -> i32 {
    if let Some(reason) = inventory.restricted_reason.as_deref() {
        eprintln!("mesh nodes unpin error: restricted mode ({reason})");
        return 2;
    }
    let mut runtime = match build_runtime_from_inventory(inventory, policy, "unpin") {
        Ok(runtime) => runtime,
        Err(code) => return code,
    };
    let decision = runtime.unpin();
    println!(
        "action={:?} allowed={} reason={}",
        decision.action, decision.allowed, decision.reason
    );
    if decision.allowed
        && let Err(error) = persist_runtime_state(args, &runtime)
    {
        eprintln!("mesh nodes unpin error: persist runtime state failed: {error}");
        return 1;
    }
    0
}

fn autoconnect(args: &[String], inventory: &MeshNodesInventory, policy: &MeshNodesPolicy) -> i32 {
    if let Some(reason) = inventory.restricted_reason.as_deref() {
        eprintln!("mesh nodes autoconnect error: restricted mode ({reason})");
        return 2;
    }
    let mut runtime = match build_runtime_from_inventory(inventory, policy, "autoconnect") {
        Ok(runtime) => runtime,
        Err(code) => return code,
    };
    match args.first().map(String::as_str) {
        Some("on") => {
            runtime.set_autoconnect(true);
            if let Err(error) = persist_runtime_state(args, &runtime) {
                eprintln!("mesh nodes autoconnect error: persist runtime state failed: {error}");
                return 1;
            }
            println!("autoconnect=on");
            0
        }
        Some("off") => {
            runtime.set_autoconnect(false);
            if let Err(error) = persist_runtime_state(args, &runtime) {
                eprintln!("mesh nodes autoconnect error: persist runtime state failed: {error}");
                return 1;
            }
            println!("autoconnect=off");
            0
        }
        _ => {
            eprintln!("mesh nodes autoconnect error: expected on|off");
            2
        }
    }
}

fn re_enroll_node(args: &[String], inventory: &MeshNodesInventory) -> i32 {
    let Some(current) = inventory.self_node_id.as_ref() else {
        eprintln!("mesh nodes re-enroll error: self node id is not configured");
        return 2;
    };
    let Some(new_node_id) = extract_flag_value(args, "--new-node-id") else {
        eprintln!("mesh nodes re-enroll error: --new-node-id is required");
        return 2;
    };
    if let Err(error) = MeshNodeId::new(new_node_id).validate() {
        eprintln!("mesh nodes re-enroll error: {error}");
        return 2;
    }
    if current.0 == new_node_id {
        eprintln!(
            "mesh nodes re-enroll error: --new-node-id must differ from current self node id"
        );
        return 2;
    }
    let json = match build_re_enroll_request_json(current.0.as_str(), new_node_id, inventory) {
        Ok(json) => json,
        Err(error) => {
            eprintln!("mesh nodes re-enroll error: {error}");
            return 2;
        }
    };
    if let Some(out_path) = extract_flag_value(args, "--out") {
        if let Err(error) = std::fs::write(out_path, &json) {
            eprintln!("mesh nodes re-enroll error: write failed: {error}");
            return 2;
        }
        println!("re_enroll=request_written out={out_path}");
        return 0;
    }
    println!("{json}");
    0
}

fn build_re_enroll_request_json(
    current_node_id: &str,
    new_node_id: &str,
    inventory: &MeshNodesInventory,
) -> Result<String, String> {
    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_secs();
    let restricted = inventory.restricted_reason.is_some();
    let restricted_reason = inventory
        .restricted_reason
        .as_deref()
        .unwrap_or("none")
        .replace('"', "'");
    Ok(format!(
        "{{\"kind\":\"mesh_reenroll_request\",\"status\":\"accepted\",\"current_node_id\":\"{}\",\"new_node_id\":\"{}\",\"restricted_mode\":{},\"restricted_reason\":\"{}\",\"issued_at_unix\":{},\"next_step\":\"issue_new_keypair_and_register\"}}",
        current_node_id,
        new_node_id,
        if restricted { "true" } else { "false" },
        restricted_reason,
        now_unix
    ))
}

fn re_enroll_prepare(args: &[String]) -> i32 {
    let Some(request_path) = extract_flag_value(args, "--request") else {
        eprintln!("mesh nodes re-enroll-prepare error: --request is required");
        return 2;
    };
    let Some(out_path) = extract_flag_value(args, "--out") else {
        eprintln!("mesh nodes re-enroll-prepare error: --out is required");
        return 2;
    };
    let Some(key_out_path) = extract_flag_value(args, "--key-out") else {
        eprintln!("mesh nodes re-enroll-prepare error: --key-out is required");
        return 2;
    };
    let request_text = match std::fs::read_to_string(request_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh nodes re-enroll-prepare error: read request failed: {error}");
            return 2;
        }
    };
    let request: serde_json::Value = match serde_json::from_str(&request_text) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh nodes re-enroll-prepare error: invalid request json: {error}");
            return 2;
        }
    };
    let Some(kind) = request.get("kind").and_then(serde_json::Value::as_str) else {
        eprintln!("mesh nodes re-enroll-prepare error: request missing kind");
        return 2;
    };
    if kind != "mesh_reenroll_request" {
        eprintln!("mesh nodes re-enroll-prepare error: unsupported kind '{kind}'");
        return 2;
    }
    let Some(new_node_id) = request
        .get("new_node_id")
        .and_then(serde_json::Value::as_str)
    else {
        eprintln!("mesh nodes re-enroll-prepare error: request missing new_node_id");
        return 2;
    };
    if let Err(error) = MeshNodeId::new(new_node_id).validate() {
        eprintln!("mesh nodes re-enroll-prepare error: {error}");
        return 2;
    }
    let now_unix = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_secs(),
        Err(error) => {
            eprintln!("mesh nodes re-enroll-prepare error: system clock error: {error}");
            return 2;
        }
    };
    let nonce = format!("{new_node_id}-{now_unix}");
    let keypair_pkcs8 = match Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("mesh nodes re-enroll-prepare error: keypair generation failed");
            return 2;
        }
    };
    let keypair = match Ed25519KeyPair::from_pkcs8(keypair_pkcs8.as_ref()) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("mesh nodes re-enroll-prepare error: keypair parse failed");
            return 2;
        }
    };
    let message = format!(
        "kind=mesh_reenroll_register\nnode_id={new_node_id}\nissued_at_unix={now_unix}\nnonce={nonce}\n"
    );
    let signature = keypair.sign(message.as_bytes());
    let pubkey_b64 =
        base64::engine::general_purpose::STANDARD.encode(keypair.public_key().as_ref());
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.as_ref());
    let key_pkcs8_b64 = base64::engine::general_purpose::STANDARD.encode(keypair_pkcs8.as_ref());
    let register_json = format!(
        "{{\"kind\":\"mesh_reenroll_register\",\"node_id\":\"{}\",\"pubkey\":\"{}\",\"issued_at_unix\":{},\"nonce\":\"{}\",\"proof_signature\":\"{}\",\"proof_message\":\"{}\"}}",
        new_node_id,
        pubkey_b64,
        now_unix,
        nonce,
        signature_b64,
        message.replace('\n', "\\n")
    );
    let key_json = format!(
        "{{\"kind\":\"mesh_reenroll_key_material\",\"node_id\":\"{}\",\"algorithm\":\"ed25519\",\"pkcs8_base64\":\"{}\"}}",
        new_node_id, key_pkcs8_b64
    );
    if let Err(error) = std::fs::write(out_path, register_json) {
        eprintln!("mesh nodes re-enroll-prepare error: write --out failed: {error}");
        return 2;
    }
    if let Err(error) = std::fs::write(key_out_path, key_json) {
        eprintln!("mesh nodes re-enroll-prepare error: write --key-out failed: {error}");
        return 2;
    }
    println!(
        "re_enroll_prepare=ok out={} key_out={}",
        out_path, key_out_path
    );
    0
}

fn re_enroll_submit(args: &[String]) -> i32 {
    let Some(register_path) = extract_flag_value(args, "--register") else {
        eprintln!("mesh nodes re-enroll-submit error: --register is required");
        return 2;
    };
    let Some(key_path) = extract_flag_value(args, "--key") else {
        eprintln!("mesh nodes re-enroll-submit error: --key is required");
        return 2;
    };
    let Some(state_out_path) = resolve_identity_state_out_path(args) else {
        eprintln!(
            "mesh nodes re-enroll-submit error: --state-out (or config/env identity_state_path) is required"
        );
        return 2;
    };
    let register_text = match std::fs::read_to_string(register_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh nodes re-enroll-submit error: read register failed: {error}");
            return 2;
        }
    };
    let key_text = match std::fs::read_to_string(key_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh nodes re-enroll-submit error: read key failed: {error}");
            return 2;
        }
    };
    let register: serde_json::Value = match serde_json::from_str(&register_text) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh nodes re-enroll-submit error: invalid register json: {error}");
            return 2;
        }
    };
    let key_material: serde_json::Value = match serde_json::from_str(&key_text) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh nodes re-enroll-submit error: invalid key json: {error}");
            return 2;
        }
    };
    let Some(node_id) = register.get("node_id").and_then(serde_json::Value::as_str) else {
        eprintln!("mesh nodes re-enroll-submit error: register missing node_id");
        return 2;
    };
    let Some(pubkey_b64) = register.get("pubkey").and_then(serde_json::Value::as_str) else {
        eprintln!("mesh nodes re-enroll-submit error: register missing pubkey");
        return 2;
    };
    let Some(proof_signature_b64) = register
        .get("proof_signature")
        .and_then(serde_json::Value::as_str)
    else {
        eprintln!("mesh nodes re-enroll-submit error: register missing proof_signature");
        return 2;
    };
    let Some(proof_message) = register
        .get("proof_message")
        .and_then(serde_json::Value::as_str)
    else {
        eprintln!("mesh nodes re-enroll-submit error: register missing proof_message");
        return 2;
    };
    let Some(key_node_id) = key_material
        .get("node_id")
        .and_then(serde_json::Value::as_str)
    else {
        eprintln!("mesh nodes re-enroll-submit error: key material missing node_id");
        return 2;
    };
    if key_node_id != node_id {
        eprintln!(
            "mesh nodes re-enroll-submit error: node_id mismatch between register and key material"
        );
        return 2;
    }
    let Some(pkcs8_b64) = key_material
        .get("pkcs8_base64")
        .and_then(serde_json::Value::as_str)
    else {
        eprintln!("mesh nodes re-enroll-submit error: key material missing pkcs8_base64");
        return 2;
    };
    let pkcs8 = match base64::engine::general_purpose::STANDARD.decode(pkcs8_b64) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "mesh nodes re-enroll-submit error: key material base64 decode failed: {error}"
            );
            return 2;
        }
    };
    let keypair = match Ed25519KeyPair::from_pkcs8(&pkcs8) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("mesh nodes re-enroll-submit error: key material parse failed");
            return 2;
        }
    };
    let derived_pubkey_b64 =
        base64::engine::general_purpose::STANDARD.encode(keypair.public_key().as_ref());
    if derived_pubkey_b64 != pubkey_b64 {
        eprintln!("mesh nodes re-enroll-submit error: pubkey mismatch with key material");
        return 2;
    }
    let proof_signature = match base64::engine::general_purpose::STANDARD
        .decode(proof_signature_b64)
    {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "mesh nodes re-enroll-submit error: proof signature base64 decode failed: {error}"
            );
            return 2;
        }
    };
    let verifier = UnparsedPublicKey::new(&ED25519, keypair.public_key().as_ref());
    if verifier
        .verify(proof_message.as_bytes(), &proof_signature)
        .is_err()
    {
        eprintln!("mesh nodes re-enroll-submit error: proof signature verification failed");
        return 2;
    }
    let now_unix = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_secs(),
        Err(error) => {
            eprintln!("mesh nodes re-enroll-submit error: system clock error: {error}");
            return 2;
        }
    };
    let state_json = format!(
        "{{\"kind\":\"mesh_identity_state\",\"status\":\"active\",\"self_node_id\":\"{}\",\"pubkey\":\"{}\",\"activated_at_unix\":{},\"restricted_mode\":false}}",
        node_id, pubkey_b64, now_unix
    );
    if let Err(error) = std::fs::write(&state_out_path, state_json) {
        eprintln!("mesh nodes re-enroll-submit error: write state failed: {error}");
        return 2;
    }
    if let Some(activation_out_path) = resolve_activation_out_path(args) {
        let activation_json = format!(
            "{{\"kind\":\"mesh_reenroll_activation\",\"status\":\"active\",\"self_node_id\":\"{}\",\"activated_at_unix\":{},\"source\":\"re_enroll_submit\"}}",
            node_id, now_unix
        );
        if let Err(error) = std::fs::write(&activation_out_path, activation_json) {
            eprintln!("mesh nodes re-enroll-submit error: write activation failed: {error}");
            return 2;
        }
        println!(
            "re_enroll_submit=ok state_out={} activation_out={}",
            state_out_path, activation_out_path
        );
        return 0;
    }
    println!("re_enroll_submit=ok state_out={state_out_path}");
    0
}

fn resolve_identity_state_out_path(args: &[String]) -> Option<String> {
    extract_flag_value(args, "--state-out")
        .map(str::to_string)
        .or_else(|| {
            let config_path = extract_flag_value(args, "--config")?;
            let text = std::fs::read_to_string(config_path).ok()?;
            let raw = chimera_config::RawConfig::parse(&text).ok()?;
            raw.get("mesh.nodes.identity_state_path")
                .map(str::to_string)
        })
        .or_else(|| std::env::var("CHIMERA_MESH_IDENTITY_STATE_PATH").ok())
}

fn resolve_activation_out_path(args: &[String]) -> Option<String> {
    extract_flag_value(args, "--activation-out")
        .map(str::to_string)
        .or_else(|| {
            let config_path = extract_flag_value(args, "--config")?;
            let text = std::fs::read_to_string(config_path).ok()?;
            let raw = chimera_config::RawConfig::parse(&text).ok()?;
            raw.get("mesh.nodes.activation_log_path")
                .map(str::to_string)
        })
        .or_else(|| std::env::var("CHIMERA_MESH_ACTIVATION_LOG_PATH").ok())
}

fn build_runtime_from_inventory(
    inventory: &MeshNodesInventory,
    policy: &MeshNodesPolicy,
    operation: &str,
) -> Result<MeshNodeRuntime, i32> {
    let mut runtime = match MeshNodeRuntime::new(policy.clone()) {
        Ok(runtime) => runtime,
        Err(errors) => {
            eprintln!("mesh nodes {operation} error: {}", errors.join("; "));
            return Err(2);
        }
    };
    runtime.state.current_node = inventory.current_node.clone();
    runtime.state.pinned_node = inventory.pinned_node.clone();
    if let Some(enabled) = inventory.autoconnect_enabled {
        runtime.set_autoconnect(enabled);
    }
    Ok(runtime)
}

fn probe_all(args: &[String], inventory: &MeshNodesInventory) -> i32 {
    let json = args.iter().any(|v| v == "--json");
    let pq_strict = proof_pq_strict_enabled(args);
    let filter = match parse_filter(args) {
        Ok(filter) => filter,
        Err(error) => {
            if json {
                println!(
                    "{}",
                    render_nodes_json_error(
                        "mesh_nodes_probe_all",
                        "probe_filter",
                        "parse_filter",
                        &error
                    )
                );
                return 2;
            }
            eprintln!("mesh nodes probe error: invalid filter: {error}");
            return 2;
        }
    };
    let filtered_nodes = group_mesh_nodes_by_country(&inventory.nodes, &filter)
        .into_iter()
        .flat_map(|group| group.nodes.into_iter())
        .collect::<Vec<_>>();
    if filtered_nodes.is_empty() {
        if json {
            println!(
                "{}",
                render_nodes_json_error(
                    "mesh_nodes_probe_all",
                    "probe_input",
                    "inspect_inventory",
                    "no nodes available for probe after filter"
                )
            );
            return 2;
        }
        println!("probe=skipped reason=no_nodes_after_filter");
        return 0;
    }
    let namespace = extract_flag_value(args, "--namespace").unwrap_or("mesh-nodes");
    let node_name = extract_flag_value(args, "--self-node-id")
        .map(str::to_string)
        .or_else(|| inventory.self_node_id.as_ref().map(|id| id.0.clone()))
        .unwrap_or_else(|| "mesh-cli".to_string());
    let timeout_ms = extract_flag_value(args, "--probe-timeout-ms")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1200)
        .max(1);
    let proof_classic = extract_flag_value(args, "--proof-token-classic")
        .or_else(|| extract_flag_value(args, "--proof-token"))
        .map(str::to_string);
    let proof_pq = extract_flag_value(args, "--proof-token-pq")
        .or_else(|| extract_flag_value(args, "--proof-token"))
        .map(str::to_string);
    let proof_key_id = extract_flag_value(args, "--proof-key-id")
        .unwrap_or("mesh-shared-v1")
        .to_string();
    let proof_pq_key_id = extract_flag_value(args, "--proof-pq-key-id")
        .unwrap_or("mesh-pq-shared-v1")
        .to_string();
    if pq_strict
        && extract_flag_value(args, "--proof-token").is_some()
        && (extract_flag_value(args, "--proof-token-classic").is_none()
            || extract_flag_value(args, "--proof-token-pq").is_none())
    {
        if json {
            println!(
                "{}",
                render_nodes_json_error(
                    "mesh_nodes_probe_all",
                    "proof_policy",
                    "enforce_pq_strict",
                    "pq_strict mode forbids legacy --proof-token; use --proof-token-classic + --proof-token-pq"
                )
            );
            return 2;
        }
        eprintln!(
            "mesh nodes probe error: pq_strict mode forbids legacy --proof-token; use --proof-token-classic + --proof-token-pq"
        );
        return 2;
    }
    let mut runtime = match MeshRuntime::bootstrap(namespace, "cli-nodes-probe") {
        Ok(runtime) => runtime,
        Err(error) => {
            if json {
                println!(
                    "{}",
                    render_nodes_json_error(
                        "mesh_nodes_probe_all",
                        "runtime_bootstrap",
                        "bootstrap_runtime",
                        &error
                    )
                );
                return 2;
            }
            eprintln!("mesh nodes probe error: runtime bootstrap failed: {error}");
            return 2;
        }
    };
    let records = filtered_nodes
        .iter()
        .map(|node| MeshDiscoveryRecord {
            node_id: node.node_id.0.clone(),
            endpoint: node.endpoint.clone(),
            region: node.country.country_code.clone(),
            load_score: node.loss_pct.unwrap_or(0.0).round().clamp(0.0, 100.0) as u8,
            reliability_score: node
                .success_rate_1h
                .unwrap_or(100.0)
                .round()
                .clamp(0.0, 100.0) as u8,
        })
        .collect::<Vec<_>>();
    if let Err(error) = runtime.merge_discovery("mesh-nodes-inventory", &records) {
        if json {
            println!(
                "{}",
                render_nodes_json_error(
                    "mesh_nodes_probe_all",
                    "discovery_merge",
                    "merge_inventory",
                    &error
                )
            );
            return 2;
        }
        eprintln!("mesh nodes probe error: discovery merge failed: {error}");
        return 2;
    }
    let request = MeshJoinRequest {
        namespace: namespace.to_string(),
        node_name,
        invite_token: None,
    };
    let policy = match MeshPathPolicy::from_dps_payload(
        "allow=mesh;mesh_traffic_class=web_interactive;mesh_multipath_mode=standby_only;mesh_continuity_policy=allow_flow_drain;mesh_max_peers=3;mesh_min_reliability=1;mesh_max_load=100;mesh_connect_fallback_ports=443,8443",
    ) {
        Ok(policy) => policy,
        Err(error) => {
            if json {
                println!(
                    "{}",
                    render_nodes_json_error(
                        "mesh_nodes_probe_all",
                        "policy_parse",
                        "build_probe_policy",
                        &error
                    )
                );
                return 2;
            }
            eprintln!("mesh nodes probe error: policy parse failed: {error}");
            return 2;
        }
    };
    match runtime.connect_probe(&request, &policy, timeout_ms) {
        Ok(report) => {
            if report.success
                && let (Some(classic), Some(pq)) = (proof_classic.as_deref(), proof_pq.as_deref())
                && let Err(error) = verify_chimera_proof(
                    &report.connected_endpoint,
                    classic,
                    pq,
                    proof_key_id.as_str(),
                    proof_pq_key_id.as_str(),
                    timeout_ms,
                )
            {
                if json {
                    println!(
                        "{}",
                        render_nodes_json_error(
                            "mesh_nodes_probe_all",
                            "proof_verify",
                            "verify_chimera_proof",
                            &error
                        )
                    );
                    return 2;
                }
                eprintln!("mesh nodes probe error: proof verify failed: {error}");
                return 2;
            }
            if json {
                println!("{}", render_probe_all_json(&report));
            } else {
                println!(
                    "probe=applied success={} selected={} attempts={} connected_peer={} connected_endpoint={}",
                    report.success,
                    report.selected_peers.len(),
                    report.attempts.len(),
                    if report.connected_peer.is_empty() {
                        "none"
                    } else {
                        report.connected_peer.as_str()
                    },
                    if report.connected_endpoint.is_empty() {
                        "none"
                    } else {
                        report.connected_endpoint.as_str()
                    }
                );
            }
            if report.success { 0 } else { 1 }
        }
        Err(error) => {
            if json {
                println!(
                    "{}",
                    render_nodes_json_error(
                        "mesh_nodes_probe_all",
                        "connect_probe",
                        "run_probe",
                        &error
                    )
                );
                return 2;
            }
            eprintln!("mesh nodes probe error: {error}");
            2
        }
    }
}

fn auto_unblock(args: &[String], inventory: &MeshNodesInventory) -> i32 {
    let probe_args = args.to_vec();
    let pq_strict = proof_pq_strict_enabled(args);
    if pq_strict
        && probe_args.iter().any(|v| v == "--proof-token")
        && (!probe_args.iter().any(|v| v == "--proof-token-classic")
            || !probe_args.iter().any(|v| v == "--proof-token-pq"))
    {
        eprintln!(
            "mesh nodes auto-unblock error: pq_strict mode forbids legacy --proof-token; use --proof-token-classic + --proof-token-pq"
        );
        return 2;
    }
    if !(probe_args.iter().any(|v| v == "--proof-token-classic")
        || probe_args.iter().any(|v| v == "--proof-token"))
        || !(probe_args.iter().any(|v| v == "--proof-token-pq")
            || probe_args.iter().any(|v| v == "--proof-token"))
    {
        eprintln!(
            "mesh nodes auto-unblock error: --proof-token-classic and --proof-token-pq are required (or legacy --proof-token)"
        );
        return 2;
    }
    let code = probe_all(&probe_args, inventory);
    if code != 0 {
        return code;
    }
    let policy = MeshNodesPolicy::default();
    let mut runtime = match build_runtime_from_inventory(inventory, &policy, "auto-unblock") {
        Ok(runtime) => runtime,
        Err(code) => return code,
    };
    if let Some(best) = chimera_mesh::select_best_mesh_node(&inventory.nodes) {
        let decision = runtime.manual_connect(&inventory.nodes, &best.node_id, 0);
        if decision.allowed {
            if let Err(error) = persist_runtime_state(&probe_args, &runtime) {
                eprintln!("mesh nodes auto-unblock error: persist runtime state failed: {error}");
                return 1;
            }
            println!("auto_unblock=ok node_id={}", best.node_id);
            return 0;
        }
    }
    eprintln!("mesh nodes auto-unblock error: no eligible node after proof");
    2
}

fn guard_listen(args: &[String]) -> i32 {
    let bind = extract_flag_value(args, "--bind").unwrap_or("0.0.0.0:8443");
    let pq_strict = proof_pq_strict_enabled(args);
    let proof_key_id = extract_flag_value(args, "--proof-key-id")
        .unwrap_or("mesh-shared-v1")
        .to_string();
    let proof_pq_key_id = extract_flag_value(args, "--proof-pq-key-id")
        .unwrap_or("mesh-pq-shared-v1")
        .to_string();
    let token_classic = extract_flag_value(args, "--proof-token-classic")
        .or_else(|| extract_flag_value(args, "--proof-token"));
    let token_pq = extract_flag_value(args, "--proof-token-pq")
        .or_else(|| extract_flag_value(args, "--proof-token"));
    if pq_strict
        && extract_flag_value(args, "--proof-token").is_some()
        && (extract_flag_value(args, "--proof-token-classic").is_none()
            || extract_flag_value(args, "--proof-token-pq").is_none())
    {
        eprintln!(
            "mesh nodes guard-listen error: pq_strict mode forbids legacy --proof-token; use --proof-token-classic + --proof-token-pq"
        );
        return 2;
    }
    let (Some(token_classic), Some(token_pq)) = (token_classic, token_pq) else {
        eprintln!(
            "mesh nodes guard-listen error: --proof-token-classic and --proof-token-pq are required (or legacy --proof-token)"
        );
        return 2;
    };
    let once = args.iter().any(|v| v == "--once");
    let listener = match TcpListener::bind(bind) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("mesh nodes guard-listen error: bind failed: {error}");
            return 2;
        }
    };
    println!("guard_listen=ready bind={bind} once={once}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let ok = handle_guard_conn(
                    stream,
                    token_classic,
                    token_pq,
                    proof_key_id.as_str(),
                    proof_pq_key_id.as_str(),
                    3_000,
                );
                if ok {
                    println!("guard_listen=proof_ok");
                    if once {
                        return 0;
                    }
                }
            }
            Err(error) => {
                eprintln!("mesh nodes guard-listen error: accept failed: {error}");
                return 2;
            }
        }
    }
    0
}

fn handle_guard_conn(
    mut stream: TcpStream,
    token_classic: &str,
    token_pq: &str,
    expected_key_id: &str,
    expected_pq_key_id: &str,
    timeout_ms: u64,
) -> bool {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(timeout_ms.max(1))));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(timeout_ms.max(1))));
    let mut reader = BufReader::new(&mut stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return false;
    }
    let challenge: serde_json::Value = match serde_json::from_str(line.trim()) {
        Ok(value) => value,
        Err(_) => return false,
    };
    if verify_guard_challenge(
        &challenge,
        token_classic,
        token_pq,
        expected_key_id,
        expected_pq_key_id,
    )
    .is_err()
    {
        return false;
    }
    stream
        .write_all(b"{\"kind\":\"mesh_guard_ack_v1\",\"status\":\"ok\"}\n")
        .is_ok()
}

pub(crate) fn verify_chimera_proof(
    endpoint: &str,
    token_classic: &str,
    token_pq: &str,
    key_id: &str,
    pq_key_id: &str,
    timeout_ms: u64,
) -> Result<(), String> {
    let mut addrs = endpoint
        .to_socket_addrs()
        .map_err(|error| format!("resolve_error:{error}"))?;
    let addr = addrs
        .next()
        .ok_or_else(|| "resolve_error:no_socket_addrs".to_string())?;
    let timeout = Duration::from_millis(timeout_ms.max(1));
    let mut stream = TcpStream::connect_timeout(&addr, timeout)
        .map_err(|error| format!("connect_error:{error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("set_read_timeout_error:{error}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| format!("set_write_timeout_error:{error}"))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system_clock_error:{error}"))?
        .as_secs();
    let expires = now.saturating_add(30);
    let nonce = format!("n-{now}-{endpoint}");
    let message = guard_sign_message(&nonce, now, expires);
    let classic_sig = guard_mac(token_classic, "chimera-classic-v1", &message);
    let pq_sig = guard_mac(token_pq, "chimera-pq-v1", &message);
    let hello = format!(
        "{{\"kind\":\"mesh_guard_challenge_v1\",\"key_id\":\"{}\",\"pq_key_id\":\"{}\",\"classic_alg\":\"hmac-sha256-v1\",\"pq_alg\":\"hmac-sha256-v1-placeholder\",\"nonce\":\"{}\",\"issued_at_unix\":{},\"expires_at_unix\":{},\"classic_sig\":\"{}\",\"pq_sig\":\"{}\"}}\n",
        escape_json(key_id),
        escape_json(pq_key_id),
        escape_json(&nonce),
        now,
        expires,
        classic_sig,
        pq_sig
    );
    stream
        .write_all(hello.as_bytes())
        .map_err(|error| format!("write_error:{error}"))?;
    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    reader
        .read_line(&mut line)
        .map_err(|error| format!("read_error:{error}"))?;
    let ack: serde_json::Value =
        serde_json::from_str(line.trim()).map_err(|error| format!("invalid_ack_json:{error}"))?;
    if ack.get("kind").and_then(serde_json::Value::as_str) != Some("mesh_guard_ack_v1")
        || ack.get("status").and_then(serde_json::Value::as_str) != Some("ok")
    {
        return Err("invalid_proof_response".to_string());
    }
    Ok(())
}

fn guard_sign_message(nonce: &str, issued_at: u64, expires_at: u64) -> String {
    format!("nonce={nonce}\nissued_at_unix={issued_at}\nexpires_at_unix={expires_at}\n")
}

fn guard_mac(token: &str, domain: &str, message: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, token.as_bytes());
    let payload = format!("{domain}\n{message}");
    let sig = hmac::sign(&key, payload.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(sig.as_ref())
}

pub(crate) fn verify_guard_challenge(
    challenge: &serde_json::Value,
    token_classic: &str,
    token_pq: &str,
    expected_key_id: &str,
    expected_pq_key_id: &str,
) -> Result<(), String> {
    const MAX_CLOCK_SKEW_SEC: u64 = 120;
    let kind = challenge
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_kind".to_string())?;
    if kind != "mesh_guard_challenge_v1" {
        return Err("invalid_kind".to_string());
    }
    let classic_alg = challenge
        .get("classic_alg")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_classic_alg".to_string())?;
    if classic_alg != "hmac-sha256-v1" {
        return Err("unsupported_classic_alg".to_string());
    }
    let pq_alg = challenge
        .get("pq_alg")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_pq_alg".to_string())?;
    if pq_alg != "hmac-sha256-v1-placeholder" {
        return Err("unsupported_pq_alg".to_string());
    }
    let key_id = challenge
        .get("key_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_key_id".to_string())?;
    if key_id != expected_key_id {
        return Err("unexpected_key_id".to_string());
    }
    let pq_key_id = challenge
        .get("pq_key_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_pq_key_id".to_string())?;
    if pq_key_id != expected_pq_key_id {
        return Err("unexpected_pq_key_id".to_string());
    }
    let nonce = challenge
        .get("nonce")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_nonce".to_string())?;
    if nonce.trim().is_empty() {
        return Err("blank_nonce".to_string());
    }
    let issued_at = challenge
        .get("issued_at_unix")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "missing_issued_at".to_string())?;
    let expires_at = challenge
        .get("expires_at_unix")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "missing_expires_at".to_string())?;
    if expires_at <= issued_at {
        return Err("invalid_ttl_window".to_string());
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system_clock_error:{error}"))?
        .as_secs();
    if issued_at > now.saturating_add(MAX_CLOCK_SKEW_SEC) {
        return Err("issued_at_too_far_in_future".to_string());
    }
    if expires_at < now {
        return Err("challenge_expired".to_string());
    }
    remember_guard_nonce(nonce, expires_at)?;
    let classic_sig = challenge
        .get("classic_sig")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_classic_sig".to_string())?;
    let pq_sig = challenge
        .get("pq_sig")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_pq_sig".to_string())?;
    let message = guard_sign_message(nonce, issued_at, expires_at);
    let expected_classic = guard_mac(token_classic, "chimera-classic-v1", &message);
    let expected_pq = guard_mac(token_pq, "chimera-pq-v1", &message);
    if classic_sig != expected_classic {
        return Err("classic_sig_mismatch".to_string());
    }
    if pq_sig != expected_pq {
        return Err("pq_sig_mismatch".to_string());
    }
    Ok(())
}

fn remember_guard_nonce(nonce: &str, expires_at: u64) -> Result<(), String> {
    static CACHE: OnceLock<Mutex<std::collections::BTreeMap<String, u64>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::BTreeMap::new()));
    let mut guard = cache
        .lock()
        .map_err(|_| "guard_nonce_cache_lock_poisoned".to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system_clock_error:{error}"))?
        .as_secs();
    guard.retain(|_, expiry| *expiry >= now);
    if guard.contains_key(nonce) {
        return Err("guard_replay_nonce".to_string());
    }
    if guard.len() >= 4096
        && let Some(first) = guard.keys().next().cloned()
    {
        guard.remove(&first);
    }
    guard.insert(nonce.to_string(), expires_at);
    Ok(())
}

fn state_cmd(args: &[String], inventory: &MeshNodesInventory) -> i32 {
    let json = args.iter().any(|v| v == "--json");
    match args.first().map(String::as_str) {
        Some("clear") => {
            let Some(path) = resolve_runtime_state_out_path(args) else {
                if json {
                    println!(
                        "{}",
                        render_nodes_json_error(
                            "mesh_nodes_runtime_state",
                            "state_path",
                            "resolve_runtime_state_path",
                            "runtime-state path is not configured"
                        )
                    );
                    return 2;
                }
                eprintln!("mesh nodes state error: runtime-state path is not configured");
                return 2;
            };
            match std::fs::remove_file(&path) {
                Ok(_) => {
                    println!("state=cleared path={path}");
                    0
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    println!("state=already_missing path={path}");
                    0
                }
                Err(error) => {
                    if json {
                        println!(
                            "{}",
                            render_nodes_json_error(
                                "mesh_nodes_runtime_state",
                                "state_clear",
                                "remove_runtime_state_file",
                                &error.to_string()
                            )
                        );
                        return 2;
                    }
                    eprintln!("mesh nodes state error: clear failed: {error}");
                    2
                }
            }
        }
        Some(other) => {
            if json {
                println!(
                    "{}",
                    render_nodes_json_error(
                        "mesh_nodes_runtime_state",
                        "options_parse",
                        "parse_state_subcommand",
                        &format!("unknown subcommand '{other}'")
                    )
                );
                return 2;
            }
            eprintln!("mesh nodes state error: unknown subcommand '{other}'");
            2
        }
        None => {
            if json {
                println!("{}", render_state_view_json(inventory));
            } else {
                println!(
                    "current={}\npinned={}\nautoconnect={}\nrestricted={}\nreason={}",
                    inventory
                        .current_node
                        .as_ref()
                        .map(|v| v.0.as_str())
                        .unwrap_or("none"),
                    inventory
                        .pinned_node
                        .as_ref()
                        .map(|v| v.0.as_str())
                        .unwrap_or("none"),
                    match inventory.autoconnect_enabled {
                        Some(true) => "on",
                        Some(false) => "off",
                        None => "default",
                    },
                    if inventory.restricted_reason.is_some() {
                        "yes"
                    } else {
                        "no"
                    },
                    inventory.restricted_reason.as_deref().unwrap_or("none"),
                );
                print_state_next_step_hint();
            }
            0
        }
    }
}

fn advertise_node(args: &[String], inventory: &MeshNodesInventory) -> i32 {
    let Some(out_path) = extract_flag_value(args, "--out") else {
        eprintln!("mesh nodes advertise error: --out is required");
        return 2;
    };
    let Some(discovery_pubkey_out) = extract_flag_value(args, "--pubkey-out") else {
        eprintln!("mesh nodes advertise error: --pubkey-out is required");
        return 2;
    };
    let node_id = match resolve_advertise_node_id(args, inventory) {
        Ok(node_id) => node_id,
        Err(error) => {
            eprintln!("mesh nodes advertise error: {error}");
            return 2;
        }
    };
    let endpoint = match resolve_advertise_endpoint(args, inventory) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            eprintln!("mesh nodes advertise error: {error}");
            return 2;
        }
    };
    let country_code = extract_flag_value(args, "--country-code")
        .map(str::to_string)
        .or_else(|| {
            inventory
                .nodes
                .iter()
                .find(|node| node.node_id.0 == node_id)
                .map(|node| node.country.country_code.clone())
        })
        .unwrap_or_else(|| "ZZ".to_string());
    let country_name = extract_flag_value(args, "--country-name")
        .map(str::to_string)
        .or_else(|| {
            inventory
                .nodes
                .iter()
                .find(|node| node.node_id.0 == node_id)
                .map(|node| node.country.country_name.clone())
        })
        .unwrap_or_else(|| "Unknown".to_string());
    let region = extract_flag_value(args, "--region")
        .map(str::to_string)
        .or_else(|| {
            inventory
                .nodes
                .iter()
                .find(|node| node.node_id.0 == node_id)
                .map(|node| node.country.country_code.to_ascii_lowercase())
        })
        .unwrap_or_else(|| "global".to_string());
    let topic = extract_flag_value(args, "--topic")
        .map(str::to_string)
        .unwrap_or_else(|| "mesh-node".to_string());
    let ttl_sec = extract_flag_value(args, "--ttl-sec")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(900)
        .max(1);
    let discovery_keypair_path = resolve_discovery_keypair_path(args);
    let keypair = match load_or_create_discovery_signing_key(&discovery_keypair_path) {
        Ok(keypair) => keypair,
        Err(error) => {
            eprintln!("mesh nodes advertise error: {error}");
            return 2;
        }
    };
    let pubkey_b64 =
        base64::engine::general_purpose::STANDARD.encode(keypair.public_key().as_ref());
    let now_unix = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_secs(),
        Err(error) => {
            eprintln!("mesh nodes advertise error: system clock error: {error}");
            return 2;
        }
    };
    let expires_at_unix = now_unix.saturating_add(ttl_sec);
    let nonce = format!("advertise-{node_id}-{now_unix}");
    let node = serde_json::json!({
        "node_id": &node_id,
        "endpoint": &endpoint,
        "country_code": &country_code,
        "country_name": &country_name,
        "status": "healthy",
        "country_source": "operator_override",
        "country_confidence": "high",
        "country_updated_at": "auto",
        "country_ttl_sec": ttl_sec,
        "country_conflict": false,
        "country_conflict_reason": null,
        "region": &region,
        "topic": &topic,
        "invite_token": selected_node_invite_token(inventory),
        "freshness_unix": now_unix,
        "ttl_sec": ttl_sec,
        "capabilities": ["node", "gateway", "mesh"],
    });
    let nodes = serde_json::json!([node]);
    let message = match super::nodes_inventory::build_discovery_signature_message(
        1,
        now_unix,
        expires_at_unix,
        &nonce,
        &nodes,
    ) {
        Ok(message) => message,
        Err(error) => {
            eprintln!("mesh nodes advertise error: {error}");
            return 2;
        }
    };
    let signature = keypair.sign(&message);
    let envelope = serde_json::json!({
        "contract_version": 1,
        "issued_at_unix": now_unix,
        "expires_at_unix": expires_at_unix,
        "key_id": "default",
        "nonce": nonce,
        "nodes": nodes,
        "signature": base64::engine::general_purpose::STANDARD.encode(signature.as_ref()),
    });
    if let Err(error) = write_discovery_artifacts(
        &out_path,
        &discovery_pubkey_out,
        &pubkey_b64,
        &envelope.to_string(),
    ) {
        eprintln!("mesh nodes advertise error: {error}");
        return 2;
    }
    println!("mesh_nodes_advertise=ok out={out_path} pubkey_out={discovery_pubkey_out}");
    println!("mesh_nodes_advertise_endpoint={endpoint}");
    println!("mesh_nodes_advertise_node_id={node_id}");
    0
}

fn resolve_advertise_node_id(args: &[String], inventory: &MeshNodesInventory) -> Result<String, String> {
    if let Some(id) = extract_flag_value(args, "--node-id") {
        let id = id.trim();
        if id.is_empty() {
            return Err("mesh nodes advertise error: --node-id is empty".to_string());
        }
        MeshNodeId::new(id).validate()?;
        return Ok(id.to_string());
    }
    if let Some(id) = inventory.self_node_id.as_ref() {
        return Ok(id.0.clone());
    }
    if let Ok(host) = std::env::var("HOSTNAME") {
        let host = host.trim();
        if !host.is_empty() {
            let sanitized = sanitize_node_id(host);
            if !sanitized.is_empty() {
                return Ok(sanitized);
            }
        }
    }
    Err("mesh nodes advertise error: cannot resolve node id (use --node-id or mesh.nodes.self_node_id)".to_string())
}

fn sanitize_node_id(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

fn resolve_advertise_endpoint(args: &[String], inventory: &MeshNodesInventory) -> Result<String, String> {
    if let Some(endpoint) = extract_flag_value(args, "--endpoint") {
        let endpoint = endpoint.trim();
        if endpoint.is_empty() {
            return Err("mesh nodes advertise error: --endpoint is empty".to_string());
        }
        return Ok(endpoint.to_string());
    }
    if let Some(endpoint) = selected_node_endpoint(inventory) {
        let endpoint = endpoint.trim();
        if !endpoint.is_empty() {
            return Ok(endpoint.to_string());
        }
    }
    let state_path = extract_flag_value(args, "--state-file")
        .map(str::to_string)
        .or_else(|| std::env::var("CHIMERA_MESH_PEER_EGRESS_STATE_PATH").ok());
    if let Some(state_path) = state_path {
        if let Ok(text) = std::fs::read_to_string(state_path) {
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("resolved_peer_listen=") {
                    let endpoint = rest.trim();
                    if !endpoint.is_empty() {
                        return Ok(endpoint.to_string());
                    }
                }
            }
        }
    }
    Err("mesh nodes advertise error: cannot resolve endpoint (use --endpoint or current selected endpoint)".to_string())
}

fn resolve_discovery_keypair_path(args: &[String]) -> PathBuf {
    if let Some(path) = extract_flag_value(args, "--keypair-path") {
        return PathBuf::from(path);
    }
    if let Ok(path) = std::env::var("CHIMERA_MESH_DISCOVERY_KEYPAIR_PATH") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|home| format!("{home}/.config"))
                .unwrap_or_else(|| ".config".to_string())
        });
    PathBuf::from(base).join("chimera/discovery_signing.keypair")
}

fn load_or_create_discovery_signing_key(
    path: &Path,
) -> Result<Ed25519KeyPair, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("create key dir failed: {error}"))?;
    }
    if path.exists() {
        let raw = std::fs::read_to_string(path)
            .map_err(|error| format!("read discovery keypair failed: {error}"))?;
        let pkcs8_b64 = raw
            .lines()
            .find_map(|line| line.strip_prefix("pkcs8_base64="))
            .ok_or_else(|| "discovery keypair file missing pkcs8_base64".to_string())?;
        let pkcs8 = base64::engine::general_purpose::STANDARD
            .decode(pkcs8_b64.trim())
            .map_err(|error| format!("decode discovery keypair failed: {error}"))?;
        return Ed25519KeyPair::from_pkcs8(&pkcs8)
            .map_err(|_| "parse discovery keypair failed".to_string());
    }
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
        .map_err(|_| "discovery keypair generation failed".to_string())?;
    let keypair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())
        .map_err(|_| "discovery keypair parse failed".to_string())?;
    let material = format!(
        "kind=mesh_discovery_keypair\nalgorithm=ed25519\npkcs8_base64={}\n",
        base64::engine::general_purpose::STANDARD.encode(pkcs8.as_ref())
    );
    std::fs::write(path, material)
        .map_err(|error| format!("write discovery keypair failed: {error}"))?;
    Ok(keypair)
}

fn write_discovery_artifacts(
    out_path: &str,
    pubkey_out_path: &str,
    pubkey_b64: &str,
    json: &str,
) -> Result<(), String> {
    if let Some(parent) = Path::new(out_path).parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("create discovery dir failed: {error}"))?;
    }
    if let Some(parent) = Path::new(pubkey_out_path).parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("create pubkey dir failed: {error}"))?;
    }
    std::fs::write(out_path, json).map_err(|error| format!("write discovery out failed: {error}"))?;
    std::fs::write(pubkey_out_path, format!("{pubkey_b64}\n"))
        .map_err(|error| format!("write pubkey out failed: {error}"))?;
    Ok(())
}

fn print_list_next_step_hint() {
    // List output already contains a clean "Следующая команда" block with examples.
}

fn print_connect_next_step_hint() {
    // No next-step hint here: reconnect list is redundant right after successful connect.
}

fn print_pin_next_step_hint() {
    println!("next: chimera mesh nodes state           # verify pinned node");
}

fn print_state_next_step_hint() {
    println!("next: chimera mesh nodes select          # choose and connect a node");
}

pub(crate) fn render_nodes_json_error(
    kind: &str,
    stage: &str,
    action: &str,
    message: &str,
) -> String {
    const CONTRACT_FAMILY: &str = "mesh_nodes_contract";
    const CONTRACT_VERSION: u64 = 1;
    let error_signature = format!("{stage}:{action}");
    let error_route_key = format!("{kind}:{action}");
    format!(
        "{{\"kind\":\"{}\",\"status\":\"error\",\"contract_family\":\"{}\",\"contract_version\":{},\"network_state\":\"not_modified\",\"stage\":\"{}\",\"action\":\"{}\",\"message\":\"{}\",\"error_signature\":\"{}\",\"error_route_key\":\"{}\"}}",
        escape_json(kind),
        CONTRACT_FAMILY,
        CONTRACT_VERSION,
        escape_json(stage),
        escape_json(action),
        escape_json(message),
        escape_json(&error_signature),
        escape_json(&error_route_key)
    )
}

pub(crate) fn render_probe_all_json(report: &MeshConnectProbeReport) -> String {
    const CONTRACT_VERSION: u64 = 1;
    let attempts = report
        .attempts
        .iter()
        .map(|a| {
            format!(
                "{{\"peer_id\":\"{}\",\"endpoint\":\"{}\",\"success\":{},\"error\":\"{}\"}}",
                escape_json(&a.peer_id),
                escape_json(&a.endpoint),
                if a.success { "true" } else { "false" },
                escape_json(&a.error)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"kind\":\"mesh_nodes_probe_all\",\"status\":\"ok\",\"contract_version\":{},\"network_state\":\"not_modified\",\"success\":{},\"selected\":{},\"attempts_count\":{},\"connected_peer\":\"{}\",\"connected_endpoint\":\"{}\",\"attempts\":[{}]}}",
        CONTRACT_VERSION,
        if report.success { "true" } else { "false" },
        report.selected_peers.len(),
        report.attempts.len(),
        escape_json(if report.connected_peer.is_empty() {
            "none"
        } else {
            &report.connected_peer
        }),
        escape_json(if report.connected_endpoint.is_empty() {
            "none"
        } else {
            &report.connected_endpoint
        }),
        attempts
    )
}

pub(crate) fn render_state_view_json(inventory: &MeshNodesInventory) -> String {
    const CONTRACT_VERSION: u64 = 1;
    format!(
        "{{\"kind\":\"mesh_nodes_runtime_state_view\",\"status\":\"ok\",\"contract_version\":{},\"network_state\":\"not_modified\",\"current_node_id\":\"{}\",\"pinned_node_id\":\"{}\",\"autoconnect\":{},\"restricted_mode\":{},\"restricted_reason\":\"{}\"}}",
        CONTRACT_VERSION,
        inventory
            .current_node
            .as_ref()
            .map(|v| v.0.as_str())
            .unwrap_or(""),
        inventory
            .pinned_node
            .as_ref()
            .map(|v| v.0.as_str())
            .unwrap_or(""),
        match inventory.autoconnect_enabled {
            Some(true) => "true",
            Some(false) => "false",
            None => "null",
        },
        if inventory.restricted_reason.is_some() {
            "true"
        } else {
            "false"
        },
        escape_json(inventory.restricted_reason.as_deref().unwrap_or(""))
    )
}

fn persist_runtime_state(args: &[String], runtime: &MeshNodeRuntime) -> Result<(), String> {
    let Some(path) = resolve_runtime_state_out_path(args) else {
        return Ok(());
    };
    let current_node = runtime
        .state
        .current_node
        .as_ref()
        .map(|id| id.0.as_str())
        .unwrap_or("");
    let pinned_node = runtime
        .state
        .pinned_node
        .as_ref()
        .map(|id| id.0.as_str())
        .unwrap_or("");
    let json = format!(
        "{{\"kind\":\"mesh_nodes_runtime_state\",\"current_node_id\":\"{}\",\"pinned_node_id\":\"{}\",\"autoconnect\":{}}}",
        current_node,
        pinned_node,
        if runtime.state.autoconnect_enabled {
            "true"
        } else {
            "false"
        }
    );
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, json).map_err(|error| error.to_string())
}

fn resolve_runtime_state_out_path(args: &[String]) -> Option<String> {
    Some(
        extract_flag_value(args, "--runtime-state")
            .map(str::to_string)
            .or_else(|| {
                let config_path = extract_flag_value(args, "--config")?;
                let text = std::fs::read_to_string(config_path).ok()?;
                let raw = chimera_config::RawConfig::parse(&text).ok()?;
                raw.get("mesh.nodes.runtime_state_path").map(str::to_string)
            })
            .or_else(|| std::env::var("CHIMERA_MESH_NODES_RUNTIME_STATE_PATH").ok())
            .unwrap_or_else(default_runtime_state_path),
    )
}

fn escape_json(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub(crate) fn proof_pq_strict_enabled(args: &[String]) -> bool {
    if args.iter().any(|v| v == "--no-pq-strict") {
        return false;
    }
    if args.iter().any(|v| v == "--pq-strict") {
        return true;
    }
    match std::env::var("CHIMERA_MESH_PROOF_PQ_STRICT") {
        Ok(value) => {
            let v = value.trim().to_ascii_lowercase();
            !(v.is_empty() || v == "0" || v == "false" || v == "off" || v == "no")
        }
        Err(_) => true,
    }
}

fn parse_filter(args: &[String]) -> Result<MeshNodeListFilter, String> {
    let mut filter = MeshNodeListFilter::default();
    if let Some(countries) = extract_flag_value(args, "--country") {
        filter.countries = countries
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_uppercase)
            .collect();
    }
    if let Some(statuses) = extract_flag_value(args, "--status") {
        for status in statuses
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            filter.statuses.insert(MeshNodeStatus::parse(status)?);
        }
    }
    filter.available_only = args.iter().any(|arg| arg == "--available-only");
    filter.search = extract_flag_value(args, "--search").map(str::to_string);
    Ok(filter)
}
