#![forbid(unsafe_code)]

use chimera_mesh::{
    MeshFailoverEvent, MeshJoinRequest, MeshPathPolicy, MeshPeerHealth, MeshPeerTablePolicy,
    MeshRuntime,
};

mod connect_probe_cmd;
mod connect_probe_flow;
mod contracts_cmd;
mod launch_preflight_cmd;
mod launch_preflight_verify_cmd;
mod nodes_inventory;
mod nodes_render;
mod options;
mod route_explain_contract;
mod route_explain_envelope;
mod route_explain_error;
mod route_explain_error_consts;
mod route_explain_fields;
mod route_explain_health;
mod route_explain_integrity;
mod route_explain_json;
mod route_explain_json_insert;
mod route_explain_meta;
mod route_explain_output;
mod route_explain_pressure;
mod route_explain_recovery;
mod route_explain_recovery_projection;
mod route_explain_types;
#[cfg(test)]
mod tests_connect_launch_error_parity;
#[cfg(test)]
mod tests_connect_launch_error_parity_options;
#[cfg(test)]
mod tests_connect_launch_error_parity_options_required;
#[cfg(test)]
mod tests_connect_probe_flow;
#[cfg(test)]
mod tests_connect_probe_json;
#[cfg(test)]
mod tests_consistency_sources;
#[cfg(test)]
mod tests_contract_constants;
#[cfg(test)]
mod tests_json_contract;
#[cfg(test)]
mod tests_json_error_contract;
#[cfg(test)]
mod tests_json_error_options_parse;
#[cfg(test)]
mod tests_json_error_options_parse_duplicates;
#[cfg(test)]
mod tests_json_error_options_parse_flag_shape;
#[cfg(test)]
mod tests_json_error_snapshot;
#[cfg(test)]
mod tests_json_error_utils;
#[cfg(test)]
mod tests_json_operator_cross_contract;
#[cfg(test)]
mod tests_json_operator_cross_contract_matrix;
#[cfg(test)]
mod tests_json_runner_utils;
#[cfg(test)]
mod tests_json_success_options_parse;
#[cfg(test)]
mod tests_json_success_presence;
#[cfg(test)]
mod tests_json_success_utils;
#[cfg(test)]
mod tests_json_utils;
#[cfg(test)]
mod tests_json_utils_contract;
#[cfg(test)]
mod tests_launch_preflight_json;
#[cfg(test)]
mod tests_launch_preflight_verify_json;
#[cfg(test)]
mod tests_nodes_inventory;
#[cfg(test)]
mod tests_nodes_reenroll;
#[cfg(test)]
mod tests_nodes_runtime_state;
#[cfg(test)]
mod tests_nodes_selection;
#[cfg(test)]
mod tests_options_helpers;
#[cfg(test)]
mod tests_options_parse;
#[cfg(test)]
mod tests_options_parse_duplicates_numeric;
#[cfg(test)]
mod tests_options_parse_flag_shape;
#[cfg(test)]
mod tests_options_parse_json_flag;
#[cfg(test)]
mod tests_options_parse_missing_values;
#[cfg(test)]
mod tests_options_parse_missing_values_required;
#[cfg(test)]
mod tests_options_parse_peer_records;
#[cfg(test)]
mod tests_options_parse_traffic_profile;
#[cfg(test)]
mod tests_route_explain_error_internal;
#[cfg(test)]
mod tests_route_explain_error_internal_matrix;
#[cfg(test)]
mod tests_route_explain_text;

