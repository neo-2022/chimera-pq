use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshRuntime};

#[test]
fn standby_shadow_respects_dps_standby_only_mode_with_primary_target() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.61:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.62:443".to_string(),
            region: "eu".to_string(),
            load_score: 21,
            reliability_score: 91,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let payload = "mesh_allowed_regions=eu;mesh_max_peers=2;mesh_max_selected_per_region=2;mesh_multipath_mode=standby_only";
    let plan = runtime
        .plan_path_from_dps_payload(&req, payload)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    assert!(plan.explain.iter().any(|line| {
        line.contains("standby_shadow_target=node-a")
            || line.contains("standby_shadow_target=node-b")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("standby_shadow_source=dps_multipath_policy"))
    );
}

#[test]
fn standby_shadow_respects_dps_flow_shard_mode_with_secondary_target() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.71:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.72:443".to_string(),
            region: "eu".to_string(),
            load_score: 21,
            reliability_score: 91,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let payload = "mesh_allowed_regions=eu;mesh_max_peers=2;mesh_max_selected_per_region=2;mesh_multipath_mode=flow_shard";
    let plan = runtime
        .plan_path_from_dps_payload(&req, payload)
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("standby_shadow_target=node-b"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("standby_shadow_source=dps_multipath_policy"))
    );
}
