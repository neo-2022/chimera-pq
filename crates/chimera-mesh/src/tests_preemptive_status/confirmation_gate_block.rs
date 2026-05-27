use super::*;
#[test]
fn runtime_confirmation_gate_blocks_unconfirmed_switch_stage() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-risky-a".to_string(),
            endpoint: "198.51.100.210:443".to_string(),
            region: "eu".to_string(),
            load_score: 100,
            reliability_score: 0,
        },
        MeshDiscoveryRecord {
            node_id: "node-risky-b".to_string(),
            endpoint: "198.51.100.211:443".to_string(),
            region: "us".to_string(),
            load_score: 100,
            reliability_score: 0,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());

    let report = runtime.status_report();
    assert_eq!(report.active_profile, "balanced");
    assert_eq!(report.preemptive_shadow_stage, "switch");
    assert!(!report.preemptive_shadow_confirm_passed);
    assert_eq!(
        report.preemptive_shadow_confirm_trigger,
        "confirmation_gate_blocked"
    );
    assert_eq!(
        report.preemptive_shadow_confirm_signal_labels,
        "load,reliability"
    );
    assert!((report.preemptive_shadow_confirm_ratio - (2.0 / 3.0)).abs() < 0.0001);
    assert_eq!(report.preemptive_shadow_confirm_missing_signals, 1);
    assert_eq!(
        report.preemptive_shadow_confirm_state,
        "hits=2/3;need=3;missing=1;passed=false"
    );
    assert_eq!(
        report.preemptive_shadow_confirm_summary,
        "hits=2/need=3;stage=clear;trigger=confirmation_gate_blocked"
    );
    assert!(
        report
            .preemptive_shadow_risk_summary
            .contains("stage=switch;trigger=pri_switch_threshold")
    );
    assert!(!report.preemptive_shadow_switch_prepare);
    assert!(!report.preemptive_shadow_switch_recommend);
    assert_eq!(report.preemptive_shadow_action, "hold");
    assert_eq!(report.preemptive_shadow_action_reason, "no_action");
    assert_eq!(
        report.preemptive_shadow_action_state,
        "action=hold;reason=no_action;priority=0;eligible=2"
    );
    assert_eq!(report.preemptive_shadow_action_priority, 0);
    assert_eq!(
        report.preemptive_shadow_switch_reason,
        "confirmation_gate_blocked"
    );
}
