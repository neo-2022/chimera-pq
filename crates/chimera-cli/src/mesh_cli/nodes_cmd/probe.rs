use chimera_mesh::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshNodesPolicy, MeshPathPolicy, MeshRuntime,
    group_mesh_nodes_by_country,
};

use crate::mesh_cli::nodes_inventory::{MeshNodesInventory, extract_flag_value};
use crate::mesh_cli::probe_redaction::{endpoint_label, peer_label, public_node_label};

use super::filter::parse_filter;
use super::guard::{proof_pq_strict_enabled, verify_chimera_proof};
use super::json::{render_nodes_json_error, render_probe_all_json};
use super::runtime::build_runtime_from_inventory;
use super::state::persist_runtime_state;

pub(super) fn probe_all(args: &[String], inventory: &MeshNodesInventory) -> i32 {
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
                        "none".to_string()
                    } else {
                        peer_label(&report, &report.connected_peer)
                    },
                    if report.connected_endpoint.is_empty() {
                        "none".to_string()
                    } else {
                        endpoint_label(&report, &report.connected_endpoint)
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

pub(super) fn auto_unblock(args: &[String], inventory: &MeshNodesInventory) -> i32 {
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
            println!(
                "auto_unblock=ok node_id={}",
                public_node_label(&best.node_id.0)
            );
            return 0;
        }
    }
    eprintln!("mesh nodes auto-unblock error: no eligible node after proof");
    2
}
