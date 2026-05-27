use super::tests_contract_constants::{
    BOOL_FALSE, BOOL_TRUE, SOURCE_DERIVED, SOURCE_PLAN_SETUP_COMPACT, SOURCE_PLAN_SETUP_EXPLAIN,
    SOURCE_SETUP_COMPUTED, SOURCE_STATUS_REPORT, SUCCESS_OPERATOR_REASON,
};
use super::*;

#[test]
fn explain_value_any_prefers_first_present_key() {
    let lines = vec![
        "status_plan_setup_discovery_table_compact=from-status".to_string(),
        "plan_setup_discovery_table_compact=from-plan".to_string(),
    ];
    let value = explain_value_any(
        &lines,
        &[
            "plan_setup_discovery_table_compact=",
            "status_plan_setup_discovery_table_compact=",
        ],
    )
    .unwrap_or("");
    assert_eq!(value, "from-plan");
}

#[test]
fn format_consistency_source_matrix_is_stable() {
    let matrix =
        format_consistency_source_matrix(["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]);
    assert_eq!(
        matrix,
        "plan=a;status_plan=b;status_setup=c;setup=d;dps_plan=e;dps_setup=f;status_compact_plan=g;status_compact_setup=h;dps_compact_plan=i;dps_compact_setup=j"
    );
}

#[test]
fn infer_source_helpers_cover_derived_paths() {
    let lines: Vec<String> = Vec::new();
    assert_eq!(infer_plan_setup_match_source(&lines), SOURCE_DERIVED);
    assert_eq!(infer_status_plan_setup_match_source(&lines), SOURCE_DERIVED);
    assert_eq!(infer_status_setup_match_source(&lines), SOURCE_DERIVED);
    assert_eq!(infer_setup_match_source(&lines), SOURCE_DERIVED);
    assert_eq!(infer_dps_setup_match_source(&lines), SOURCE_DERIVED);
    assert_eq!(infer_dps_plan_setup_match_source(&lines), SOURCE_DERIVED);
}

#[test]
fn infer_source_helpers_prefer_explicit_sources() {
    let lines = vec![
        "dps_payload_setup_compact_consistency_match_source=computed_from_setup_compact"
            .to_string(),
        format!("dps_payload_setup_compact_consistency_match={BOOL_TRUE}"),
        "dps_payload_plan_setup_discovery_table_compact_consistency_match_source=plan_setup_explain"
            .to_string(),
        format!("dps_payload_plan_setup_discovery_table_compact_consistency_match={BOOL_TRUE}"),
        "plan_setup_discovery_table_compact_consistency_match_source=plan_setup_compact"
            .to_string(),
        format!("plan_setup_discovery_table_compact_consistency_match={BOOL_TRUE}"),
        "status_plan_setup_discovery_table_compact_consistency_match_source=status_report"
            .to_string(),
        "status_setup_compact_consistency_match_source=status_report".to_string(),
    ];
    assert_eq!(infer_dps_setup_match_source(&lines), SOURCE_SETUP_COMPUTED);
    assert_eq!(
        infer_dps_plan_setup_match_source(&lines),
        SOURCE_PLAN_SETUP_EXPLAIN
    );
    assert_eq!(
        infer_plan_setup_match_source(&lines),
        SOURCE_PLAN_SETUP_COMPACT
    );
    assert_eq!(
        infer_status_plan_setup_match_source(&lines),
        SOURCE_STATUS_REPORT
    );
    assert_eq!(
        infer_status_setup_match_source(&lines),
        SOURCE_STATUS_REPORT
    );
}

#[test]
fn infer_source_helpers_read_compact_fallback_sources() {
    let lines = vec![
        format!(
            "status_preemptive_shadow_compact=stage:clear;trigger:{SUCCESS_OPERATOR_REASON};pri=0.10;degraded={BOOL_FALSE};consistency_gate=ok;confidence=conf=0.9000;reason_chain=reason={SUCCESS_OPERATOR_REASON};setup_compact=sources:2;setup_consistency=gate_match:{BOOL_TRUE};degraded_match:{BOOL_TRUE};setup_match={BOOL_TRUE};setup_match_source=status_report;plan_setup_match_source=status_report"
        ),
        format!(
            "dps_payload_preemptive_shadow_compact=pri=10.00;stage=clear;trigger:{SUCCESS_OPERATOR_REASON};degraded={BOOL_FALSE};consistency_gate=ok;setup_consistency=gate_match:{BOOL_TRUE};degraded_match:{BOOL_TRUE};setup_match={BOOL_TRUE};setup_match_source=computed_from_setup_compact;plan_setup_match_source=plan_setup_explain"
        ),
    ];
    assert_eq!(infer_plan_setup_match_source(&lines), SOURCE_STATUS_REPORT);
    assert_eq!(
        infer_status_plan_setup_match_source(&lines),
        SOURCE_STATUS_REPORT
    );
    assert_eq!(
        infer_status_setup_match_source(&lines),
        SOURCE_STATUS_REPORT
    );
    assert_eq!(infer_setup_match_source(&lines), SOURCE_STATUS_REPORT);
    assert_eq!(infer_dps_setup_match_source(&lines), SOURCE_SETUP_COMPUTED);
    assert_eq!(
        infer_dps_plan_setup_match_source(&lines),
        SOURCE_PLAN_SETUP_EXPLAIN
    );
}
