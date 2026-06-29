use std::collections::BTreeSet;

use chimera_config::RawConfig;
use chimera_mesh::{
    MeshNode, MeshNodeCountry, MeshNodeCountryConfidence, MeshNodeCountrySource, MeshNodeId,
    MeshNodeStatus,
};

use super::{MeshNodesInventory, MeshNodesInventorySource};

pub(crate) fn parse_inventory_config_text(input: &str) -> Result<MeshNodesInventory, String> {
    let raw = RawConfig::parse(input).map_err(|error| error.to_string())?;
    let ids = parse_required_ids(&raw)?;
    validate_config_keys(&raw, &ids)?;

    let mut nodes = Vec::new();
    for id in &ids {
        nodes.push(parse_config_node(&raw, id)?);
    }

    Ok(MeshNodesInventory {
        nodes,
        self_node_id: optional_node_id(&raw, "mesh.nodes.self_node_id")?,
        current_node: optional_node_id(&raw, "mesh.nodes.current")?,
        pinned_node: optional_node_id(&raw, "mesh.nodes.pinned")?,
        autoconnect_enabled: optional_bool(&raw, "mesh.nodes.autoconnect")?,
        restricted_reason: None,
        last_activation_node_id: None,
        last_activation_unix: None,
        source: MeshNodesInventorySource::Config,
    })
}

pub(crate) fn parse_optional_f64(value: &str) -> Result<Option<f64>, String> {
    if value == "-" || value.trim().is_empty() {
        return Ok(None);
    }
    value
        .parse::<f64>()
        .map(Some)
        .map_err(|_| format!("invalid numeric value: {value}"))
}

