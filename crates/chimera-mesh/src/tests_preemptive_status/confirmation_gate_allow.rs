use super::*;
#[test]
fn runtime_confirmation_gate_allows_switch_when_signals_confirmed() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-fast-a".to_string(),
            endpoint: "198.51.100.220:443".to_string(),
            region: "eu".to_string(),
            load_score: 100,
            reliability_score: 0,
        },
        MeshDiscoveryRecord {
            node_id: "node-stable-b".to_string(),
            endpoint: "198.51.100.221:443".to_string(),
            region: "us".to_string(),
            load_score: 10,
            reliability_score: 100,
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
                MeshPeerHealth {
                    node_id: "ghost-health-c".to_string(),
                    healthy: false,
                    cooldown_active: false,
                },
                MeshPeerHealth {
                    node_id: "ghost-health-d".to_string(),
                    healthy: false,
                    cooldown_active: false,
                },
                MeshPeerHealth {
                    node_id: "ghost-health-e".to_string(),
                    healthy: false,
                    cooldown_active: false,
                },
                MeshPeerHealth {
                    node_id: "ghost-health-f".to_string(),
                    healthy: false,
                    cooldown_active: false,
                },
                MeshPeerHealth {
                    node_id: "ghost-health-g".to_string(),
                    healthy: false,
                    cooldown_active: false,
                },
                MeshPeerHealth {
                    node_id: "ghost-health-h".to_string(),
                    healthy: false,
                    cooldown_active: false,
                },
                MeshPeerHealth {
                    node_id: "ghost-health-i".to_string(),
                    healthy: false,
                    cooldown_active: false,
                },
                MeshPeerHealth {
                    node_id: "ghost-health-j".to_string(),
                    healthy: false,
                    cooldown_active: false,
                },
            ])
            .is_ok()
    );

    let report = runtime.status_report();
    assert_eq!(report.active_profile, "resilient");
    assert_eq!(report.preemptive_shadow_stage, "switch");
    assert!(report.preemptive_shadow_confirm_passed);
    assert_eq!(
        report.preemptive_shadow_confirm_trigger,
        "pri_switch_threshold"
    );
    assert_eq!(
        report.preemptive_shadow_confirm_signal_labels,
        "reliability,health"
    );
    assert!((report.preemptive_shadow_confirm_ratio - (2.0 / 3.0)).abs() < 0.0001);
    assert_eq!(report.preemptive_shadow_confirm_missing_signals, 0);
    assert_eq!(
        report.preemptive_shadow_confirm_state,
        "hits=2/3;need=2;missing=0;passed=true"
    );
    assert_eq!(
        report.preemptive_shadow_confirm_summary,
        "hits=2/need=2;stage=switch;trigger=pri_switch_threshold"
    );
    assert!(
        report
            .preemptive_shadow_risk_summary
            .contains("stage=switch;trigger=pri_switch_threshold")
    );
    assert!(report.preemptive_shadow_switch_prepare);
    assert!(report.preemptive_shadow_switch_recommend);
    assert_eq!(report.preemptive_shadow_action, "recommend_switch");
    assert_eq!(report.preemptive_shadow_action_reason, "switch_recommended");
    assert_eq!(
        report.preemptive_shadow_action_state,
        "action=recommend_switch;reason=switch_recommended;priority=3;eligible=2"
    );
    assert_eq!(report.preemptive_shadow_action_priority, 3);
    assert_eq!(
        report.preemptive_shadow_switch_reason,
        "pri_switch_threshold"
    );
    assert_ne!(report.preemptive_shadow_switch_target, "none");
    assert!(report.preemptive_shadow_eligible_candidates >= 1);
}
