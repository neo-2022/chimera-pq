use chimera_mesh::{
    MeshNodeId, MeshNodesPolicy, build_mesh_node_explain, render_mesh_node_explain,
};

use crate::mesh_cli::nodes_inventory::{MeshNodesInventory, extract_flag_value};
use crate::mesh_cli::nodes_selection::{choose_node_id, resolve_node_id_selector};

use super::runtime::build_runtime_from_inventory;
use super::state::persist_runtime_state;

pub(super) fn explain_node(
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

pub(super) fn connect_node(
    args: &[String],
    inventory: &MeshNodesInventory,
    policy: &MeshNodesPolicy,
) -> i32 {
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

pub(super) fn select_node(
    args: &[String],
    inventory: &MeshNodesInventory,
    policy: &MeshNodesPolicy,
) -> i32 {
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

fn selected_node_for_args<'a>(
    args: &[String],
    inventory: &'a MeshNodesInventory,
) -> Result<Option<&'a chimera_mesh::MeshNode>, String> {
    if crate::mesh_cli::nodes_selection::has_direct_selector(args) {
        let id = resolve_node_id_selector(args, inventory)?;
        return Ok(inventory.nodes.iter().find(|node| node.node_id.0 == id));
    }
    let selected_id = inventory
        .current_node
        .as_ref()
        .or(inventory.pinned_node.as_ref());
    Ok(selected_id.and_then(|id| inventory.nodes.iter().find(|node| node.node_id == *id)))
}

pub(crate) fn node_update_bootstrap_url_for_args<'a>(
    args: &[String],
    inventory: &'a MeshNodesInventory,
) -> Result<Option<&'a str>, String> {
    Ok(selected_node_for_args(args, inventory)?
        .and_then(|node| node.update_bootstrap_url.as_deref()))
}

fn render_peer_spec(node: &chimera_mesh::MeshNode) -> String {
    let region = node.country.country_code.to_ascii_lowercase();
    let load_score = node.loss_pct.unwrap_or(0.0).round().clamp(0.0, 100.0) as u8;
    let reliability_score = node
        .success_rate_1h
        .unwrap_or(100.0)
        .round()
        .clamp(0.0, 100.0) as u8;
    format!(
        "{}@{}@{}@{}@{}",
        node.node_id, node.endpoint, region, load_score, reliability_score
    )
}

pub(crate) fn selected_node_peer_spec_for_args(
    args: &[String],
    inventory: &MeshNodesInventory,
) -> Result<Option<String>, String> {
    Ok(selected_node_for_args(args, inventory)?.map(render_peer_spec))
}

pub(super) fn selected_endpoint(_args: &[String], inventory: &MeshNodesInventory) -> i32 {
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

pub(super) fn selected_update_bootstrap_url(
    args: &[String],
    inventory: &MeshNodesInventory,
) -> i32 {
    if let Some(reason) = inventory.restricted_reason.as_deref() {
        eprintln!("mesh nodes selected-update-bootstrap-url error: restricted mode ({reason})");
        return 2;
    }
    match node_update_bootstrap_url_for_args(args, inventory) {
        Err(error) => {
            eprintln!("mesh nodes selected-update-bootstrap-url error: {error}");
            2
        }
        Ok(Some(url)) => {
            println!("{url}");
            0
        }
        Ok(None) => {
            eprintln!(
                "mesh nodes selected-update-bootstrap-url error: no selected node update_bootstrap_url"
            );
            2
        }
    }
}

pub(super) fn selected_invite_token(_args: &[String], inventory: &MeshNodesInventory) -> i32 {
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

pub(super) fn selected_peer_spec(args: &[String], inventory: &MeshNodesInventory) -> i32 {
    if let Some(reason) = inventory.restricted_reason.as_deref() {
        eprintln!("mesh nodes selected-peer-spec error: restricted mode ({reason})");
        return 2;
    }
    match selected_node_peer_spec_for_args(args, inventory) {
        Err(error) => {
            eprintln!("mesh nodes selected-peer-spec error: {error}");
            2
        }
        Ok(Some(peer_spec)) => {
            println!("{peer_spec}");
            0
        }
        Ok(None) => {
            eprintln!("mesh nodes selected-peer-spec error: no selected node");
            2
        }
    }
}

pub(super) fn pin_node(
    args: &[String],
    inventory: &MeshNodesInventory,
    policy: &MeshNodesPolicy,
) -> i32 {
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

pub(super) fn unpin_node(
    args: &[String],
    inventory: &MeshNodesInventory,
    policy: &MeshNodesPolicy,
) -> i32 {
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

pub(super) fn autoconnect(
    args: &[String],
    inventory: &MeshNodesInventory,
    policy: &MeshNodesPolicy,
) -> i32 {
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
pub(super) fn print_list_next_step_hint() {
    // List output already contains a clean "Следующая команда" block with examples.
}

fn print_connect_next_step_hint() {
    // No next-step hint here: reconnect list is redundant right after successful connect.
}

fn print_pin_next_step_hint() {
    println!("next: chimera mesh nodes state           # verify pinned node");
}

pub(super) fn print_state_next_step_hint() {
    println!("next: chimera mesh nodes select          # choose and connect a node");
}