pub(crate) fn parse_u32(value: &str, label: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("invalid {label}: {value}"))
}
pub(super) fn parse_cli_nodes(args: &[String]) -> Result<Vec<MeshNode>, String> {
    let mut nodes = Vec::new();
    let mut ids = BTreeSet::new();
    let mut index = 0usize;
    while index < args.len() {
        if args[index] == "--node" {
            let raw = args
                .get(index + 1)
                .ok_or_else(|| "--node requires a value".to_string())?;
            let node = parse_cli_node(raw)?;
            if !ids.insert(node.node_id.0.clone()) {
                return Err(format!("duplicate node_id: {}", node.node_id));
            }
            nodes.push(node);
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(nodes)
}

fn parse_cli_node(raw: &str) -> Result<MeshNode, String> {
    let parts = raw.split('@').collect::<Vec<_>>();
    if parts.len() != 12 && parts.len() != 13 {
        return Err("node record must have 12 or 13 @-separated fields".to_string());
    }
    build_node(
        parts[0],
        parts[1],
        parts[2],
        parts[3],
        "node_claim",
        "low",
        "cli",
        "86400",
        "false",
        None,
        parts[4],
        parts[5],
        parts[6],
        parts[7],
        parts[8],
        parts[9],
        parts[10],
        parts[11],
        None,
        parts.get(12).copied(),
        None,
        "cli_node_record",
    )
}

fn parse_config_node(raw: &RawConfig, id: &str) -> Result<MeshNode, String> {
    let prefix = format!("mesh.node.{id}.");
    let invite_token = raw.get(&format!("{prefix}invite_token"));
    build_node(
        id,
        required(raw, &format!("{prefix}endpoint"))?,
        raw.get(&format!("{prefix}country_code"))
            .unwrap_or(MeshNodeCountry::UNKNOWN_CODE),
        raw.get(&format!("{prefix}country_name"))
            .unwrap_or(MeshNodeCountry::UNKNOWN_NAME),
        raw.get(&format!("{prefix}country_source"))
            .unwrap_or("node_claim"),
        raw.get(&format!("{prefix}country_confidence"))
            .unwrap_or("low"),
        raw.get(&format!("{prefix}country_updated_at"))
            .unwrap_or("config"),
        raw.get(&format!("{prefix}country_ttl_sec"))
            .unwrap_or("86400"),
        raw.get(&format!("{prefix}country_conflict"))
            .unwrap_or("false"),
        raw.get(&format!("{prefix}country_conflict_reason")),
        raw.get(&format!("{prefix}status")).unwrap_or("checking"),
        raw.get(&format!("{prefix}latency_ms")).unwrap_or("-"),
        raw.get(&format!("{prefix}jitter_ms")).unwrap_or("-"),
        raw.get(&format!("{prefix}loss_pct")).unwrap_or("-"),
        raw.get(&format!("{prefix}success_rate_5m")).unwrap_or("-"),
        raw.get(&format!("{prefix}success_rate_1h")).unwrap_or("-"),
        raw.get(&format!("{prefix}consecutive_failures"))
            .unwrap_or("0"),
        raw.get(&format!("{prefix}observation_count"))
            .unwrap_or("0"),
        invite_token,
        raw.get(&format!("{prefix}update_bootstrap_url")),
        raw.get(&format!("{prefix}endpoint_generation")),
        raw.get(&format!("{prefix}explain_reason"))
            .unwrap_or("config_node_record"),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_node(
    id: &str,
    endpoint: &str,
    country_code_raw: &str,
    country_name: &str,
    country_source: &str,
    country_confidence: &str,
    country_updated_at: &str,
    country_ttl_sec: &str,
    country_conflict: &str,
    country_conflict_reason: Option<&str>,
    status: &str,
    latency_ms: &str,
    jitter_ms: &str,
    loss_pct: &str,
    success_rate_5m: &str,
    success_rate_1h: &str,
    consecutive_failures: &str,
    observation_count: &str,
    invite_token: Option<&str>,
    update_bootstrap_url: Option<&str>,
    endpoint_generation: Option<&str>,
    explain_reason: &str,
) -> Result<MeshNode, String> {
    let country_code = country_code_raw.to_ascii_uppercase();
    let ttl = country_ttl_sec
        .parse::<u64>()
        .map_err(|_| format!("invalid country_ttl_sec: {country_ttl_sec}"))?;
    let conflict = parse_bool_value(country_conflict, "country_conflict")?;
    let country = if country_code == MeshNodeCountry::UNKNOWN_CODE {
        MeshNodeCountry::unknown(country_updated_at, ttl)
    } else {
        MeshNodeCountry {
            country_code,
            country_name: country_name.to_string(),
            country_source: MeshNodeCountrySource::parse(country_source)?,
            country_confidence: MeshNodeCountryConfidence::parse(country_confidence)?,
            country_updated_at: country_updated_at.to_string(),
            country_ttl_sec: ttl,
            country_conflict: conflict,
            country_conflict_reason: country_conflict_reason.map(str::to_string),
        }
    };
    let node = MeshNode {
        node_id: MeshNodeId::new(id),
        endpoint: endpoint.to_string(),
        invite_token: invite_token
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        update_bootstrap_url: update_bootstrap_url
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        endpoint_generation: parse_optional_endpoint_generation(endpoint_generation)?,
        country,
        status: MeshNodeStatus::parse(status)?,
        latency_ms: parse_optional_f64(latency_ms)?,
        jitter_ms: parse_optional_f64(jitter_ms)?,
        loss_pct: parse_optional_f64(loss_pct)?,
        success_rate_5m: parse_optional_f64(success_rate_5m)?,
        success_rate_1h: parse_optional_f64(success_rate_1h)?,
        consecutive_failures: parse_u32(consecutive_failures, "consecutive_failures")?,
        observation_count: parse_u32(observation_count, "observation_count")?,
        score: 0.0,
        explain_reason: explain_reason.to_string(),
    };
    node.validate()?;
    Ok(node)
}

fn parse_required_ids(raw: &RawConfig) -> Result<Vec<String>, String> {
    let ids = required(raw, "mesh.nodes.ids")?
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Err("mesh.nodes.ids must contain at least one node id".to_string());
    }
    let mut seen = BTreeSet::new();
    for id in &ids {
        validate_config_id(id)?;
        if !seen.insert(id.clone()) {
            return Err(format!("duplicate node_id in mesh.nodes.ids: {id}"));
        }
    }
    Ok(ids)
}

fn validate_config_keys(raw: &RawConfig, ids: &[String]) -> Result<(), String> {
    let ids = ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for key in raw.keys() {
        if matches!(
            key,
            "mesh.nodes.ids"
                | "mesh.nodes.self_node_id"
                | "mesh.nodes.identity_state_path"
                | "mesh.nodes.activation_log_path"
                | "mesh.nodes.runtime_state_path"
                | "mesh.nodes.current"
                | "mesh.nodes.pinned"
                | "mesh.nodes.autoconnect"
        ) {
            continue;
        }
        let Some(rest) = key.strip_prefix("mesh.node.") else {
            return Err(format!("unknown mesh nodes config key: {key}"));
        };
        let Some((id, field)) = rest.rsplit_once('.') else {
            return Err(format!("invalid mesh node config key: {key}"));
        };
        if !ids.contains(id) {
            return Err(format!(
                "mesh node key references id not listed in mesh.nodes.ids: {id}"
            ));
        }
        if !is_allowed_node_field(field) {
            return Err(format!("unknown mesh node field '{field}' in key {key}"));
        }
    }
    Ok(())
}

fn is_allowed_node_field(field: &str) -> bool {
    matches!(
        field,
        "endpoint"
            | "country_code"
            | "country_name"
            | "country_source"
            | "country_confidence"
            | "country_updated_at"
            | "country_ttl_sec"
            | "country_conflict"
            | "country_conflict_reason"
            | "status"
            | "latency_ms"
            | "jitter_ms"
            | "loss_pct"
            | "success_rate_5m"
            | "success_rate_1h"
            | "consecutive_failures"
            | "observation_count"
            | "invite_token"
            | "update_bootstrap_url"
            | "endpoint_generation"
            | "explain_reason"
    )
}

fn parse_optional_endpoint_generation(value: Option<&str>) -> Result<Option<u64>, String> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let generation = value
        .parse::<u64>()
        .map_err(|_| "invalid endpoint_generation".to_string())?;
    if generation == 0 {
        return Err("endpoint_generation must be > 0".to_string());
    }
    Ok(Some(generation))
}

fn validate_config_id(id: &str) -> Result<(), String> {
    if id.trim().is_empty() || id.contains('.') || id.chars().any(|ch| ch.is_whitespace()) {
        return Err(format!("invalid mesh node id for config key syntax: {id}"));
    }
    MeshNodeId::new(id).validate()
}

fn required<'a>(raw: &'a RawConfig, key: &str) -> Result<&'a str, String> {
    raw.get(key)
        .ok_or_else(|| format!("missing required mesh nodes config key: {key}"))
}

fn optional_node_id(raw: &RawConfig, key: &str) -> Result<Option<MeshNodeId>, String> {
    let Some(value) = raw.get(key) else {
        return Ok(None);
    };
    if value.trim().is_empty() || value == "none" {
        return Ok(None);
    }
    validate_config_id(value)?;
    Ok(Some(MeshNodeId::new(value)))
}

fn optional_bool(raw: &RawConfig, key: &str) -> Result<Option<bool>, String> {
    raw.get(key)
        .map(|value| parse_bool_value(value, key))
        .transpose()
}

fn parse_bool_value(value: &str, label: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("{label} must be true or false")),
    }
}
