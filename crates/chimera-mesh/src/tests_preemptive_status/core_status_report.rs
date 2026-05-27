use super::*;
#[test]
fn runtime_status_report_reflects_current_runtime_state() {
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
            endpoint: "198.51.100.110:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.111:443".to_string(),
            region: "us".to_string(),
            load_score: 12,
            reliability_score: 93,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let report = runtime.status_report();
    assert_eq!(report.namespace, "cef-public");
    assert_eq!(report.source_count, 2);
    assert_eq!(report.peer_count, 2);
    assert_eq!(report.health_state_count, 0);
    assert_eq!(report.table_policy.max_entries, 8);
    assert_eq!(report.table_policy.resilient_region_spread_bonus_weight, 9);
    assert!(report.preemptive_shadow_pri >= 0.0);
    assert!(report.preemptive_shadow_pri <= 100.0);
    assert!(report.preemptive_shadow_instant_risk >= 0.0);
    assert!(report.preemptive_shadow_instant_risk <= 1.0);
    assert!(report.preemptive_shadow_trend_risk >= 0.0);
    assert!(report.preemptive_shadow_trend_risk <= 1.0);
    assert!(!report.preemptive_shadow_stage.is_empty());
    assert!(!report.preemptive_shadow_trigger.is_empty());
    assert!(report.preemptive_shadow_switch_confidence >= 0.0);
    assert!(report.preemptive_shadow_switch_confidence <= 1.0);
    assert!(!report.preemptive_shadow_switch_reason.is_empty());
    assert!(!report.preemptive_shadow_switch_target.is_empty());
    assert_eq!(report.preemptive_shadow_switch_mode, "unknown");
    assert_eq!(report.preemptive_shadow_hints_status, "unknown");
    assert_eq!(report.preemptive_shadow_hints_source, "none");
    assert_eq!(report.preemptive_shadow_hints_reason, "no_payload_context");
    assert!(!report.preemptive_shadow_hints_present);
    assert_eq!(report.preemptive_shadow_hints_multipath_mode, "unknown");
    assert_eq!(report.preemptive_shadow_hints_continuity_policy, "unknown");
    assert_eq!(
        report.preemptive_shadow_hints_summary,
        "status=unknown;present=false;reason=no_payload_context;multipath_mode=unknown;continuity_policy=unknown;source=none"
    );
    assert!(!report.preemptive_shadow_action.is_empty());
    assert!(!report.preemptive_shadow_action_reason.is_empty());
    assert!(!report.preemptive_shadow_action_state.is_empty());
    assert!(report.preemptive_shadow_action_priority <= 3);
    assert!(report.preemptive_shadow_confirm_n <= report.preemptive_shadow_confirm_m);
    assert!(report.preemptive_shadow_confirm_signal_hits <= report.preemptive_shadow_confirm_m);
    assert!(report.preemptive_shadow_confirm_ratio >= 0.0);
    assert!(report.preemptive_shadow_confirm_ratio <= 1.0);
    assert!(report.preemptive_shadow_confirm_missing_signals <= report.preemptive_shadow_confirm_n);
    assert!(report.preemptive_shadow_confirm_state.contains("hits="));
    assert!(!report.preemptive_shadow_confirm_signal_labels.is_empty());
    assert!(!report.preemptive_shadow_confirm_stage.is_empty());
    assert!(!report.preemptive_shadow_confirm_trigger.is_empty());
    assert!(report.preemptive_shadow_risk_valid);
    assert!(report.preemptive_shadow_switch_valid);
    assert!(report.preemptive_shadow_eligible_candidates <= report.peer_count);
    assert!(report.preemptive_shadow_health_blocked_count <= report.health_state_count);
    assert!(
        report.preemptive_shadow_tuning_source == "default"
            || report.preemptive_shadow_tuning_source == "env"
    );
    assert!(!report.preemptive_shadow_tuning_confirmation.is_empty());
    assert!(report.table_runtime_consistency_all_true);
    assert_eq!(report.table_runtime_consistency_gate, "ok");
    assert_eq!(
        report.table_runtime_consistency_summary,
        "gate=ok;all_true=true"
    );
    assert!(
        report
            .plan_setup_discovery_table_compact
            .contains(&format!("sources:{}", report.source_count))
    );
    assert!(report.plan_setup_discovery_table_compact.contains(&format!(
        "entries_after:{}",
        report.table_enforcement.total_peers_after
    )));
    assert!(report.plan_setup_discovery_table_compact.contains(&format!(
        "consistency_gate:{}",
        report.table_runtime_consistency_gate
    )));
    assert!(report.plan_setup_discovery_table_compact.contains(&format!(
        "degraded:{}",
        report.preemptive_shadow_degraded_path
    )));
    assert!(!report.preemptive_shadow_degraded_path);
    assert_eq!(report.preemptive_shadow_degraded_reason, "none");
    assert_eq!(
        report.preemptive_shadow_degraded_summary,
        "path=false;reason=none;gate=ok;all_true=true"
    );
    assert_eq!(
        report.setup_compact_consistency_match_source,
        "status_report"
    );
    assert_eq!(
        report.plan_setup_compact_consistency_match_source,
        "status_report"
    );
    assert!(
        report
            .preemptive_shadow_degraded_summary
            .ends_with(&report.table_runtime_consistency_summary)
    );
    assert!(!report.standby_shadow_mode.is_empty());
    assert!(!report.standby_shadow_target.is_empty());
    assert!(!report.standby_shadow_target_source.is_empty());
    assert!(!report.standby_shadow_reason.is_empty());
    assert!(!report.standby_shadow_source.is_empty());
    assert!(!report.standby_shadow_stage_source.is_empty());
    assert!(report.standby_shadow_stage_source.contains("stage:"));
    assert!(report.standby_shadow_stage_source.contains("trigger:"));
    assert!(!report.standby_shadow_summary.is_empty());
    assert!(report.table_enforcement.total_peers_after <= report.peer_count);
}

