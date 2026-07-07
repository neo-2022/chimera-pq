mod advertise;
mod advertise_state;
mod basic;
mod filter;
mod guard;
mod json;
mod probe;
mod reenroll;
mod runtime;
mod state;

use chimera_mesh::{MeshNodesPolicy, refresh_mesh_node_scores};

use super::nodes_inventory::load_mesh_nodes_inventory;
use super::nodes_render::{render_best, render_nodes_list};

#[cfg(test)]
pub(crate) use basic::node_update_bootstrap_url_for_args;
#[cfg(test)]
pub(crate) use basic::selected_node_endpoint;
#[cfg(test)]
pub(crate) use basic::selected_node_peer_spec_for_args;
#[cfg(test)]
pub(crate) use guard::{proof_pq_strict_enabled, verify_chimera_proof, verify_guard_challenge};
#[cfg(test)]
pub(crate) use json::{render_nodes_json_error, render_probe_all_json, render_state_view_json};

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
        "list" => match filter::parse_filter(rest) {
            Ok(filter) => {
                println!("{}", render_nodes_list(&inventory, &filter));
                basic::print_list_next_step_hint();
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
        "explain" => basic::explain_node(rest, &inventory.nodes, &policy),
        "connect" => basic::connect_node(rest, &inventory, &policy),
        "select" => basic::select_node(rest, &inventory, &policy),
        "selected-endpoint" => basic::selected_endpoint(rest, &inventory),
        "selected-update-bootstrap-url" => basic::selected_update_bootstrap_url(rest, &inventory),
        "selected-invite-token" => basic::selected_invite_token(rest, &inventory),
        "selected-peer-spec" => basic::selected_peer_spec(rest, &inventory),
        "pin" => basic::pin_node(rest, &inventory, &policy),
        "unpin" => basic::unpin_node(rest, &inventory, &policy),
        "autoconnect" => basic::autoconnect(rest, &inventory, &policy),
        "auto-unblock" => probe::auto_unblock(rest, &inventory),
        "guard-listen" => guard::guard_listen(rest),
        "state" => state::state_cmd(rest, &inventory),
        "advertise" => advertise::advertise_node(rest, &inventory),
        "re-enroll" => reenroll::re_enroll_node(rest, &inventory),
        "re-enroll-prepare" => reenroll::re_enroll_prepare(rest),
        "re-enroll-submit" => reenroll::re_enroll_submit(rest),
        "probe" if rest.first().map(String::as_str) == Some("--all") => {
            probe::probe_all(rest, &inventory)
        }
        _ => {
            eprintln!("{}", usage());
            2
        }
    }
}

fn usage() -> &'static str {
    "usage: chimera mesh nodes <list|best|explain|connect|select|selected-endpoint|selected-update-bootstrap-url|selected-invite-token|selected-peer-spec|pin|unpin|autoconnect|auto-unblock|guard-listen|state|advertise|re-enroll|re-enroll-prepare|re-enroll-submit|probe> [--config path] [--self-node-id <id>] [--runtime-state <file>] [--namespace <name>] [--json] [--proof-token <token>] [--proof-token-classic <token>] [--proof-token-pq <token>] [--proof-key-id <id>] [--proof-pq-key-id <id>] [--bind <host:port>] [--once] [--discovery-url http(s)://...] [--skip-discovery] [--probe-timeout-ms n] [--node <id@endpoint@country_code@country_name@status@latency_ms@jitter_ms@loss_pct@success5m@success1h@failures@observations[@update_bootstrap_url]>] [--country DE,NL] [--status healthy,checking] [--available-only] [--search text] [--id node_id] [--new-node-id <id>] [--request <file>] [--out <file>] [--key-out <file>] [--state-file <file>] [--update-state-file <file>] [--update-bootstrap-url <url>] [--register <file>] [--key <file>] [--state-out <file>] [--activation-out <file>]"
}
