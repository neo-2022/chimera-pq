use super::route_explain_contract::explain_value;

pub(crate) struct PressureExplainFields<'a> {
    pub(crate) selection_summary: &'a str,
    pub(crate) selection_level: &'a str,
    pub(crate) selection_score: &'a str,
    pub(crate) selection_dominant: &'a str,
    pub(crate) selection_action_hint: &'a str,
    pub(crate) selection_compact: &'a str,
    pub(crate) selection_reason: &'a str,
    pub(crate) dps_summary: &'a str,
    pub(crate) dps_level: &'a str,
    pub(crate) dps_score: &'a str,
    pub(crate) dps_dominant: &'a str,
    pub(crate) dps_action_hint: &'a str,
    pub(crate) dps_compact: &'a str,
    pub(crate) dps_reason: &'a str,
    pub(crate) projection_consistency: String,
    pub(crate) projection_gate: &'static str,
}

pub(crate) fn collect_pressure_fields(lines: &[String]) -> PressureExplainFields<'_> {
    let selection_summary = explain_value(lines, "selection_pressure_summary=").unwrap_or(
        "considered:0;selected:0;rejected:0;limit_skipped:0;utilization_pct:0;headroom:0",
    );
    let selection_level = explain_value(lines, "selection_pressure_level=").unwrap_or("unknown");
    let selection_score = explain_value(lines, "selection_pressure_score=").unwrap_or("0");
    let selection_dominant = explain_value(lines, "selection_pressure_dominant=").unwrap_or("none");
    let selection_action_hint =
        explain_value(lines, "selection_pressure_action_hint=").unwrap_or("none");
    let selection_compact = explain_value(lines, "selection_pressure_compact=")
        .unwrap_or("level:unknown;score:0;dominant:none;action:none");
    let selection_reason = explain_value(lines, "selection_pressure_reason=")
        .unwrap_or("level=unknown;dominant=none;blocked=0;health=0;region=0;reliability=0;load=0;limit_skipped=0;headroom=0");

    let dps_summary = explain_value(lines, "dps_payload_selection_pressure_summary=")
        .unwrap_or(selection_summary);
    let dps_level =
        explain_value(lines, "dps_payload_selection_pressure_level=").unwrap_or(selection_level);
    let dps_score =
        explain_value(lines, "dps_payload_selection_pressure_score=").unwrap_or(selection_score);
    let dps_dominant = explain_value(lines, "dps_payload_selection_pressure_dominant=")
        .unwrap_or(selection_dominant);
    let dps_action_hint = explain_value(lines, "dps_payload_selection_pressure_action_hint=")
        .unwrap_or(selection_action_hint);
    let dps_compact = explain_value(lines, "dps_payload_selection_pressure_compact=")
        .unwrap_or(selection_compact);
    let dps_reason =
        explain_value(lines, "dps_payload_selection_pressure_reason=").unwrap_or(selection_reason);
    let summary_match = selection_summary == dps_summary;
    let level_match = selection_level == dps_level;
    let score_match = selection_score == dps_score;
    let compact_match = selection_compact == dps_compact;
    let projection_consistency = format!(
        "summary_match:{};level_match:{};score_match:{};compact_match:{}",
        summary_match, level_match, score_match, compact_match
    );
    let projection_gate = if summary_match && level_match && score_match && compact_match {
        "ok"
    } else {
        "warn:pressure_projection_mismatch"
    };

    PressureExplainFields {
        selection_summary,
        selection_level,
        selection_score,
        selection_dominant,
        selection_action_hint,
        selection_compact,
        selection_reason,
        dps_summary,
        dps_level,
        dps_score,
        dps_dominant,
        dps_action_hint,
        dps_compact,
        dps_reason,
        projection_consistency,
        projection_gate,
    }
}

#[cfg(test)]
mod tests {
    use super::collect_pressure_fields;

    #[test]
    fn pressure_projection_gate_warns_on_dps_mismatch() {
        let lines = vec![
            "selection_pressure_summary=considered:1;selected:1;rejected:0;limit_skipped:0;utilization_pct:100;headroom:0".to_string(),
            "selection_pressure_level=saturated".to_string(),
            "selection_pressure_score=100".to_string(),
            "selection_pressure_compact=level:saturated;score:100;dominant:capacity;action:capacity_full".to_string(),
            "dps_payload_selection_pressure_level=healthy".to_string(),
            "dps_payload_selection_pressure_score=0".to_string(),
            "dps_payload_selection_pressure_compact=level:healthy;score:0;dominant:none;action:none".to_string(),
        ];

        let fields = collect_pressure_fields(&lines);

        assert_eq!(fields.projection_gate, "warn:pressure_projection_mismatch");
        assert_eq!(
            fields.projection_consistency,
            "summary_match:true;level_match:false;score_match:false;compact_match:false"
        );
    }
}
