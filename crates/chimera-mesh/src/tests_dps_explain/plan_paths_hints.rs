use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshRuntime};

#[test]
fn runtime_plan_path_from_dps_payload_builds_route() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-eu".to_string(),
            endpoint: "198.51.100.50:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-us".to_string(),
            endpoint: "198.51.100.51:443".to_string(),
            region: "us".to_string(),
            load_score: 12,
            reliability_score: 92,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let payload = "mesh_allowed_regions=eu;mesh_max_peers=1;mesh_max_selected_per_region=1;mesh_min_distinct_regions=1;mesh_traffic_class=gaming_fps;mesh_multipath_mode=standby_only;mesh_continuity_policy=same_egress_only";
    let plan = runtime
        .plan_path_from_dps_payload(&req, payload)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers.len(), 1);
    assert_eq!(plan.selected_peers[0].node_id, "node-eu");
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("policy_source=dps_payload"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_origin=plan"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_hints_status=ok"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_hints_status=ok"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_hints_present=true"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_reason=dps_payload_parsed") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_source=dps_payload") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_multipath_mode=standby_only") })
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("preemptive_shadow_hints_continuity_policy=same_egress_only")
    }));
    assert!(plan.explain.iter().any(|line| {
        line.contains("preemptive_shadow_hints_summary=")
            && line.contains("status=ok")
            && line.contains("present=true")
            && line.contains("reason=dps_payload_parsed")
            && line.contains("multipath_mode=standby_only")
            && line.contains("continuity_policy=same_egress_only")
            && line.contains("source=dps_payload")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("preemptive_shadow_switch_mode=transport_only") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_hints_present=true"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_hints_reason=dps_payload_parsed"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_hints_source=dps_payload"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_explain_contract_version=mesh_explain_v1") })
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_hints_summary=")
            && line.contains("multipath_mode=standby_only")
            && line.contains("continuity_policy=same_egress_only")
            && line.contains("source=dps_payload")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_switch_guard="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_switch_guard_source="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_switch_guard_summary="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_switch_block_reason_chain="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_confirm_summary="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_risk_summary="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_preemptive_switch_candidate_confidence=") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_preemptive_switch_confidence_gate_min=") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_preemptive_switch_confidence_gate_passed=") })
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_preemptive_switch_candidate_sample_age_ticks=")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_preemptive_switch_confidence_summary=") })
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_preemptive_switch_confidence_summary=")
            && line.contains(";reason_chain=reason=")
    }));
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_preemptive_shadow_compact=")
            && line.contains("setup_match_source=computed_from_setup_compact")
            && line.contains("plan_setup_match_source=plan_setup_explain")
    }));
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_consistency_source_matrix=")
            && line.contains("setup=computed_from_setup_compact")
            && line.contains("plan_setup=plan_setup_explain")
            && line.contains("compact_setup=computed_from_setup_compact")
            && line.contains("compact_plan_setup=plan_setup_explain")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_candidate_readiness_summary="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_selection_pressure_summary=considered:"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_selection_pressure_level="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_selection_pressure_score="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_selection_pressure_dominant="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_selection_pressure_action_hint="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_selection_pressure_compact=level:"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_selection_pressure_reason=level="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_tuning_summary="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_table_runtime_consistency_gate=ok"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_table_runtime_consistency_all_true=true"))
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_table_runtime_consistency_summary=gate=ok;all_true=true")
    }));
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_plan_setup_discovery_table_compact=")
            && line.contains("join_mode:")
            && line.contains("consistency_gate:")
            && line.contains("degraded:")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_preemptive_degraded_path=false"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_preemptive_degraded_reason=none"))
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains(
            "dps_payload_preemptive_degraded_summary=path=false;reason=none;gate=ok;all_true=true",
        )
    }));
    assert!(plan.explain.iter().any(|line| {
        line.contains(
            "dps_payload_setup_compact_consistency_match_source=computed_from_setup_compact",
        )
    }));
    assert!(plan.explain.iter().any(|line| {
        line.contains(
            "dps_payload_plan_setup_discovery_table_compact_consistency_match_source=plan_setup_explain",
        )
    }));
    let dps_consistency_summary = plan
        .explain
        .iter()
        .find_map(|line| line.strip_prefix("dps_payload_table_runtime_consistency_summary="))
        .unwrap_or("");
    let dps_consistency_gate = plan
        .explain
        .iter()
        .find_map(|line| line.strip_prefix("dps_payload_table_runtime_consistency_gate="))
        .unwrap_or("");
    let dps_degraded_path = plan
        .explain
        .iter()
        .find_map(|line| line.strip_prefix("dps_payload_preemptive_degraded_path="))
        .unwrap_or("");
    let dps_setup_compact = plan
        .explain
        .iter()
        .find_map(|line| line.strip_prefix("dps_payload_plan_setup_discovery_table_compact="))
        .unwrap_or("");
    let dps_degraded_summary = plan
        .explain
        .iter()
        .find_map(|line| line.strip_prefix("dps_payload_preemptive_degraded_summary="))
        .unwrap_or("");
    assert!(!dps_consistency_summary.is_empty());
    assert!(!dps_setup_compact.is_empty());
    assert!(dps_setup_compact.contains("join_mode:"));
    assert!(dps_setup_compact.contains(&format!("consistency_gate:{dps_consistency_gate}")));
    assert!(dps_setup_compact.contains(&format!("degraded:{dps_degraded_path}")));
    assert!(dps_degraded_summary.ends_with(dps_consistency_summary));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_standby_summary=mode="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_standby_mode="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_standby_target_source="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_standby_target_source=switch_target") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_standby_source="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_standby_stage_source=stage:") })
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_standby_stage_source=") && line.contains("trigger:")
    }));
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_standby_summary=") && line.contains("stage_source=")
    }));
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_mesh_keys=mesh_allowed_regions,mesh_continuity_policy,mesh_max_peers,mesh_max_selected_per_region,mesh_min_distinct_regions,mesh_multipath_mode,mesh_traffic_class")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("dps_payload_traffic_class=gaming_fps"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_multipath_mode=standby_only") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_continuity_policy=same_egress_only") })
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("dps_payload_shadow_switch_mode=transport_only") })
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("dps_payload_traffic_profile=lat_p95:20.0,jit_p95:2.0,loss:0.100,pri_warm:0.60,pri_switch:0.85")
    }));
}