pub(crate) use options::MeshRouteExplainOptions;
#[cfg(test)]
pub(crate) use options::parse_mesh_route_explain_options;
#[cfg(test)]
pub(crate) use route_explain_contract::*;
use route_explain_error::emit_route_explain_error;
use route_explain_error_consts::{
    STAGE_DISCOVERY_MERGE, STAGE_FAILOVER_PLAN, STAGE_HEALTH_STATE_UPDATE, STAGE_OPTIONS_PARSE,
    STAGE_PEER_SPEC, STAGE_PEER_TABLE_POLICY, STAGE_PLAN_PATH, STAGE_POLICY_PARSE,
    STAGE_RESELECTION_PLAN, STAGE_RUNTIME_BOOTSTRAP, STAGE_SIMULATION_INPUT,
};
use route_explain_meta::ROUTE_EXPLAIN_CONTRACT_VERSION;
use route_explain_output::build_mesh_route_explain_output;
use route_explain_types::MeshRouteExplainRender;

mod nodes_cmd;
mod nodes_selection;

pub(crate) fn mesh_command(usage: &str, subcommand: Option<&str>, args: &[String]) -> i32 {
    let Some(subcommand) = subcommand else {
        eprintln!("{usage}");
        return 2;
    };
    match subcommand {
        "nodes" => nodes_cmd::mesh_nodes_command(args),
        "contracts" => contracts_cmd::mesh_contracts_command(usage, args),
        "route-explain" => mesh_route_explain_command(usage, args),
        "connect-probe" => connect_probe_cmd::mesh_connect_probe_command(usage, args),
        "launch-preflight" => launch_preflight_cmd::mesh_launch_preflight_command(usage, args),
        "launch-preflight-verify" => {
            launch_preflight_verify_cmd::mesh_launch_preflight_verify_command(usage, args)
        }
        _ => {
            eprintln!("{usage}");
            2
        }
    }
}

