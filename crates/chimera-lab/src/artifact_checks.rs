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
            && content.contains("\"state_file\":\"")
            && content.contains("\"network_state\":\"not_modified\""))
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
        && content.contains("\"gateway_explain\":\"")
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
            && content.contains("\"network_state\":\"not_modified\""))
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
        && !content.to_ascii_lowercase().contains("password")
        && !content.to_ascii_lowercase().contains("private_key")
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
                && value.contains("\"initial_selected_peer\":\"node-eu-1\"")
                && value.contains("\"failover_selected_peer\":\"node-eu-2\"")
                && value.contains("\"cooldown_selected_peer\":\"node-eu-1\"")
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
