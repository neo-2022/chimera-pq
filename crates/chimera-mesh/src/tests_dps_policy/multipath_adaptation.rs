use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshRuntime};

#[test]
fn dps_traffic_class_gaming_adapts_policy_and_filters_high_load_peer() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.31:443".to_string(),
            region: "eu".to_string(),
            load_score: 41,
            reliability_score: 100,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.32:443".to_string(),
            region: "eu".to_string(),
            load_score: 25,
            reliability_score: 91,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let payload = "mesh_allowed_regions=eu;mesh_traffic_class=gaming_fps";
    let plan = runtime
        .plan_path_from_dps_payload(&req, payload)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    assert_eq!(plan.selected_peers.len(), 1);
    assert_eq!(plan.selected_peers[0].node_id, "node-b");
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_load=40"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_min_reliability=90"))
    );
}

#[test]
fn dps_multipath_flow_shard_raises_peer_limits_when_not_overridden() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.31:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.32:443".to_string(),
            region: "eu".to_string(),
            load_score: 22,
            reliability_score: 91,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let payload = "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard";
    let plan = runtime
        .plan_path_from_dps_payload(&req, payload)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_peers=2"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_selected_per_region=2"))
    );
}

#[test]
fn dps_multipath_aggregate_clamps_selected_per_region_to_explicit_max_peers() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.31:443".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 90,
    }];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let payload = "mesh_allowed_regions=eu;mesh_max_peers=1;mesh_multipath_mode=aggregate_buffered";
    let plan = runtime
        .plan_path_from_dps_payload(&req, payload)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_peers=1"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_selected_per_region=1"))
    );
}

#[test]
fn dps_traffic_class_control_dns_applies_strict_reliability_profile() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.31:443".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 99,
    }];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let payload = "mesh_allowed_regions=eu;mesh_traffic_class=control_dns";
    let plan = runtime
        .plan_path_from_dps_payload(&req, payload)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_min_reliability=95"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_load=45"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_peers=1"))
    );
}

#[test]
fn dps_traffic_class_bulk_transfer_and_legacy_bulk_download_match_policy() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.31:443".to_string(),
            region: "eu".to_string(),
            load_score: 40,
            reliability_score: 95,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.32:443".to_string(),
            region: "eu".to_string(),
            load_score: 30,
            reliability_score: 94,
        },
        MeshDiscoveryRecord {
            node_id: "node-c".to_string(),
            endpoint: "198.51.100.33:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 93,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    for payload in [
        "mesh_allowed_regions=eu;mesh_traffic_class=bulk_transfer",
        "mesh_allowed_regions=eu;mesh_traffic_class=bulk_download",
    ] {
        let plan = runtime
            .plan_path_from_dps_payload(&req, payload)
            .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
        assert!(
            plan.explain
                .iter()
                .any(|line| line.contains("effective_min_reliability=60"))
        );
        assert!(
            plan.explain
                .iter()
                .any(|line| line.contains("effective_max_load=85"))
        );
        assert!(
            plan.explain
                .iter()
                .any(|line| line.contains("effective_max_peers=3"))
        );
    }
}

#[test]
fn dps_traffic_class_auth_sensitive_filters_low_reliability_and_limits_to_one_peer() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.31:443".to_string(),
            region: "eu".to_string(),
            load_score: 45,
            reliability_score: 98,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.32:443".to_string(),
            region: "eu".to_string(),
            load_score: 40,
            reliability_score: 90,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let payload = "mesh_allowed_regions=eu;mesh_traffic_class=auth_sensitive";
    let plan = runtime
        .plan_path_from_dps_payload(&req, payload)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    assert_eq!(plan.selected_peers.len(), 1);
    assert_eq!(plan.selected_peers[0].node_id, "node-a");
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_min_reliability=95"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_load=50"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_peers=1"))
    );
}

#[test]
fn dps_traffic_class_p2p_restricted_expands_to_two_peers_by_default() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.31:443".to_string(),
            region: "eu".to_string(),
            load_score: 35,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.32:443".to_string(),
            region: "eu".to_string(),
            load_score: 30,
            reliability_score: 89,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let payload = "mesh_allowed_regions=eu;mesh_traffic_class=p2p_restricted";
    let plan = runtime
        .plan_path_from_dps_payload(&req, payload)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_max_peers=2"))
    );
}
