pub(crate) struct MeshRouteExplainTextView<'a> {
    pub(crate) contract_version: &'a str,
    pub(crate) namespace: &'a str,
    pub(crate) node_name: &'a str,
    pub(crate) selected_peer: &'a str,
    pub(crate) join_mode: chimera_mesh::MeshJoinMode,
    pub(crate) consistency_gate: &'a str,
    pub(crate) degraded_path: &'a str,
    pub(crate) confidence_summary: &'a str,
    pub(crate) readiness_summary: &'a str,
    pub(crate) selection_pressure_summary: &'a str,
    pub(crate) selection_pressure_level: &'a str,
    pub(crate) selection_pressure_score: &'a str,
    pub(crate) selection_pressure_dominant: &'a str,
    pub(crate) selection_pressure_action_hint: &'a str,
    pub(crate) selection_pressure_compact: &'a str,
    pub(crate) selection_pressure_reason: &'a str,
    pub(crate) reason_chain: &'a str,
    pub(crate) setup_compact: &'a str,
    pub(crate) setup_compact_consistency: &'a str,
    pub(crate) plan_setup_compact_consistency: &'a str,
    pub(crate) plan_setup_compact_consistency_match: &'a str,
    pub(crate) plan_setup_compact_consistency_match_source: &'a str,
    pub(crate) setup_compact_consistency_match: &'a str,
    pub(crate) setup_compact_consistency_match_source: &'a str,
    pub(crate) status_shadow_setup_match_source_from_compact: &'a str,
    pub(crate) status_shadow_plan_setup_match_source_from_compact: &'a str,
    pub(crate) dps_plan_setup_compact_consistency_match: &'a str,
    pub(crate) dps_plan_setup_compact_consistency_match_source: &'a str,
    pub(crate) dps_setup_compact_consistency_match: &'a str,
    pub(crate) dps_setup_compact_consistency_match_source: &'a str,
    pub(crate) dps_shadow_setup_match_source_from_compact: &'a str,
    pub(crate) dps_shadow_plan_setup_match_source_from_compact: &'a str,
    pub(crate) dps_selection_pressure_summary: &'a str,
    pub(crate) dps_selection_pressure_level: &'a str,
    pub(crate) dps_selection_pressure_score: &'a str,
    pub(crate) dps_selection_pressure_dominant: &'a str,
    pub(crate) dps_selection_pressure_action_hint: &'a str,
    pub(crate) dps_selection_pressure_compact: &'a str,
    pub(crate) dps_selection_pressure_reason: &'a str,
    pub(crate) selection_pressure_projection_consistency: &'a str,
    pub(crate) selection_pressure_projection_gate: &'a str,
    pub(crate) route_explain_health_gate: &'a str,
    pub(crate) route_explain_health_summary: &'a str,
    pub(crate) route_explain_operator_summary: &'a str,
    pub(crate) route_explain_contract_integrity: &'a str,
    pub(crate) consistency_source_matrix: &'a str,
    pub(crate) dps_shadow_compact: &'a str,
}

