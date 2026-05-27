use crate::{MeshDiscoveryRecord, MeshRuntime};

#[test]
fn runtime_dps_runtime_flow_status_marks_invalid_hints_summary() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-eu".to_string(),
        endpoint: "198.51.100.200:443".to_string(),
        region: "eu".to_string(),
        load_score: 10,
        reliability_score: 95,
    }];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let lines = runtime.status_explain_with_dps_payload("mesh_multipath_mode=broken");
    assert!(lines.iter().any(|line| {
        line.contains("preemptive_shadow_hints_summary=")
            && line.contains("status=invalid")
            && line.contains("present=false")
            && line.contains("reason=dps_payload_invalid")
            && line.contains("multipath_mode=invalid")
            && line.contains("continuity_policy=invalid")
    }));
    assert!(
        lines
            .iter()
            .any(|line| { line.contains("preemptive_shadow_hints_source=invalid_payload") })
    );
}
