use super::*;
#[test]
fn runtime_status_explain_contains_core_runtime_fields() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy_from_dps_payload(
                "mesh_table_max_entries=8;mesh_table_max_entries_per_region=4;mesh_table_target_distinct_regions=2;mesh_table_stale_after_ticks=16;mesh_table_replacement_min_score_delta=1;mesh_table_degraded_replacement_min_score_delta=1;mesh_table_max_replacements_per_window=8;mesh_table_stability_window_ticks=8;mesh_table_profile_hysteresis_ticks=4;mesh_table_resilient_region_spread_bonus_weight=9"
            )
            .is_ok()
    );
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.120:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.121:443".to_string(),
            region: "us".to_string(),
            load_score: 12,
            reliability_score: 93,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let lines = runtime.status_explain();
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_namespace=cef-public"))
    );
    assert!(
        lines
            .iter()
            .any(|line| { line.contains("status_explain_contract_version=mesh_explain_v1") })
    );
    assert!(lines.iter().any(|line| line.contains("status_sources=2")));
    assert!(lines.iter().any(|line| line.contains("status_peers=2")));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_active_profile="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_pri="))
    );
    let compact_idx = lines
        .iter()
        .position(|line| line.contains("status_preemptive_shadow_compact="))
        .unwrap_or(usize::MAX);
    let pri_idx = lines
        .iter()
        .position(|line| line.contains("status_preemptive_shadow_pri="))
        .unwrap_or(0);
    assert!(compact_idx < pri_idx);
    assert!(lines.iter().any(|line| {
        line.contains("status_preemptive_shadow_compact=") && line.contains("reason_chain=")
    }));
    assert!(lines.iter().any(|line| {
        line.contains("status_preemptive_shadow_compact=")
            && line.contains("setup_match_source=status_report")
            && line.contains("plan_setup_match_source=status_report")
    }));
    assert!(lines.iter().any(|line| {
        line.contains("status_consistency_source_matrix=")
            && line.contains("setup=status_report")
            && line.contains("plan_setup=status_report")
            && line.contains("compact_setup=status_report")
            && line.contains("compact_plan_setup=status_report")
    }));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_instant_risk="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_trend_risk="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_stage="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_trigger="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_standby_shadow_mode="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_standby_shadow_target_source="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_standby_shadow_stage_source=stage:"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_risk_summary="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_switch_prepare="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_switch_recommend="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_switch_reason="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_switch_guard="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_switch_guard_source="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_switch_guard_summary="))
    );
    assert!(
        lines
            .iter()
            .any(|line| { line.contains("status_preemptive_shadow_switch_block_reason_chain=") })
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_switch_confidence="))
    );
    assert!(
        lines
            .iter()
            .any(|line| { line.contains("status_preemptive_shadow_switch_confidence_summary=") })
    );
    assert!(
        lines
            .iter()
            .any(|line| { line.contains("status_preemptive_shadow_candidate_readiness_summary=") })
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_switch_target="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_switch_mode="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_status="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_source="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_source=none"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_reason="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_present="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_multipath_mode="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_continuity_policy="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_hints_summary="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_action="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_action_reason="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_action_state="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_action_priority="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_confirm_passed="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_confirm_n="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_confirm_m="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_confirm_signal_hits="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_confirm_ratio="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_confirm_missing_signals="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_confirm_state="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_confirm_signal_labels="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_confirm_stage="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_confirm_trigger="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_confirm_summary="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_risk_valid="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_switch_valid="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_eligible_candidates="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_health_blocked_count="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_tuning_source="))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_tuning_confirmation="))
    );
    assert!(lines.iter().any(|line| {
        line.contains("status_table_runtime_consistency_summary=gate=ok;all_true=true")
    }));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_degraded_path=false"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_preemptive_shadow_degraded_reason=none"))
    );
    assert!(lines.iter().any(|line| {
        line.contains(
            "status_preemptive_shadow_degraded_summary=path=false;reason=none;gate=ok;all_true=true",
        )
    }));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_region_distribution=eu:1,us:1"))
    );
    assert!(lines.iter().any(|line| {
        line.contains("status_table_policy_resilient_region_spread_bonus_weight=9")
    }));
}

#[test]
fn runtime_status_explain_setup_compact_matches_status_report_field() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.210:443".to_string(),
            region: "eu".to_string(),
            load_score: 11,
            reliability_score: 94,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.211:443".to_string(),
            region: "us".to_string(),
            load_score: 13,
            reliability_score: 91,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());

    let report = runtime.status_report();
    let lines = runtime.status_explain();
    let compact = lines
        .iter()
        .find_map(|line| line.strip_prefix("status_plan_setup_discovery_table_compact="))
        .unwrap_or("");
    let setup_compact_consistency_match = lines
        .iter()
        .find_map(|line| {
            line.strip_prefix("status_plan_setup_discovery_table_compact_consistency_match=")
        })
        .unwrap_or("");
    let setup_compact_consistency_match_source = lines
        .iter()
        .find_map(|line| line.strip_prefix("status_setup_compact_consistency_match_source="))
        .unwrap_or("");
    let plan_setup_compact_consistency_match_source = lines
        .iter()
        .find_map(|line| {
            line.strip_prefix("status_plan_setup_discovery_table_compact_consistency_match_source=")
        })
        .unwrap_or("");

    assert!(!compact.is_empty());
    assert_eq!(compact, report.plan_setup_discovery_table_compact);
    assert_eq!(
        setup_compact_consistency_match,
        report.setup_compact_consistency_match.to_string()
    );
    assert_eq!(
        setup_compact_consistency_match_source,
        report.setup_compact_consistency_match_source
    );
    assert_eq!(
        plan_setup_compact_consistency_match_source,
        report.plan_setup_compact_consistency_match_source
    );
    assert!(compact.contains("sources:"));
    assert!(compact.contains("entries_after:"));
    assert!(compact.contains("consistency_gate:"));
    assert!(compact.contains("degraded:"));
}