pub(crate) fn format_mesh_route_explain_text(view: &MeshRouteExplainTextView<'_>) -> String {
    format!(
        "Mesh route explain: contract={} namespace={} node={} selected_peer={} join_mode={:?} route_explain_operator_summary={} route_explain_contract_integrity={} consistency_gate={} degraded={} confidence={} readiness={} selection_pressure={} selection_pressure_level={} selection_pressure_score={} selection_pressure_dominant={} selection_pressure_action_hint={} selection_pressure_compact={} selection_pressure_reason={} reason_chain={} setup_compact={} setup_compact_consistency={} plan_setup_compact_consistency={} plan_setup_compact_consistency_match={} plan_setup_compact_consistency_match_source={} setup_compact_consistency_match={} setup_compact_consistency_match_source={} status_shadow_setup_match_source_from_compact={} status_shadow_plan_setup_match_source_from_compact={} dps_plan_setup_compact_consistency_match={} dps_plan_setup_compact_consistency_match_source={} dps_setup_compact_consistency_match={} dps_setup_compact_consistency_match_source={} dps_shadow_setup_match_source_from_compact={} dps_shadow_plan_setup_match_source_from_compact={} dps_selection_pressure={} dps_selection_pressure_level={} dps_selection_pressure_score={} dps_selection_pressure_dominant={} dps_selection_pressure_action_hint={} dps_selection_pressure_compact={} dps_selection_pressure_reason={} selection_pressure_projection_consistency={} selection_pressure_projection_gate={} route_explain_health_gate={} route_explain_health_summary={} consistency_source_matrix={} dps_shadow_compact={}",
        view.contract_version,
        view.namespace,
        view.node_name,
        view.selected_peer,
        view.join_mode,
        view.route_explain_operator_summary,
        view.route_explain_contract_integrity,
        view.consistency_gate,
        view.degraded_path,
        view.confidence_summary,
        view.readiness_summary,
        view.selection_pressure_summary,
        view.selection_pressure_level,
        view.selection_pressure_score,
        view.selection_pressure_dominant,
        view.selection_pressure_action_hint,
        view.selection_pressure_compact,
        view.selection_pressure_reason,
        view.reason_chain,
        view.setup_compact,
        view.setup_compact_consistency,
        view.plan_setup_compact_consistency,
        view.plan_setup_compact_consistency_match,
        view.plan_setup_compact_consistency_match_source,
        view.setup_compact_consistency_match,
        view.setup_compact_consistency_match_source,
        view.status_shadow_setup_match_source_from_compact,
        view.status_shadow_plan_setup_match_source_from_compact,
        view.dps_plan_setup_compact_consistency_match,
        view.dps_plan_setup_compact_consistency_match_source,
        view.dps_setup_compact_consistency_match,
        view.dps_setup_compact_consistency_match_source,
        view.dps_shadow_setup_match_source_from_compact,
        view.dps_shadow_plan_setup_match_source_from_compact,
        view.dps_selection_pressure_summary,
        view.dps_selection_pressure_level,
        view.dps_selection_pressure_score,
        view.dps_selection_pressure_dominant,
        view.dps_selection_pressure_action_hint,
        view.dps_selection_pressure_compact,
        view.dps_selection_pressure_reason,
        view.selection_pressure_projection_consistency,
        view.selection_pressure_projection_gate,
        view.route_explain_health_gate,
        view.route_explain_health_summary,
        view.consistency_source_matrix,
        view.dps_shadow_compact
    )
}

pub(crate) fn format_consistency_source_matrix(sources: [&str; 10]) -> String {
    format!(
        "plan={};status_plan={};status_setup={};setup={};dps_plan={};dps_setup={};status_compact_plan={};status_compact_setup={};dps_compact_plan={};dps_compact_setup={}",
        sources[0],
        sources[1],
        sources[2],
        sources[3],
        sources[4],
        sources[5],
        sources[6],
        sources[7],
        sources[8],
        sources[9]
    )
}

pub(crate) fn explain_value<'a>(lines: &'a [String], key: &str) -> Option<&'a str> {
    lines.iter().find_map(|line| line.strip_prefix(key))
}

pub(crate) fn explain_value_any<'a>(lines: &'a [String], keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| explain_value(lines, key))
}

pub(crate) fn infer_plan_setup_match_source(lines: &[String]) -> &str {
    if explain_value(
        lines,
        "plan_setup_discovery_table_compact_consistency_match_source=",
    )
    .is_some()
    {
        explain_value(
            lines,
            "plan_setup_discovery_table_compact_consistency_match_source=",
        )
        .unwrap_or("unknown")
    } else if explain_value(
        lines,
        "status_plan_setup_discovery_table_compact_consistency_match_source=",
    )
    .is_some()
    {
        explain_value(
            lines,
            "status_plan_setup_discovery_table_compact_consistency_match_source=",
        )
        .unwrap_or("unknown")
    } else if let Some(status_compact) = explain_value(lines, "status_preemptive_shadow_compact=")
        && let Some(source) = compact_field_value(status_compact, "plan_setup_match_source")
    {
        source
    } else if explain_value(
        lines,
        "plan_setup_discovery_table_compact_consistency_match=",
    )
    .is_some()
    {
        "plan_setup"
    } else if explain_value(
        lines,
        "status_plan_setup_discovery_table_compact_consistency_match=",
    )
    .is_some()
        || explain_value(lines, "status_setup_compact_consistency_match=").is_some()
    {
        "status_report"
    } else {
        "derived"
    }
}

pub(crate) fn infer_status_plan_setup_match_source(lines: &[String]) -> &str {
    if explain_value(
        lines,
        "status_plan_setup_discovery_table_compact_consistency_match_source=",
    )
    .is_some()
    {
        explain_value(
            lines,
            "status_plan_setup_discovery_table_compact_consistency_match_source=",
        )
        .unwrap_or("unknown")
    } else if let Some(status_compact) = explain_value(lines, "status_preemptive_shadow_compact=")
        && let Some(source) = compact_field_value(status_compact, "plan_setup_match_source")
    {
        source
    } else if explain_value(
        lines,
        "status_plan_setup_discovery_table_compact_consistency_match=",
    )
    .is_some()
    {
        "status_report"
    } else {
        "derived"
    }
}

