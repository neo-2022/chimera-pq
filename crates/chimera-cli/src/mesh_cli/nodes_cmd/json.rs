use chimera_mesh::MeshConnectProbeReport;

use crate::mesh_cli::nodes_inventory::MeshNodesInventory;
use crate::mesh_cli::probe_redaction::{
    endpoint_label, peer_label, public_node_label, redact_public_diagnostic_text,
};

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
        escape_json(&redact_public_diagnostic_text(message)),
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
                escape_json(&peer_label(report, &a.peer_id)),
                escape_json(&endpoint_label(report, &a.endpoint)),
                if a.success { "true" } else { "false" },
                escape_json(&redact_public_diagnostic_text(&a.error))
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
        escape_json(&if report.connected_peer.is_empty() {
            "none".to_string()
        } else {
            peer_label(report, &report.connected_peer)
        }),
        escape_json(&if report.connected_endpoint.is_empty() {
            "none".to_string()
        } else {
            endpoint_label(report, &report.connected_endpoint)
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
            .map(|v| public_node_label(&v.0))
            .unwrap_or_default(),
        inventory
            .pinned_node
            .as_ref()
            .map(|v| public_node_label(&v.0))
            .unwrap_or_default(),
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

pub(super) fn escape_json(input: &str) -> String {
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
