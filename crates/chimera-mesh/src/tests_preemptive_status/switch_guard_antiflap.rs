use super::*;
#[test]
fn runtime_switch_recommendation_is_blocked_by_antiflap_budget_guard() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));

    let table_policy = MeshPeerTablePolicy {
        max_replacements_per_window: 1,
        stability_window_ticks: 64,
        ..MeshPeerTablePolicy::default()
    };
    assert!(runtime.set_peer_table_policy(table_policy).is_ok());

    let initial_records = vec![
        MeshDiscoveryRecord {
            node_id: "node-risky-a".to_string(),
            endpoint: "198.51.100.240:443".to_string(),
            region: "eu".to_string(),
            load_score: 100,
            reliability_score: 0,
        },
        MeshDiscoveryRecord {
            node_id: "node-stable-b".to_string(),
            endpoint: "198.51.100.241:443".to_string(),
            region: "us".to_string(),
            load_score: 20,
            reliability_score: 100,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &initial_records).is_ok());

    let replacement_records = vec![
        MeshDiscoveryRecord {
            node_id: "node-risky-a".to_string(),
            endpoint: "198.51.100.240:443".to_string(),
            region: "eu".to_string(),
            load_score: 100,
            reliability_score: 0,
        },
        MeshDiscoveryRecord {
            node_id: "node-stable-b".to_string(),
            endpoint: "198.51.100.242:443".to_string(),
            region: "us".to_string(),
            load_score: 10,
            reliability_score: 100,
        },
    ];
    assert!(
        runtime
            .merge_discovery("seed-b", &replacement_records)
            .is_ok()
    );
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
    assert!(report.preemptive_shadow_antiflap_blocked);
    assert_eq!(
        report.preemptive_shadow_antiflap_reason,
        "switch_budget_exceeded"
    );
    assert_eq!(report.preemptive_shadow_antiflap_replacements_window, 1);
    assert_eq!(report.preemptive_shadow_antiflap_replacements_limit, 1);
    assert!(!report.preemptive_shadow_switch_recommend);
    assert_eq!(
        report.preemptive_shadow_switch_reason,
        "switch_budget_exceeded"
    );
    assert_eq!(report.preemptive_shadow_action, "keep_hot_standby");
    assert_eq!(
        report.preemptive_shadow_action_reason,
        "switch_budget_exceeded"
    );
}
