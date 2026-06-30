#![forbid(unsafe_code)]

use std::fs;

pub(crate) fn check_runtime_apply_dns_artifact(path: &str) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    content.contains("\"status\":\"ok\"")
        && content.contains("\"kind\":\"runtime_apply_dns_smoke\"")
        && content.contains("\"network_state\":\"modified\"")
        && content.contains("\"rollback_ok\":true")
}

pub(crate) fn check_runtime_apply_route_artifact(path: &str) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    content.contains("\"status\":\"ok\"")
        && content.contains("\"kind\":\"runtime_apply_route_smoke\"")
        && content.contains("\"network_state\":\"modified\"")
        && content.contains("\"rollback_ok\":true")
        && content.contains("\"apply_attempt_ok\":true")
}

pub(crate) fn check_runtime_route_policy_validation_artifact(path: &str) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    content.contains("\"status\":\"ok\"")
        && content.contains("\"kind\":\"runtime_route_policy_validation_smoke\"")
        && content.contains("\"network_state\":\"not_modified\"")
        && content.contains("\"apply_rejected\":true")
        && content.contains("\"state_not_created\":true")
}

pub(crate) fn check_runtime_tun_name_validation_artifact(path: &str) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    content.contains("\"status\":\"ok\"")
        && content.contains("\"kind\":\"runtime_tun_name_validation_smoke\"")
        && content.contains("\"network_state\":\"not_modified\"")
        && content.contains("\"apply_rejected\":true")
        && content.contains("\"state_not_created\":true")
}

pub(crate) fn check_runtime_forced_stop_rollback_artifact(path: &str) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    content.contains("\"status\":\"ok\"")
        && content.contains("\"kind\":\"runtime_forced_stop_rollback_smoke\"")
        && content.contains("\"network_state\":\"modified\"")
        && content.contains("\"apply_attempt_ok\":true")
        && content.contains("\"recover_ok\":true")
        && content.contains("\"down_state_clean\":true")
}

pub(crate) fn check_rollback_json_artifacts(paths: &[(&str, &str, bool)]) -> bool {
    for (path, expected_action, expected_state_existed) in paths {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => return false,
        };
        if !(content.contains("\"status\":\"ok\"")
            && content.contains("\"kind\":\"rollback\"")
            && content.contains(&format!("\"action\":\"{expected_action}\""))
            && content.contains(&format!("\"state_existed\":{expected_state_existed}"))
            && content.contains("\"state_file\":\"<redacted>\"")
            && content.contains("\"state_file_state\":\"")
            && content.contains("\"network_state\":\"not_modified\"")
            && !contains_raw_diagnostic_value(&content))
        {
            return false;
        }
    }
    true
}

pub(crate) fn check_route_explain_artifact(path: &str) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    content.contains("\"status\":\"ok\"")
        && content.contains("\"kind\":\"route_explain\"")
        && content.contains("\"rule_used\":\"")
        && content.contains("\"outbound\":\"")
        && content.contains("\"reason\":\"")
        && content.contains("\"rules_checked\":")
}

pub(crate) fn check_datapath_artifact(path: &str) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    content.contains("\"status\":\"ok\"")
        && content.contains("\"kind\":\"datapath_report\"")
        && content.contains("\"transit_explain\":\"")
        && content.contains("\"block_explain\":\"")
        && content.contains("\"direct_explain\":\"")
        && content.contains("\"network_state\":\"not_modified\"")
}

pub(crate) fn check_doctor_artifacts(paths: &[(&str, &str)]) -> bool {
    for (path, expected_kind) in paths {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => return false,
        };
        if !(content.contains("\"status\":\"ok\"")
            && content.contains(&format!("\"kind\":\"{expected_kind}\""))
            && content.contains("\"network_state\":\"not_modified\"")
            && doctor_redaction_contract_ok(&content, expected_kind)
            && !contains_raw_diagnostic_value(&content))
        {
            return false;
        }
    }
    true
}

