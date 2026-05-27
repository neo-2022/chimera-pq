use crate::TrafficClass;

#[test]
fn traffic_class_starter_profile_has_expected_thresholds() {
    let profile = TrafficClass::GamingFps.starter_profile();
    assert_eq!(profile.latency_p95_ms, 20.0);
    assert_eq!(profile.jitter_p95_ms, 2.0);
    assert_eq!(profile.loss_pct, 0.1);
    assert_eq!(profile.pri_warm_threshold, 0.60);
    assert_eq!(profile.pri_switch_threshold, 0.85);
    let profile = TrafficClass::BulkTransfer.starter_profile();
    assert_eq!(profile.latency_p95_ms, 500.0);
    assert_eq!(profile.jitter_p95_ms, 30.0);
    assert_eq!(profile.loss_pct, 3.0);
}
