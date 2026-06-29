use crate::{
    MeshDiscoveryRecord, MeshFailoverEvent, MeshJoinRequest, MeshPathPolicy, MeshPeerHealth,
    MeshRuntime, MultipathMode,
};

#[test]
fn failover_plan_reselects_peer() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());

    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 80,
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
    let initial = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(initial.selected_peers[0].node_id, "node-a");

    let failover = MeshFailoverEvent {
        failed_node_id: "node-a".to_string(),
        reason: "health_probe_timeout".to_string(),
    };
    let fallback = runtime
        .failover_plan(&req, &policy, &failover)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(fallback.selected_peers[0].node_id, "node-b");

    let fallback_core = runtime
        .failover_plan_core(&req, &policy, &failover)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(fallback_core.selected_peers, fallback.selected_peers);
    assert_eq!(
        fallback_core.multipath_schedule,
        fallback.multipath_schedule
    );
}

#[test]
fn reselection_respects_health_cooldown() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());

    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 80,
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
    let health = vec![MeshPeerHealth {
        node_id: "node-a".to_string(),
        healthy: true,
        cooldown_active: true,
    }];
    let plan = runtime
        .reselection_plan_with_health(&req, &policy, &health)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers[0].node_id, "node-b");

    let core_plan = runtime
        .reselection_plan_core_with_health(&req, &policy, &health)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(core_plan.selected_peers, plan.selected_peers);
    assert_eq!(core_plan.multipath_schedule, plan.multipath_schedule);
}

#[test]
fn persisted_health_state_affects_future_reselection() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    assert!(
        runtime
            .update_health_state(&[MeshPeerHealth {
                node_id: "node-a".to_string(),
                healthy: true,
                cooldown_active: true,
            }])
            .is_ok()
    );
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 80,
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
    let plan = runtime
        .reselection_plan_with_health(&req, &policy, &[])
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(plan.selected_peers[0].node_id, "node-b");
}

#[test]
fn failover_plan_preserves_policy_multipath_mode_after_reselection() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-failed".to_string(),
            endpoint: "198.51.100.12:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.13:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 94,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.14:443".to_string(),
            region: "eu".to_string(),
            load_score: 22,
            reliability_score: 93,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 80,
        max_load_score: 60,
        max_peers: 2,
        prefer_region_diversity: true,
        max_selected_per_region: 2,
        min_distinct_regions: 1,
        path_profile_override: None,
        multipath_mode: Some(MultipathMode::FlowShard),
        multipath_demand: None,
        connect_fallback_ports: vec![443, 8443],
    };
    let event = MeshFailoverEvent {
        failed_node_id: "node-failed".to_string(),
        reason: "health_probe_timeout".to_string(),
    };

    let plan = runtime
        .failover_plan(&req, &policy, &event)
        .unwrap_or_else(|e| unreachable!("{e}"));
    let core_plan = runtime
        .failover_plan_core(&req, &policy, &event)
        .unwrap_or_else(|e| unreachable!("{e}"));

    assert_eq!(plan.selected_peers.len(), 2);
    assert_eq!(core_plan.selected_peers, plan.selected_peers);
    assert_eq!(core_plan.multipath_schedule, plan.multipath_schedule);
    assert_eq!(plan.multipath_schedule.mode.as_str(), "flow_shard");
    assert_eq!(plan.multipath_schedule.active_lane_count, 2);
    assert!(plan.multipath_schedule.route_binding_id.is_none());
    assert_eq!(plan.multipath_schedule.carrier_lane_bindings.len(), 0);
    assert!(
        plan.multipath_schedule
            .lanes
            .iter()
            .all(|lane| lane.peer_node_id != "node-failed")
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("multipath_schedule_execution_status=planner_only_not_carrier_bound")
    }));
}

