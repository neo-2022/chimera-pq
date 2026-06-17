use chimera_mesh::MeshConnectProbeReport;

const REDACTED: &str = "<redacted>";

pub(crate) fn peer_label(report: &MeshConnectProbeReport, peer_id: &str) -> String {
    if peer_id.is_empty() {
        return String::new();
    }
    if peer_id == "none" {
        return "none".to_string();
    }
    if is_public_peer_label(peer_id) {
        return peer_id.to_string();
    }
    report
        .selected_peers
        .iter()
        .position(|selected| selected == peer_id)
        .map(|index| format!("peer#{}", index + 1))
        .unwrap_or_else(|| REDACTED.to_string())
}

pub(crate) fn endpoint_label(report: &MeshConnectProbeReport, endpoint: &str) -> String {
    if endpoint.is_empty() {
        return String::new();
    }
    if endpoint == "none" || is_public_endpoint_label(endpoint) {
        return endpoint.to_string();
    }
    report
        .attempts
        .iter()
        .position(|attempt| attempt.endpoint == endpoint)
        .map(|index| format!("endpoint#{}:<redacted>", index + 1))
        .unwrap_or_else(|| REDACTED.to_string())
}

pub(crate) fn public_node_label(node_id: &str) -> String {
    let value = node_id.trim();
    if value.is_empty() || value == "none" || is_public_peer_label(value) {
        value.to_string()
    } else {
        REDACTED.to_string()
    }
}

pub(crate) fn public_diagnostic_node_label(node_id: &str) -> String {
    if node_id.trim().is_empty() {
        String::new()
    } else {
        REDACTED.to_string()
    }
}

