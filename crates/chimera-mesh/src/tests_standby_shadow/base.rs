use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshRuntime};

#[test]
fn standby_shadow_uses_switch_target_when_preemptive_recommends_candidate() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.41:443".to_string(),
            region: "eu".to_string(),
            load_score: 95,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.42:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 99,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let plan = runtime
        .plan_path(&req, &MeshPathPolicy::default_auto())
        .unwrap_or_else(|e| unreachable!("planning should succeed: {e}"));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("standby_shadow_target=node-b"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("standby_shadow_target_source="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("standby_shadow_target_source=switch_target"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("standby_shadow_summary=mode:"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.starts_with("standby_shadow_warm_ready="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.starts_with("standby_shadow_hot_ready="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.starts_with("standby_shadow_stage_source=stage:"))
    );
}

#[test]
fn standby_shadow_uses_single_peer_target_when_only_one_peer_selected() {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")
        .unwrap_or_else(|e| unreachable!("runtime bootstrap should succeed: {e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.51:443".to_string(),
        region: "eu".to_string(),
        load_score: 30,
        reliability_score: 90,
    }];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let payload = "mesh_allowed_regions=eu;mesh_max_peers=1;mesh_max_selected_per_region=1";
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
            .any(|line| line.contains("standby_shadow_target_source="))
    );
    assert!(plan.explain.iter().any(|line| {
        line.contains("standby_shadow_target_source=selected_primary")
            || line.contains("standby_shadow_target_source=switch_target")
    }));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("standby_shadow_summary=mode:"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.starts_with("standby_shadow_warm_ready="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.starts_with("standby_shadow_hot_ready="))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.starts_with("standby_shadow_stage_source=stage:"))
    );
}