pub(crate) fn check_benchmark_artifact(path: &str) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    content.contains("\"status\":\"ok\"")
        && content.contains("\"perf_smoke\":true")
        && content.contains("\"net_sim\":true")
        && content.contains("\"encode_ops_per_sec\":")
        && content.contains("\"decode_ops_per_sec\":")
        && content.contains("\"net_sim_reconnect_events\":")
        && content.contains("\"net_sim_dropped\":")
}

pub(crate) fn check_diag_export_artifact(path: &str) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    content.contains("\"status\":\"ok\"")
        && content.contains("\"kind\":\"diag_export\"")
        && content.contains("\"secrets\":\"<redacted>\"")
        && content.contains("\"carrier_addr\":\"<redacted>\"")
        && content.contains("\"carrier_server_name\":\"<redacted>\"")
        && !contains_raw_diagnostic_value(&content)
}

fn contains_raw_diagnostic_value(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    contains_ipv4_literal(&lower)
        || contains_ipv6_literal(&lower)
        || contains_public_hostname_leak(&lower)
        || lower.contains("/home/")
        || lower.contains("gateway.local")
        || lower.contains("node.example")
        || lower.contains("localhost:")
        || lower.contains("[::")
        || contains_unredacted_sensitive_assignment(&lower)
}

fn doctor_redaction_contract_ok(content: &str, expected_kind: &str) -> bool {
    match expected_kind {
        "doctor" => {
            content.contains("\"carrier_addr\":\"<redacted>\"")
                && content.contains("\"carrier_endpoint_state\":\"")
        }
        "gateway_doctor" => {
            content.contains("\"listen_addr\":\"<redacted>\"")
                && content.contains("\"listen_state\":\"")
        }
        _ => true,
    }
}

fn contains_ipv4_literal(content: &str) -> bool {
    content
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .any(is_ipv4_literal)
}

fn is_ipv4_literal(token: &str) -> bool {
    let mut parts = token.split('.');
    let Some(first) = parts.next() else {
        return false;
    };
    if !is_ipv4_octet(first) {
        return false;
    }
    for _ in 0..3 {
        let Some(part) = parts.next() else {
            return false;
        };
        if !is_ipv4_octet(part) {
            return false;
        }
    }
    parts.next().is_none()
}

fn is_ipv4_octet(part: &str) -> bool {
    !part.is_empty() && part.len() <= 3 && part.parse::<u8>().is_ok()
}

fn contains_ipv6_literal(content: &str) -> bool {
    content
        .split(|c: char| !(c.is_ascii_hexdigit() || c == ':'))
        .any(is_ipv6_literal)
}

fn is_ipv6_literal(token: &str) -> bool {
    let trimmed = token.trim_matches(':');
    trimmed.contains("::")
        || (trimmed.matches(':').count() >= 2
            && trimmed
                .split(':')
                .filter(|part| !part.is_empty())
                .all(|part| part.len() <= 4 && part.chars().all(|c| c.is_ascii_hexdigit())))
}

fn contains_public_hostname_leak(content: &str) -> bool {
    content
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
        .any(is_public_hostname_leak)
}

fn is_public_hostname_leak(token: &str) -> bool {
    let trimmed = token.trim_matches('.');
    if trimmed.len() < 4 || trimmed.contains("<redacted") {
        return false;
    }
    let mut labels = trimmed.split('.');
    let mut label_count = 0usize;
    let mut last = "";
    for label in labels.by_ref() {
        if label.is_empty()
            || label.starts_with('-')
            || label.ends_with('-')
            || !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return false;
        }
        label_count = label_count.saturating_add(1);
        last = label;
    }
    label_count >= 2
        && last.len() >= 2
        && last.len() <= 24
        && last.chars().all(|c| c.is_ascii_alphabetic())
}

fn contains_unredacted_sensitive_assignment(content: &str) -> bool {
    const KEYS: &[&str] = &[
        "password",
        "pass",
        "token",
        "secret",
        "private_key",
        "privatekey",
        "authorization",
    ];
    KEYS.iter()
        .any(|key| contains_unredacted_assignment_for_key(content, key))
}