pub(crate) fn redact_public_diagnostic_text(input: &str) -> String {
    let mut redact_next = false;
    input
        .split_whitespace()
        .map(|token| {
            if redact_next {
                redact_next = false;
                return REDACTED.to_string();
            }
            if is_sensitive_context_marker(token) {
                redact_next = true;
                return token.to_string();
            }
            redact_public_diagnostic_token(token)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_public_peer_label(value: &str) -> bool {
    value
        .strip_prefix("peer#")
        .and_then(|suffix| suffix.parse::<usize>().ok())
        .is_some_and(|index| index > 0)
}

fn is_public_endpoint_label(value: &str) -> bool {
    let Some(index) = value
        .strip_prefix("endpoint#")
        .and_then(|suffix| suffix.strip_suffix(":<redacted>"))
        .and_then(|index| index.parse::<usize>().ok())
    else {
        return false;
    };
    index > 0
}

fn redact_public_diagnostic_token(token: &str) -> String {
    let trimmed = token.trim_matches(|ch: char| {
        matches!(
            ch,
            '\'' | '"' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}'
        )
    });
    if trimmed.parse::<std::net::IpAddr>().is_ok()
        || trimmed.parse::<std::net::SocketAddr>().is_ok()
    {
        return REDACTED.to_string();
    }
    if token_contains_socket_hint(trimmed) {
        return REDACTED.to_string();
    }
    let mut token = token.to_string();
    for marker in [
        "node_id=",
        "peer_id=",
        "peer=",
        "endpoint=",
        "connected_endpoint=",
    ] {
        if token.contains(marker) {
            token = REDACTED.to_string();
            break;
        }
    }
    if token.contains('@') && !token.contains("peer#") && !token.contains(REDACTED) {
        return REDACTED.to_string();
    }
    token
}

fn token_contains_socket_hint(token: &str) -> bool {
    if token.contains("://") {
        return true;
    }
    token.rsplit_once(':').is_some_and(|(host, port)| {
        port.parse::<u16>().is_ok()
            && (host.contains('.')
                || host.contains('[')
                || host.contains(']')
                || host == "localhost")
    })
}

fn is_sensitive_context_marker(token: &str) -> bool {
    matches!(
        token.trim_end_matches(':'),
        "node_id" | "peer_id" | "endpoint" | "endpoints"
    )
}

pub(crate) fn selected_peer_labels(report: &MeshConnectProbeReport) -> Vec<String> {
    report
        .selected_peers
        .iter()
        .enumerate()
        .map(|(index, _)| format!("peer#{}", index + 1))
        .collect()
}

pub(crate) fn redact_explain_line(line: &str, report: &MeshConnectProbeReport) -> String {
    if let Some(value) = line.strip_prefix("preemptive_shadow_switch_target=") {
        return format!(
            "preemptive_shadow_switch_target={}",
            peer_label(report, value)
        );
    }
    if let Some(value) = line.strip_prefix("standby_shadow_target=") {
        return format!("standby_shadow_target={}", peer_label(report, value));
    }
    if let Some(value) = line.strip_prefix("selected_peer_ids=") {
        return format!("selected_peer_ids={}", redact_peer_list(value, report));
    }
    if let Some(value) = line.strip_prefix("selected_peer_endpoints=") {
        return format!(
            "selected_peer_endpoints={}",
            redact_endpoint_list(value, report)
        );
    }
    if let Some(value) = line.strip_prefix("selected_peer_connect_priority=") {
        return format!(
            "selected_peer_connect_priority={}",
            redact_connect_priority(value, report)
        );
    }
    if let Some(value) = line.strip_prefix("selected_peer_connect_retry_plan=") {
        return format!(
            "selected_peer_connect_retry_plan={}",
            redact_connect_retry_plan(value, report)
        );
    }
    if let Some(value) = line.strip_prefix("selected_peer_scores=") {
        return format!(
            "selected_peer_scores={}",
            redact_peer_metric_list(value, report)
        );
    }
    if let Some(value) = line.strip_prefix("selected_peer_stability=") {
        return format!(
            "selected_peer_stability={}",
            redact_peer_metric_list(value, report)
        );
    }
    if let Some(value) = line.strip_prefix("selected_effective_replacement_thresholds=") {
        return format!(
            "selected_effective_replacement_thresholds={}",
            redact_peer_metric_list(value, report)
        );
    }
    if let Some(value) = line.strip_prefix("selected_replacement_decisions=") {
        return format!(
            "selected_replacement_decisions={}",
            redact_peer_metric_list(value, report)
        );
    }
    if let Some(value) = line.strip_prefix("selected_replacement_budget_remaining=") {
        return format!(
            "selected_replacement_budget_remaining={}",
            redact_peer_metric_list(value, report)
        );
    }
    if let Some(value) = line.strip_prefix("connect_probe_connected_peer=") {
        return format!("connect_probe_connected_peer={}", peer_label(report, value));
    }
    if let Some(value) = line.strip_prefix("connect_probe_connected_endpoint=") {
        return format!(
            "connect_probe_connected_endpoint={}",
            endpoint_label(report, value)
        );
    }
    redact_public_diagnostic_text(line)
}

fn redact_peer_list(value: &str, report: &MeshConnectProbeReport) -> String {
    if value == "none" || value.is_empty() {
        return value.to_string();
    }
    value
        .split(',')
        .map(|peer| peer_label(report, peer))
        .collect::<Vec<_>>()
        .join(",")
}

fn redact_endpoint_list(value: &str, report: &MeshConnectProbeReport) -> String {
    if value == "none" || value.is_empty() {
        return value.to_string();
    }
    value
        .split(',')
        .map(|endpoint| endpoint_label(report, endpoint))
        .collect::<Vec<_>>()
        .join(",")
}

fn redact_connect_priority(value: &str, report: &MeshConnectProbeReport) -> String {
    if value == "none" || value.is_empty() {
        return value.to_string();
    }
    value
        .split(',')
        .map(|entry| {
            let Some((rank, peer_endpoint)) = entry.split_once(':') else {
                return "<redacted>".to_string();
            };
            let Some((peer_id, _endpoint)) = peer_endpoint.split_once('@') else {
                return format!("{rank}:<redacted>");
            };
            format!("{}:{}@<redacted>", rank, peer_label(report, peer_id))
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn redact_connect_retry_plan(value: &str, report: &MeshConnectProbeReport) -> String {
    if value == "none" || value.is_empty() {
        return value.to_string();
    }
    value
        .split(',')
        .map(|entry| redact_retry_plan_entry(entry, report))
        .collect::<Vec<_>>()
        .join(",")
}

fn redact_retry_plan_entry(entry: &str, report: &MeshConnectProbeReport) -> String {
    let Some((peer_id, rest)) = entry.split_once('@') else {
        return "<redacted>".to_string();
    };
    let peer = peer_label(report, peer_id);
    let Some(action_start) = rest.find(":try0(") else {
        return format!("{peer}@<redacted>");
    };
    let actions = &rest[(action_start + 1)..];
    let actions = redact_retry_plan_actions(actions, report);
    format!("{peer}@<redacted>:{actions}")
}

fn redact_retry_plan_actions(actions: &str, report: &MeshConnectProbeReport) -> String {
    actions
        .split(';')
        .map(|part| {
            if let Some(value) = part.strip_prefix("ports=") {
                let fallback = value
                    .split_once(";fallback_ports=")
                    .map(|(_, fallback)| fallback)
                    .unwrap_or("");
                if fallback.is_empty() {
                    "ports=<redacted>".to_string()
                } else {
                    format!("ports=<redacted>;fallback_ports={fallback}")
                }
            } else if let Some(value) = part.strip_prefix("fallback_ports=") {
                format!("fallback_ports={value}")
            } else if let Some(value) = part.strip_prefix("fallback:") {
                redact_retry_fallback(value, report)
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn redact_retry_fallback(value: &str, report: &MeshConnectProbeReport) -> String {
    let Some((peer_id, _endpoint)) = value.split_once('@') else {
        return "fallback:<redacted>".to_string();
    };
    format!("fallback:{}@<redacted>", peer_label(report, peer_id))
}

fn redact_peer_metric_list(value: &str, report: &MeshConnectProbeReport) -> String {
    if value == "none" || value.is_empty() {
        return value.to_string();
    }
    value
        .split(',')
        .map(|entry| redact_peer_metric_entry(entry, report))
        .collect::<Vec<_>>()
        .join(",")
}

fn redact_peer_metric_entry(entry: &str, report: &MeshConnectProbeReport) -> String {
    let Some((peer_id, suffix)) = entry.split_once(':') else {
        return peer_label(report, entry);
    };
    format!("{}:{}", peer_label(report, peer_id), suffix)
}
