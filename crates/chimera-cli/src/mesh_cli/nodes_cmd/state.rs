use chimera_mesh::MeshNodeRuntime;

use crate::mesh_cli::nodes_inventory::{
    MeshNodesInventory, default_runtime_state_path, extract_flag_value,
};

use super::basic::print_state_next_step_hint;
use super::json::{render_nodes_json_error, render_state_view_json};

pub(super) fn state_cmd(args: &[String], inventory: &MeshNodesInventory) -> i32 {
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

pub(super) fn persist_runtime_state(
    args: &[String],
    runtime: &MeshNodeRuntime,
) -> Result<(), String> {
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
