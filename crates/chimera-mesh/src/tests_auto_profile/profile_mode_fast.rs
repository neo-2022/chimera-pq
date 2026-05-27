use crate::{MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshRuntime};

#[test]
fn plan_auto_profile_switches_to_fast_on_strong_signals() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.10:443".to_string(),
        region: "eu".to_string(),
        load_score: 5,
        reliability_score: 99,
    }];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    let policy = MeshPathPolicy::default_auto();
    let plan = runtime
        .plan_path(&req, &policy)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("path_profile=fast"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("path_profile_reason=auto:fast_signals"))
    );
    assert!(
        plan.explain
            .iter()
            .any(|line| line.contains("effective_prefer_region_diversity=false"))
    );
}
