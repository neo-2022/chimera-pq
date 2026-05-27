use super::*;
#[test]
fn runtime_switch_recommendation_is_blocked_when_candidate_confidence_is_low() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.230:443".to_string(),
            region: "eu".to_string(),
            load_score: 100,
            reliability_score: 0,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.231:443".to_string(),
            region: "us".to_string(),
            load_score: 100,
            reliability_score: 0,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    assert!(
        runtime
            .update_health_state(&[
                MeshPeerHealth {
                    node_id: "ghost-health-a".to_string(),
                    healthy: false,
                    cooldown_active: false,
                },
                MeshPeerHealth {
                    node_id: "ghost-health-b".to_string(),
                    healthy: false,
                    cooldown_active: false,
                },
            ])
            .is_ok()
    );

    let report = runtime.status_report();
    assert!(report.preemptive_shadow_confirm_passed);
    assert_eq!(report.preemptive_shadow_stage, "switch");
    assert!(!report.preemptive_shadow_switch_recommend);
    assert_eq!(
        report.preemptive_shadow_switch_reason,
        "candidate_low_confidence"
    );
    assert_eq!(report.preemptive_shadow_switch_guard, "confidence_guard");
    assert_eq!(
        report.preemptive_shadow_switch_guard_source,
        "switch_confidence_gate"
    );
    assert_eq!(
        report.preemptive_shadow_switch_guard_summary,
        "confidence_guard|switch_confidence_gate"
    );
    assert!(!report.preemptive_shadow_antiflap_blocked);
    assert_eq!(report.preemptive_shadow_antiflap_reason, "none");
    assert_eq!(report.preemptive_shadow_action, "hold");
}