#[test]
fn reselection_plan_preserves_policy_multipath_mode_after_health_filter() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-cooling".to_string(),
            endpoint: "198.51.100.15:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.16:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 94,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.17:443".to_string(),
            region: "eu".to_string(),
            load_score: 22,
            reliability_score: 93,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 80,
        max_load_score: 60,
        max_peers: 2,
        prefer_region_diversity: true,
        max_selected_per_region: 2,
        min_distinct_regions: 1,
        path_profile_override: None,
        multipath_mode: Some(MultipathMode::FlowShard),
        multipath_demand: None,
        connect_fallback_ports: vec![443, 8443],
    };
    let health = [MeshPeerHealth {
        node_id: "node-cooling".to_string(),
        healthy: true,
        cooldown_active: true,
    }];

    let plan = runtime
        .reselection_plan_with_health(&req, &policy, &health)
        .unwrap_or_else(|e| unreachable!("{e}"));
    let core_plan = runtime
        .reselection_plan_core_with_health(&req, &policy, &health)
        .unwrap_or_else(|e| unreachable!("{e}"));

    assert_eq!(plan.selected_peers.len(), 2);
    assert_eq!(core_plan.selected_peers, plan.selected_peers);
    assert_eq!(core_plan.multipath_schedule, plan.multipath_schedule);
    assert_eq!(plan.multipath_schedule.mode.as_str(), "flow_shard");
    assert_eq!(plan.multipath_schedule.active_lane_count, 2);
    assert!(plan.multipath_schedule.route_binding_id.is_none());
    assert_eq!(plan.multipath_schedule.carrier_lane_bindings.len(), 0);
    assert!(
        plan.multipath_schedule
            .lanes
            .iter()
            .all(|lane| lane.peer_node_id != "node-cooling")
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("multipath_schedule_execution_status=planner_only_not_carrier_bound")
    }));
}

#[test]
fn failover_core_from_dps_payload_preserves_route_binding_schedule() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-failed".to_string(),
            endpoint: "198.51.100.18:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.19:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 94,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.20:443".to_string(),
            region: "eu".to_string(),
            load_score: 22,
            reliability_score: 93,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let event = MeshFailoverEvent {
        failed_node_id: "node-failed".to_string(),
        reason: "health_probe_timeout".to_string(),
    };
    let payload = concat!(
        "mesh_allowed_regions=eu;",
        "mesh_max_peers=2;",
        "mesh_max_selected_per_region=2;",
        "mesh_multipath_mode=flow_shard;",
        "mesh_route_binding_id=7101"
    );

    let full = runtime
        .failover_plan_from_dps_payload(&req, payload, &event)
        .unwrap_or_else(|e| unreachable!("{e}"));
    let core = runtime
        .failover_plan_core_from_dps_payload(&req, payload, &event)
        .unwrap_or_else(|e| unreachable!("{e}"));

    assert_eq!(core.selected_peers, full.selected_peers);
    assert_eq!(core.multipath_schedule, full.multipath_schedule);
    assert_eq!(
        core.multipath_schedule
            .route_binding_id
            .unwrap_or_else(|| unreachable!("route binding should exist"))
            .get(),
        7101
    );
    assert_eq!(core.multipath_schedule.carrier_lane_bindings.len(), 2);
    assert!(
        core.multipath_schedule
            .lanes
            .iter()
            .all(|lane| lane.peer_node_id != "node-failed")
    );
}

#[test]
fn reselection_core_from_dps_payload_preserves_route_binding_schedule() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-cooling".to_string(),
            endpoint: "198.51.100.21:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.22:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 94,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.23:443".to_string(),
            region: "eu".to_string(),
            load_score: 22,
            reliability_score: 93,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: Some("inv-123".to_string()),
    };
    let health = [MeshPeerHealth {
        node_id: "node-cooling".to_string(),
        healthy: true,
        cooldown_active: true,
    }];
    let payload = concat!(
        "mesh_allowed_regions=eu;",
        "mesh_max_peers=2;",
        "mesh_max_selected_per_region=2;",
        "mesh_multipath_mode=flow_shard;",
        "mesh_route_binding_id=7102"
    );

    let full = runtime
        .reselection_plan_with_health_from_dps_payload(&req, payload, &health)
        .unwrap_or_else(|e| unreachable!("{e}"));
    let core = runtime
        .reselection_plan_core_with_health_from_dps_payload(&req, payload, &health)
        .unwrap_or_else(|e| unreachable!("{e}"));

    assert_eq!(core.selected_peers, full.selected_peers);
    assert_eq!(core.multipath_schedule, full.multipath_schedule);
    assert_eq!(
        core.multipath_schedule
            .route_binding_id
            .unwrap_or_else(|| unreachable!("route binding should exist"))
            .get(),
        7102
    );
    assert_eq!(core.multipath_schedule.carrier_lane_bindings.len(), 2);
    assert!(
        core.multipath_schedule
            .lanes
            .iter()
            .all(|lane| lane.peer_node_id != "node-cooling")
    );
}
