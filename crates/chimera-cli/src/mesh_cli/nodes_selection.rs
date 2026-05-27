use std::io::{self, BufRead, IsTerminal, Write};
use std::process::Command;

use chimera_mesh::{MeshNode, MeshNodeListFilter, select_best_mesh_node};

use super::nodes_inventory::MeshNodesInventory;
use super::nodes_render::{fmt_ms, fmt_pct};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MeshNodeSelectionEntry {
    pub index: usize,
    pub node_id: String,
    pub label: String,
    pub is_best: bool,
}

pub(crate) fn resolve_node_id_selector(
    args: &[String],
    inventory: &MeshNodesInventory,
) -> Result<String, String> {
    if let Some(id) = super::nodes_inventory::extract_flag_value(args, "--id") {
        return Ok(id.to_string());
    }
    let selector = extract_positional_selector(args)
        .ok_or_else(|| "--id <node_id> or positional <index|node_id> is required".to_string())?;
    if let Ok(index) = selector.parse::<usize>() {
        if index == 0 {
            return Err("index must start from 1".to_string());
        }
        let filter = parse_filter(args)?;
        let ordered = ordered_nodes(inventory, &filter);
        let Some(node) = ordered.get(index - 1) else {
            return Err(format!(
                "index {index} is out of range (available: 1..={})",
                ordered.len()
            ));
        };
        return Ok(node.node_id.0.clone());
    }
    Ok(selector.to_string())
}

pub(crate) fn choose_node_id(
    args: &[String],
    inventory: &MeshNodesInventory,
) -> Result<String, String> {
    if has_direct_selector(args) {
        return resolve_node_id_selector(args, inventory);
    }
    let filter = parse_filter(args)?;
    let entries = build_selection_entries(inventory, &filter);
    if entries.is_empty() {
        return Err("no mesh nodes available for selection".to_string());
    }
    match choose_node_id_gui(&entries)? {
        Some(node_id) => return Ok(node_id),
        None => {}
    }
    choose_node_id_tty(&entries)
}

pub(crate) fn build_selection_entries(
    inventory: &MeshNodesInventory,
    filter: &MeshNodeListFilter,
) -> Vec<MeshNodeSelectionEntry> {
    let ordered = ordered_nodes(inventory, filter);
    let best_node_id = select_best_mesh_node(&ordered).map(|node| node.node_id.0.clone());
    ordered
        .into_iter()
        .enumerate()
        .map(|(index, node)| MeshNodeSelectionEntry {
            index: index + 1,
            node_id: node.node_id.0.clone(),
            label: build_selection_label(&node, inventory, best_node_id.as_deref()),
            is_best: best_node_id.as_deref() == Some(node.node_id.0.as_str()),
        })
        .collect()
}

pub(crate) fn render_selection_prompt(entries: &[MeshNodeSelectionEntry]) -> String {
    let mut out = String::new();
    out.push_str("Доступные узлы CHIMERA:\n");
    out.push_str("------------------------------------------------------------\n");
    for entry in entries {
        out.push_str(&format!("{:>2}. {}\n", entry.index, entry.label));
    }
    out.push_str("------------------------------------------------------------\n");
    out.push_str("Выберите узел: номер, node_id или Enter для лучшего узла\n");
    out
}

fn choose_node_id_gui(entries: &[MeshNodeSelectionEntry]) -> Result<Option<String>, String> {
    if !display_available() {
        return Ok(None);
    }
    if command_exists("zenity") {
        return Ok(Some(choose_node_id_with_zenity(entries)?));
    }
    if command_exists("kdialog") {
        return Ok(Some(choose_node_id_with_kdialog(entries)?));
    }
    if command_exists("yad") {
        return Ok(Some(choose_node_id_with_yad(entries)?));
    }
    Ok(None)
}

fn choose_node_id_with_zenity(entries: &[MeshNodeSelectionEntry]) -> Result<String, String> {
    let mut cmd = Command::new("zenity");
    cmd.arg("--list")
        .arg("--title=CHIMERA Mesh Nodes")
        .arg("--text=Выберите узел для подключения")
        .arg("--column=Node ID")
        .arg("--column=Details")
        .arg("--print-column=1")
        .arg("--separator=|")
        .arg("--height=420")
        .arg("--width=900");
    for entry in entries {
        cmd.arg(&entry.node_id).arg(&entry.label);
    }
    read_gui_selection(cmd)
}

fn choose_node_id_with_yad(entries: &[MeshNodeSelectionEntry]) -> Result<String, String> {
    let mut cmd = Command::new("yad");
    cmd.arg("--list")
        .arg("--title=CHIMERA Mesh Nodes")
        .arg("--text=Выберите узел для подключения")
        .arg("--column=Node ID")
        .arg("--column=Details")
        .arg("--print-column=1")
        .arg("--separator=|")
        .arg("--height=420")
        .arg("--width=900")
        .arg("--no-headers");
    for entry in entries {
        cmd.arg(&entry.node_id).arg(&entry.label);
    }
    read_gui_selection(cmd)
}

fn choose_node_id_with_kdialog(entries: &[MeshNodeSelectionEntry]) -> Result<String, String> {
    let mut cmd = Command::new("kdialog");
    cmd.arg("--menu").arg("CHIMERA Mesh Nodes");
    for entry in entries {
        cmd.arg(&entry.node_id).arg(&entry.label);
    }
    read_gui_selection(cmd)
}