fn mesh_route_explain_command(usage: &str, args: &[String]) -> i32 {
    let options = match options::parse_mesh_route_explain_options(args) {
        Ok(value) => value,
        Err(error) => {
            let namespace = options::extract_non_empty_flag_value(args, "--namespace")
                .unwrap_or_else(|| "unknown".to_string());
            let node = options::extract_non_empty_flag_value(args, "--node")
                .unwrap_or_else(|| "unknown".to_string());
            let json = route_explain_error::build_route_explain_error_json_with_identity(
                &namespace,
                &node,
                STAGE_OPTIONS_PARSE,
                &error,
            );
            if let Some(path) = options::extract_non_empty_flag_value(args, "--out")
                && let Err(error) = std::fs::write(path, &json)
            {
                eprintln!("mesh route explain error write failed: {error}");
                return 1;
            }
            if options::wants_json_output(args) {
                println!("{json}");
                return 2;
            }
            eprintln!("{usage}");
            return 2;
        }
    };
    let mut runtime = match MeshRuntime::bootstrap(&options.namespace, "cli-seed") {
        Ok(value) => value,
        Err(error) => return emit_route_explain_error(&options, STAGE_RUNTIME_BOOTSTRAP, &error),
    };
    if options.table_max_entries.is_some()
        || options.table_max_entries_per_region.is_some()
        || options.table_stale_after_ticks.is_some()
    {
        let mut table_policy = MeshPeerTablePolicy::default();
        if let Some(value) = options.table_max_entries {
            table_policy.max_entries = value;
        }
        if let Some(value) = options.table_max_entries_per_region {
            table_policy.max_entries_per_region = value;
        }
        if let Some(value) = options.table_stale_after_ticks {
            table_policy.stale_after_ticks = value;
        }
        if let Err(error) = runtime.set_peer_table_policy(table_policy) {
            return emit_route_explain_error(&options, STAGE_PEER_TABLE_POLICY, &error);
        }
    }
    let records = match options::parse_mesh_peer_records(&options.peers) {
        Ok(records) => records,
        Err(error) => {
            let stage = if error.contains("duplicate peer node_id") {
                STAGE_SIMULATION_INPUT
            } else {
                STAGE_PEER_SPEC
            };
            return emit_route_explain_error(&options, stage, &error);
        }
    };
    if let Err(error) = validate_simulation_nodes(&options, &records) {
        return emit_route_explain_error(&options, STAGE_SIMULATION_INPUT, &error);
    }
    if let Err(error) = runtime.merge_discovery("cli-peer-list", &records) {
        return emit_route_explain_error(&options, STAGE_DISCOVERY_MERGE, &error);
    }
    let request = MeshJoinRequest {
        namespace: options.namespace.clone(),
        node_name: options.node_name.clone(),
        invite_token: options.invite_token.clone(),
    };
    let policy = match MeshPathPolicy::from_dps_payload(&options.policy_payload) {
        Ok(value) => value,
        Err(error) => return emit_route_explain_error(&options, STAGE_POLICY_PARSE, &error),
    };
    let initial = match runtime.plan_path(&request, &policy) {
        Ok(value) => value,
        Err(error) => return emit_route_explain_error(&options, STAGE_PLAN_PATH, &error),
    };
    let failover_selected = if let Some(failed_node) = options.failed_node_id.as_deref() {
        match runtime.failover_plan(
            &request,
            &policy,
            &MeshFailoverEvent {
                failed_node_id: failed_node.to_string(),
                reason: "cli_manual_failover".to_string(),
            },
        ) {
            Ok(plan) => plan
                .selected_peers
                .first()
                .map(|peer| peer.node_id.clone())
                .unwrap_or_default(),
            Err(error) => return emit_route_explain_error(&options, STAGE_FAILOVER_PLAN, &error),
        }
    } else {
        String::new()
    };

    let cooldown_selected = if let Some(cooldown_node) = options.cooldown_node_id.as_deref() {
        if let Err(error) = runtime.update_health_state(&[MeshPeerHealth {
            node_id: cooldown_node.to_string(),
            healthy: true,
            cooldown_active: true,
        }]) {
            return emit_route_explain_error(&options, STAGE_HEALTH_STATE_UPDATE, &error);
        }
        match runtime.reselection_plan_with_health(&request, &policy, &[]) {
            Ok(plan) => plan
                .selected_peers
                .first()
                .map(|peer| peer.node_id.clone())
                .unwrap_or_default(),
            Err(error) => {
                return emit_route_explain_error(&options, STAGE_RESELECTION_PLAN, &error);
            }
        }
    } else {
        String::new()
    };

    let output = build_mesh_route_explain_output(MeshRouteExplainRender {
        contract_version: ROUTE_EXPLAIN_CONTRACT_VERSION,
        options: &options,
        initial: &initial,
        failover_selected: &failover_selected,
        cooldown_selected: &cooldown_selected,
    });

    if let Some(path) = options.out_path.as_deref()
        && let Err(error) = std::fs::write(path, &output.json)
    {
        eprintln!("mesh route explain write failed: {error}");
        return 1;
    }
    if options.json_output {
        println!("{}", output.json);
    } else {
        println!("{}", output.text);
    }
    0
}

fn validate_simulation_nodes(
    options: &MeshRouteExplainOptions,
    records: &[chimera_mesh::MeshDiscoveryRecord],
) -> Result<(), String> {
    options::validate_unique_peer_node_ids(records)?;
    let known: std::collections::BTreeSet<&str> = records
        .iter()
        .map(|record| record.node_id.as_str())
        .collect();
    if let Some(failed) = options.failed_node_id.as_deref()
        && !known.contains(failed)
    {
        return Err(format!(
            "failed node '{failed}' is not present in --peer set"
        ));
    }
    if let Some(cooldown) = options.cooldown_node_id.as_deref()
        && !known.contains(cooldown)
    {
        return Err(format!(
            "cooldown node '{cooldown}' is not present in --peer set"
        ));
    }
    if let (Some(failed), Some(cooldown)) = (
        options.failed_node_id.as_deref(),
        options.cooldown_node_id.as_deref(),
    ) && failed == cooldown
    {
        return Err(format!(
            "failed node '{failed}' and cooldown node '{cooldown}' must differ"
        ));
    }
    Ok(())
}
