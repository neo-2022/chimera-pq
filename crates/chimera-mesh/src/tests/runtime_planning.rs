use crate::{MeshDiscoveryRecord, MeshJoinMode, MeshJoinRequest, MeshPathPolicy, MeshRuntime};

#[test]
fn runtime_bootstrap_discovery_and_plan_select_best_peer() {
    let mut runtime = match MeshRuntime::bootstrap("cef-public", "seed-a") {
        Ok(value) => value,
        Err(error) => unreachable!("runtime bootstrap should succeed: {error}"),
    };
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 75,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    assert_eq!(runtime.source_count(), 2);
    assert_eq!(runtime.peer_count(), 2);

    let req = MeshJoinRequest {
        namespace: "  cef-public  ".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 70,
        max_load_score: 60,
        max_peers: 1,
        prefer_region_diversity: true,
        max_selected_per_region: 1,
        min_distinct_regions: 1,
        path_profile_override: None,
        multipath_mode: None,
        multipath_demand: None,
        connect_fallback_ports: vec![443, 8443],
    };
    let plan = match runtime.plan_path(&req, &policy) {
        Ok(value) => value,
        Err(error) => unreachable!("path planning should succeed: {error}"),
    };
    assert_eq!(plan.join_mode, MeshJoinMode::InvitationOnly);
    assert_eq!(plan.selected_peers.len(), 1);
    assert_eq!(plan.selected_peers[0].node_id, "node-a");
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_peers=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_peer_ids=peer#1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_peer_regions=eu"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_peer_endpoints=endpoint#1:<redacted>"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| { line.contains("selected_peer_connect_priority=1:peer#1@<redacted>") })
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains(
            "selected_peer_connect_retry_plan=peer#1@<redacted>:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=<redacted>;fallback_ports=configured",
        )
    }));
    assert!(plan.explain.iter().any(|line| {
        line.contains(
            "selected_peer_connect_backoff_profile=initial=0ms;retry1=250ms;retry2=1000ms;jitter_step=50ms;fanout=1",
        )
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_peer_scores=peer#1:160"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_score_sum=160"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_reliability_avg=90"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selected_load_avg=20"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("candidates_considered=2"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("candidates_selected=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("candidates_rejected_total=0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("candidates_skipped_due_to_max_peers=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("candidates_skipped_due_to_limit=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_pri="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_stage="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_instant_risk="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_trend_risk="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_trigger="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_risk_summary="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_switch_prepare="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_switch_recommend="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_switch_reason="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_switch_guard="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_switch_confidence="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_switch_target="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_action="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_action_reason="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_action_state="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_action_priority="))
    );
    let explain = plan.explain.join("\n");
    assert!(!explain.contains("node-a"));
    assert!(!explain.contains("node-b"));
    assert!(!explain.contains("198.51.100.10"));
    assert!(!explain.contains("198.51.100.11"));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_confirm_passed="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_confirm_n="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_confirm_m="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_confirm_signal_hits="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_confirm_ratio="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_confirm_missing_signals="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_confirm_state="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_confirm_signal_labels="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_confirm_stage="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_confirm_trigger="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_confirm_summary="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_risk_valid="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_switch_valid="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_eligible_candidates="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_tuning_source="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_tuning_weights="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_tuning_thresholds="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_tuning_confirmation="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("peer_table_entries_before=2"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("peer_table_entries_after=2"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("peer_table_dropped_total=0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("peer_table_effective_target_distinct_regions=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("peer_table_runtime_consistency_gate=ok"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("peer_table_runtime_consistency_all_true=true"))
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("peer_table_runtime_consistency_summary=gate=ok;all_true=true")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_degraded_path=false"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("preemptive_shadow_degraded_reason=none"))
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains(
            "preemptive_shadow_degraded_summary=path=false;reason=none;gate=ok;all_true=true",
        )
    }));
    let plan_consistency_summary = plan
        .explain
        .iter()
        .find_map(|line| line.strip_prefix("peer_table_runtime_consistency_summary="))
        .unwrap_or("");
    let plan_degraded_summary = plan
        .explain
        .iter()
        .find_map(|line| line.strip_prefix("preemptive_shadow_degraded_summary="))
        .unwrap_or("");
    assert!(!plan_consistency_summary.is_empty());
    assert!(plan_degraded_summary.ends_with(plan_consistency_summary));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selection_utilization_pct=100"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("selection_headroom=0"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("decision_control_mode=manual_override"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("manual_override_count=3"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("discovery_source_names=seed-a,seed-b"))
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("plan_setup_discovery_table_compact=")
            && line.contains("consistency_gate:")
            && line.contains("degraded:")
    }));
    assert!(plan.explain.iter().any(|line| {
        line.contains(
            "plan_setup_discovery_table_compact_consistency_match_source=plan_setup_compact",
        )
    }));
}