fn contains_unredacted_assignment_for_key(content: &str, key: &str) -> bool {
    let mut search_from = 0;
    while let Some(relative) = content[search_from..].find(key) {
        let start = search_from + relative;
        let end = start + key.len();
        search_from = end;
        if !has_key_boundary(content, start, end) {
            continue;
        }
        let mut rest = &content[end..];
        rest = rest.trim_start();
        if rest.starts_with('"') {
            rest = rest[1..].trim_start();
        }
        if !(rest.starts_with('=') || rest.starts_with(':')) {
            continue;
        }
        rest = rest[1..].trim_start();
        if rest.starts_with("<redacted>")
            || rest.starts_with("\"<redacted>\"")
            || rest.starts_with("'<redacted>'")
            || rest.starts_with("bearer <redacted>")
        {
            continue;
        }
        if !rest.is_empty() {
            return true;
        }
    }
    false
}

fn has_key_boundary(content: &str, start: usize, end: usize) -> bool {
    let before = content[..start].chars().next_back();
    let after = content[end..].chars().next();
    !before.is_some_and(is_key_char) && !after.is_some_and(is_key_char)
}

fn is_key_char(value: char) -> bool {
    value.is_ascii_alphanumeric() || value == '_'
}

pub(crate) fn check_json_bilingual_message_fields(paths: &[&str]) -> bool {
    paths.iter().all(|path| {
        fs::read_to_string(path)
            .map(|content| {
                content.contains("\"message_en\":\"") && content.contains("\"message_ru\":\"")
            })
            .unwrap_or(false)
    })
}

pub(crate) fn check_cef_phase1_smoke_artifact(path: &str) -> bool {
    fs::read_to_string(path)
        .map(|value| {
            value.contains("\"status\":\"ok\"")
                && value.contains("\"kind\":\"cef_phase1_smoke\"")
                && value.contains("\"network_state\":\"not_modified\"")
                && value.contains("\"mesh_join_mode_resolved\":true")
                && value.contains("\"dht_discovery_record_verified\":true")
                && value.contains("\"dps_policy_fragment_verified\":true")
                && value.contains("\"relay_policy_verified\":true")
                && value.contains("\"emergency_offer_valid\":true")
                && value.contains("\"roaming_cache_active_hit\":true")
                && value.contains("\"reputation_penalty_applied\":true")
        })
        .unwrap_or(false)
}

pub(crate) fn check_mesh_route_explain_artifact(path: &str) -> bool {
    fs::read_to_string(path)
        .map(|value| {
            value.contains("\"status\":\"ok\"")
                && value.contains("\"kind\":\"mesh_route_explain\"")
                && value.contains("\"join_mode\":\"InvitationOnly\"")
                && value.contains("\"initial_selected_peer\":\"peer#1\"")
                && value.contains("\"failover_selected_peer\":\"peer#1\"")
                && value.contains("\"cooldown_selected_peer\":\"peer#1\"")
                && value.contains("\"network_state\":\"not_modified\"")
        })
        .unwrap_or(false)
}

pub(crate) fn check_mesh_auto_adaptive_trace_artifact(path: &str) -> bool {
    fs::read_to_string(path)
        .map(|value| {
            value.contains("\"status\":\"ok\"")
                && value.contains("\"kind\":\"mesh_auto_adaptive_trace\"")
                && value.contains("\"network_state\":\"not_modified\"")
                && value.contains("\"auto_baseline\"")
                && value.contains("\"auto_degraded\"")
                && value.contains("\"manual_override\"")
                && value.contains("effective_filter_source=auto_profile")
                && value.contains("effective_filter_source=manual_override")
                && value.contains("path_profile_reason=auto:fast_signals")
                && value.contains("path_profile_reason=auto:degraded_active")
        })
        .unwrap_or(false)
}
