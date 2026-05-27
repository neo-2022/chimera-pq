use super::*;
#[test]
fn runtime_status_explain_with_dps_payload_contains_hint_lines() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.140:443".to_string(),
            region: "eu".to_string(),
            load_score: 16,
            reliability_score: 96,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.141:443".to_string(),
            region: "us".to_string(),
            load_score: 18,
            reliability_score: 94,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    let payload = "mesh_multipath_mode=flow_shard;mesh_continuity_policy=allow_flow_drain";
    let lines = runtime.status_explain_with_dps_payload(payload);
    assert!(
        lines
            .iter()
            .any(|line| { line.contains("status_preemptive_shadow_switch_mode=flow_drain") })
    );
    assert!(
        lines
            .iter()
            .any(|line| { line.contains("status_preemptive_shadow_hints_status=ok") })
    );
    assert!(
        lines
            .iter()
            .any(|line| { line.contains("status_preemptive_shadow_hints_source=dps_payload") })
    );
    assert!(
        lines.iter().any(|line| {
            line.contains("status_preemptive_shadow_hints_reason=dps_payload_parsed")
        })
    );
    assert!(
        lines
            .iter()
            .any(|line| { line.contains("status_preemptive_shadow_hints_present=true") })
    );
    assert!(
        lines.iter().any(|line| {
            line.contains("status_preemptive_shadow_hints_multipath_mode=flow_shard")
        })
    );
    assert!(lines.iter().any(|line| {
        line.contains("status_preemptive_shadow_hints_continuity_policy=allow_flow_drain")
    }));
    assert!(lines.iter().any(|line| {
        line.contains("status_preemptive_shadow_hints_summary=")
            && line.contains("status=ok")
            && line.contains("present=true")
            && line.contains("reason=dps_payload_parsed")
            && line.contains("multipath_mode=flow_shard")
            && line.contains("continuity_policy=allow_flow_drain")
    }));
    assert!(
        lines
            .iter()
            .any(|line| { line.contains("status_standby_shadow_source=dps_multipath_policy") })
    );
}

#[test]
fn runtime_status_explain_reports_none_distribution_when_empty() {
    let runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let lines = runtime.status_explain();
    assert!(
        lines
            .iter()
            .any(|line| line.contains("status_region_distribution=none"))
    );
    assert!(lines.iter().any(|line| line.contains("status_sources=1")));
    assert!(lines.iter().any(|line| line.contains("status_peers=0")));
}