fn read_gui_selection(mut cmd: Command) -> Result<String, String> {
    let output = cmd.output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("selection cancelled".to_string());
    }
    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selected.is_empty() {
        Err("selection cancelled".to_string())
    } else {
        Ok(selected)
    }
}

fn choose_node_id_tty(entries: &[MeshNodeSelectionEntry]) -> Result<String, String> {
    if !io::stdin().is_terminal() {
        return Err(
            "interactive selection requires a terminal or GUI; use --id <node_id>".to_string(),
        );
    }
    print!("{}", render_selection_prompt(entries));
    io::stdout().flush().map_err(|error| error.to_string())?;
    let mut line = String::new();
    io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|error| error.to_string())?;
    let value = line.trim();
    if value.is_empty() {
        return select_best_entry(entries)
            .map(|entry| entry.node_id.clone())
            .ok_or_else(|| "no nodes available for best selection".to_string());
    }
    if let Ok(index) = value.parse::<usize>() {
        if index == 0 {
            return Err("index must start from 1".to_string());
        }
        let Some(entry) = entries.get(index - 1) else {
            return Err(format!(
                "index {index} is out of range (available: 1..={})",
                entries.len()
            ));
        };
        return Ok(entry.node_id.clone());
    }
    if entries.iter().any(|entry| entry.node_id == value) {
        return Ok(value.to_string());
    }
    Err("unknown node_id or index".to_string())
}

fn select_best_entry<'a>(
    entries: &'a [MeshNodeSelectionEntry],
) -> Option<&'a MeshNodeSelectionEntry> {
    entries
        .iter()
        .find(|entry| entry.is_best)
        .or_else(|| entries.first())
}

fn build_selection_label(
    node: &MeshNode,
    inventory: &MeshNodesInventory,
    best_node_id: Option<&str>,
) -> String {
    let mut markers = Vec::new();
    if inventory.current_node.as_ref() == Some(&node.node_id) {
        markers.push("текущий");
    }
    if inventory.pinned_node.as_ref() == Some(&node.node_id) {
        markers.push("закреплен");
    }
    if best_node_id == Some(node.node_id.0.as_str()) {
        markers.push("лучший");
    }
    let markers = if markers.is_empty() {
        String::new()
    } else {
        format!(" [{}]", markers.join(", "))
    };
    format!(
        "{} | {} ({}) | {} | score {:.2} | latency {} | loss {} | endpoint {}{}",
        node.node_id,
        node.country.country_name,
        node.country.country_code,
        node.status.as_str(),
        node.score,
        fmt_ms(node.latency_ms),
        fmt_pct(node.loss_pct),
        node.endpoint,
        markers
    )
}

fn ordered_nodes(inventory: &MeshNodesInventory, filter: &MeshNodeListFilter) -> Vec<MeshNode> {
    chimera_mesh::group_mesh_nodes_by_country(&inventory.nodes, filter)
        .into_iter()
        .flat_map(|group| group.nodes.into_iter())
        .collect()
}

fn has_direct_selector(args: &[String]) -> bool {
    super::nodes_inventory::extract_flag_value(args, "--id").is_some()
        || extract_positional_selector(args).is_some()
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {command} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn display_available() -> bool {
    std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

fn parse_filter(args: &[String]) -> Result<MeshNodeListFilter, String> {
    let mut filter = MeshNodeListFilter::default();
    if let Some(countries) = super::nodes_inventory::extract_flag_value(args, "--country") {
        filter.countries = countries
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_uppercase)
            .collect();
    }
    if let Some(statuses) = super::nodes_inventory::extract_flag_value(args, "--status") {
        for status in statuses
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            filter
                .statuses
                .insert(chimera_mesh::MeshNodeStatus::parse(status)?);
        }
    }
    filter.available_only = args.iter().any(|arg| arg == "--available-only");
    filter.search =
        super::nodes_inventory::extract_flag_value(args, "--search").map(str::to_string);
    Ok(filter)
}

fn extract_positional_selector(args: &[String]) -> Option<String> {
    let mut i = 0usize;
    while i < args.len() {
        let arg = &args[i];
        if arg.starts_with('-') {
            if flag_takes_value(arg.as_str()) {
                i = i.saturating_add(2);
                continue;
            }
            i += 1;
            continue;
        }
        return Some(arg.clone());
    }
    None
}

fn flag_takes_value(flag: &str) -> bool {
    matches!(
        flag,
        "--config"
            | "--self-node-id"
            | "--runtime-state"
            | "--namespace"
            | "--proof-token"
            | "--proof-token-classic"
            | "--proof-token-pq"
            | "--proof-key-id"
            | "--proof-pq-key-id"
            | "--bind"
            | "--discovery-url"
            | "--probe-timeout-ms"
            | "--node"
            | "--country"
            | "--status"
            | "--search"
            | "--id"
            | "--new-node-id"
            | "--request"
            | "--out"
            | "--key-out"
            | "--register"
            | "--key"
            | "--state-out"
            | "--activation-out"
            | "--activation-log"
            | "--activation-file"
            | "--activation-path"
            | "--activation-dir"
            | "--activation-json"
            | "--activation"
            | "--probe-timeout"
            | "--probe-url"
            | "--probe-host"
            | "--probe-port"
            | "--probe-target"
            | "--probe-id"
            | "--probe-node"
            | "--probe-namespace"
            | "--probe-key"
            | "--probe-output"
            | "--probe-state"
            | "--probe-register"
            | "--probe-request"
            | "--probe-activation"
            | "--probe-filter"
            | "--probe-search"
            | "--probe-country"
            | "--probe-status"
    )
}