#[test]
fn runtime_status_report_with_dps_payload_exposes_hint_modes() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.130:443".to_string(),
            region: "eu".to_string(),
            load_score: 15,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.131:443".to_string(),
            region: "us".to_string(),
            load_score: 17,
            reliability_score: 93,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let payload = "mesh_multipath_mode=standby_only;mesh_continuity_policy=same_egress_only";
    let report = runtime.status_report_with_dps_payload(payload);
    assert_eq!(report.preemptive_shadow_switch_mode, "transport_only");
    assert_eq!(report.preemptive_shadow_hints_status, "ok");
    assert_eq!(report.preemptive_shadow_hints_source, "dps_payload");
    assert_eq!(report.preemptive_shadow_hints_reason, "dps_payload_parsed");
    assert!(report.preemptive_shadow_hints_present);
    assert_eq!(
        report.preemptive_shadow_hints_multipath_mode,
        "standby_only"
    );
    assert_eq!(
        report.preemptive_shadow_hints_continuity_policy,
        "same_egress_only"
    );
    assert_eq!(
        report.preemptive_shadow_hints_summary,
        "status=ok;present=true;reason=dps_payload_parsed;multipath_mode=standby_only;continuity_policy=same_egress_only;source=dps_payload"
    );
    assert_eq!(report.standby_shadow_source, "dps_multipath_policy");
}

#[test]
fn runtime_status_report_without_candidates_uses_safe_switch_fallback() {
    let runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let report = runtime.status_report();
    assert!(!report.preemptive_shadow_switch_prepare);
    assert!(!report.preemptive_shadow_switch_recommend);
    assert_eq!(report.preemptive_shadow_switch_reason, "no_candidate");
    assert_eq!(report.preemptive_shadow_switch_guard, "candidate_guard");
    assert_eq!(
        report.preemptive_shadow_switch_guard_source,
        "candidate_selection"
    );
    assert_eq!(report.preemptive_shadow_switch_target, "none");
    assert_eq!(report.preemptive_shadow_eligible_candidates, 0);
    assert_eq!(report.standby_shadow_target_source, "none");
    assert_eq!(report.table_runtime_consistency_gate, "ok");
    assert!(report.table_runtime_consistency_all_true);
    assert_eq!(
        report.table_runtime_consistency_summary,
        "gate=ok;all_true=true"
    );
    assert!(!report.preemptive_shadow_degraded_path);
    assert_eq!(report.preemptive_shadow_degraded_reason, "none");
    assert_eq!(
        report.preemptive_shadow_degraded_summary,
        "path=false;reason=none;gate=ok;all_true=true"
    );
    assert!(
        report
            .preemptive_shadow_degraded_summary
            .ends_with(&report.table_runtime_consistency_summary)
    );
}

#[test]
fn runtime_status_report_consistency_gate_and_degraded_fields_are_coherent() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.150:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.151:443".to_string(),
            region: "us".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let report = runtime.status_report();

    if report.preemptive_shadow_degraded_path {
        assert!(report.table_runtime_consistency_gate.starts_with("warn:"));
        assert!(
            report
                .table_runtime_consistency_summary
                .starts_with("gate=warn:")
        );
        assert!(
            report
                .table_runtime_consistency_summary
                .ends_with(";all_true=false")
        );
        assert_eq!(
            report.preemptive_shadow_degraded_reason,
            report.table_runtime_consistency_gate
        );
        assert!(
            report
                .preemptive_shadow_degraded_summary
                .starts_with("path=true;reason=warn:")
        );
        assert!(
            report
                .preemptive_shadow_degraded_summary
                .ends_with(";all_true=false")
        );
        assert!(
            report
                .preemptive_shadow_degraded_summary
                .ends_with(&report.table_runtime_consistency_summary)
        );
        assert!(!report.table_runtime_consistency_all_true);
    } else {
        assert_eq!(report.table_runtime_consistency_gate, "ok");
        assert_eq!(
            report.table_runtime_consistency_summary,
            "gate=ok;all_true=true"
        );
        assert_eq!(report.preemptive_shadow_degraded_reason, "none");
        assert_eq!(
            report.preemptive_shadow_degraded_summary,
            "path=false;reason=none;gate=ok;all_true=true"
        );
        assert!(
            report
                .preemptive_shadow_degraded_summary
                .ends_with(&report.table_runtime_consistency_summary)
        );
        assert!(report.table_runtime_consistency_all_true);
    }
    assert!(
        report
            .plan_setup_discovery_table_compact
            .contains(&format!("sources:{}", report.source_count))
    );
    assert!(report.plan_setup_discovery_table_compact.contains(&format!(
        "entries_after:{}",
        report.table_enforcement.total_peers_after
    )));
    assert!(report.plan_setup_discovery_table_compact.contains(&format!(
        "consistency_gate:{}",
        report.table_runtime_consistency_gate
    )));
    assert!(report.plan_setup_discovery_table_compact.contains(&format!(
        "degraded:{}",
        report.preemptive_shadow_degraded_path
    )));
}