pub(crate) fn infer_status_setup_match_source(lines: &[String]) -> &str {
    if explain_value(lines, "status_setup_compact_consistency_match_source=").is_some() {
        explain_value(lines, "status_setup_compact_consistency_match_source=").unwrap_or("unknown")
    } else if let Some(status_compact) = explain_value(lines, "status_preemptive_shadow_compact=")
        && let Some(source) = compact_field_value(status_compact, "setup_match_source")
    {
        source
    } else if explain_value(lines, "status_setup_compact_consistency_match=").is_some() {
        "status_report"
    } else {
        "derived"
    }
}

pub(crate) fn infer_setup_match_source(lines: &[String]) -> &str {
    if explain_value(lines, "dps_payload_setup_compact_consistency_match_source=").is_some() {
        explain_value(lines, "dps_payload_setup_compact_consistency_match_source=")
            .unwrap_or("unknown")
    } else if explain_value(lines, "status_setup_compact_consistency_match_source=").is_some() {
        explain_value(lines, "status_setup_compact_consistency_match_source=").unwrap_or("unknown")
    } else if let Some(status_compact) = explain_value(lines, "status_preemptive_shadow_compact=")
        && let Some(source) = compact_field_value(status_compact, "setup_match_source")
    {
        source
    } else if let Some(dps_compact) = explain_value(lines, "dps_payload_preemptive_shadow_compact=")
        && let Some(source) = compact_field_value(dps_compact, "setup_match_source")
    {
        source
    } else if explain_value(lines, "dps_payload_setup_compact_consistency_match=").is_some() {
        "dps_payload"
    } else if explain_value_any(
        lines,
        &[
            "plan_setup_discovery_table_compact_consistency_match=",
            "status_plan_setup_discovery_table_compact_consistency_match=",
            "status_setup_compact_consistency_match=",
        ],
    )
    .is_some()
    {
        "plan_or_status"
    } else {
        "derived"
    }
}

pub(crate) fn infer_dps_setup_match_source(lines: &[String]) -> &str {
    if explain_value(lines, "dps_payload_setup_compact_consistency_match_source=").is_some() {
        explain_value(lines, "dps_payload_setup_compact_consistency_match_source=")
            .unwrap_or("unknown")
    } else if let Some(dps_compact) = explain_value(lines, "dps_payload_preemptive_shadow_compact=")
        && let Some(source) = compact_field_value(dps_compact, "setup_match_source")
    {
        source
    } else if explain_value(lines, "dps_payload_setup_compact_consistency_match=").is_some() {
        "dps_payload"
    } else if explain_value(lines, "dps_payload_plan_setup_compact_consistency=").is_some() {
        "dps_payload_compact"
    } else {
        "derived"
    }
}

pub(crate) fn infer_dps_plan_setup_match_source(lines: &[String]) -> &str {
    if explain_value(
        lines,
        "dps_payload_plan_setup_discovery_table_compact_consistency_match_source=",
    )
    .is_some()
    {
        explain_value(
            lines,
            "dps_payload_plan_setup_discovery_table_compact_consistency_match_source=",
        )
        .unwrap_or("unknown")
    } else if let Some(dps_compact) = explain_value(lines, "dps_payload_preemptive_shadow_compact=")
        && let Some(source) = compact_field_value(dps_compact, "plan_setup_match_source")
    {
        source
    } else if explain_value(
        lines,
        "dps_payload_plan_setup_discovery_table_compact_consistency_match=",
    )
    .is_some()
    {
        "dps_payload"
    } else if explain_value(
        lines,
        "plan_setup_discovery_table_compact_consistency_match=",
    )
    .is_some()
    {
        "plan_setup"
    } else {
        "derived"
    }
}

pub(crate) fn compact_field_value<'a>(compact: &'a str, key: &str) -> Option<&'a str> {
    compact.split(';').find_map(|part| {
        let (k, v) = part.split_once('=')?;
        if k == key { Some(v) } else { None }
    })
}

pub(crate) fn parse_consistency_match(summary: &str) -> bool {
    let mut gate_match = None;
    let mut degraded_match = None;
    for part in summary.split(';') {
        if let Some(value) = part.strip_prefix("gate_match:") {
            gate_match = Some(value == "true");
        } else if let Some(value) = part.strip_prefix("degraded_match:") {
            degraded_match = Some(value == "true");
        }
    }
    gate_match.unwrap_or(false) && degraded_match.unwrap_or(false)
}

pub(crate) fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