#[test]
fn runtime_plan_core_matches_full_selection_and_schedule() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|error| unreachable!("runtime bootstrap should succeed: {error}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.11:8443".to_string(),
            region: "us".to_string(),
            load_score: 15,
            reliability_score: 95,
        },
    ];
    runtime
        .merge_discovery("seed-b", &records)
        .unwrap_or_else(|error| unreachable!("merge discovery should succeed: {error}"));

    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let policy = MeshPathPolicy::from_dps_payload(
        "allow=mesh;mesh_allowed_regions=eu,us;mesh_max_peers=2;mesh_max_selected_per_region=1;mesh_min_distinct_regions=2;mesh_multipath_mode=flow_shard;mesh_route_binding_id=8123",
    )
    .unwrap_or_else(|error| unreachable!("policy should parse: {error}"));

    let full = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|error| unreachable!("full plan should succeed: {error}"));
    let core = runtime
        .plan_path_core(&req, &policy)
        .unwrap_or_else(|error| unreachable!("core plan should succeed: {error}"));

    assert_eq!(core.namespace, full.namespace);
    assert_eq!(core.join_mode, full.join_mode);
    assert_eq!(core.selected_peers, full.selected_peers);
    assert_eq!(core.multipath_schedule, full.multipath_schedule);
}

#[test]
fn runtime_plan_core_from_dps_payload_matches_full_selection_and_schedule() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|error| unreachable!("runtime bootstrap should succeed: {error}"));
    runtime
        .merge_discovery(
            "seed-b",
            &[
                MeshDiscoveryRecord {
                    node_id: "node-a".to_string(),
                    endpoint: "198.51.100.10:443".to_string(),
                    region: "eu".to_string(),
                    load_score: 20,
                    reliability_score: 90,
                },
                MeshDiscoveryRecord {
                    node_id: "node-b".to_string(),
                    endpoint: "198.51.100.11:8443".to_string(),
                    region: "us".to_string(),
                    load_score: 15,
                    reliability_score: 95,
                },
            ],
        )
        .unwrap_or_else(|error| unreachable!("merge discovery should succeed: {error}"));

    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let payload = concat!(
        "mesh_allowed_regions=eu,us;",
        "mesh_max_peers=2;",
        "mesh_max_selected_per_region=1;",
        "mesh_min_distinct_regions=2;",
        "mesh_multipath_mode=flow_shard;",
        "mesh_route_binding_id=8123"
    );

    let full = runtime
        .plan_path_from_dps_payload(&req, payload)
        .unwrap_or_else(|error| unreachable!("full dps plan should succeed: {error}"));
    let core = runtime
        .plan_path_core_from_dps_payload(&req, payload)
        .unwrap_or_else(|error| unreachable!("core dps plan should succeed: {error}"));

    assert_eq!(core.namespace, full.namespace);
    assert_eq!(core.join_mode, full.join_mode);
    assert_eq!(core.selected_peers, full.selected_peers);
    assert_eq!(core.multipath_schedule, full.multipath_schedule);
    assert_eq!(
        core.multipath_schedule.route_binding_id,
        full.multipath_schedule.route_binding_id
    );
    assert!(!full.explain.is_empty());
}
