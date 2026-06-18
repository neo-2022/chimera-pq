use crate::{
    MeshMultipathRebuildAction, MeshMultipathRebuildPolicy, MeshMultipathRebuildSignal, MeshRuntime,
};

use super::{record, runtime_with_peers};

fn policy() -> MeshMultipathRebuildPolicy {
    MeshMultipathRebuildPolicy::new(3, 4)
        .unwrap_or_else(|e| unreachable!("policy should be accepted: {e}"))
}

fn soft_signal(
    reason: &str,
    generation: u64,
    fingerprint: u64,
    epoch: u64,
    observed_tick: u64,
) -> MeshMultipathRebuildSignal {
    MeshMultipathRebuildSignal::soft(reason, generation, fingerprint, epoch, observed_tick)
        .unwrap_or_else(|e| unreachable!("signal should be accepted: {e}"))
}

fn urgent_signal(observed_tick: u64) -> MeshMultipathRebuildSignal {
    MeshMultipathRebuildSignal::urgent_failover(
        "urgent_failover",
        10,
        0xfeed_face,
        3,
        observed_tick,
    )
    .unwrap_or_else(|e| unreachable!("signal should be accepted: {e}"))
}

fn hard_signal(observed_tick: u64) -> MeshMultipathRebuildSignal {
    MeshMultipathRebuildSignal::hard_safety(
        "route_binding_mismatch",
        10,
        0xfeed_face,
        3,
        observed_tick,
    )
    .unwrap_or_else(|e| unreachable!("signal should be accepted: {e}"))
}

fn advance_tick(runtime: &mut MeshRuntime, source: &str) {
    runtime
        .merge_discovery(source, &[])
        .unwrap_or_else(|e| unreachable!("empty discovery tick should succeed: {e}"));
}

fn seeded_runtime() -> MeshRuntime {
    runtime_with_peers(vec![record("node-a", "198.51.100.31:443", "eu", 10, 95)])
}

#[test]
fn first_rebuild_signal_is_allowed() {
    let mut runtime = seeded_runtime();
    let signal = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);

    let decision = runtime
        .evaluate_multipath_rebuild(&signal, &policy())
        .unwrap_or_else(|e| unreachable!("rebuild gate should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::AllowRebuild);
    assert!(decision.rebuild_allowed);
    assert_eq!(decision.reason, "initial_observation");
    assert!(!decision.debounced);
    assert!(!decision.stale);
    assert_eq!(decision.pending_count, 0);
    assert!(
        decision
            .explain
            .iter()
            .any(|line| line == "multipath_rebuild_action=allow_rebuild")
    );
}

#[test]
fn duplicate_soft_signal_inside_debounce_window_is_suppressed() {
    let mut runtime = seeded_runtime();
    let first = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);
    let duplicate = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);

    let first_decision = runtime
        .evaluate_multipath_rebuild(&first, &policy())
        .unwrap_or_else(|e| unreachable!("first signal should evaluate: {e}"));
    let second_decision = runtime
        .evaluate_multipath_rebuild(&duplicate, &policy())
        .unwrap_or_else(|e| unreachable!("duplicate signal should evaluate: {e}"));

    assert_eq!(
        first_decision.action,
        MeshMultipathRebuildAction::AllowRebuild
    );
    assert_eq!(
        second_decision.action,
        MeshMultipathRebuildAction::SuppressRebuild
    );
    assert_eq!(second_decision.reason, "debounced_same_fingerprint");
    assert!(second_decision.debounced);
    assert_eq!(second_decision.pending_count, 1);
}

#[test]
fn duplicate_soft_signal_after_debounce_window_is_allowed() {
    let mut runtime = seeded_runtime();
    let signal = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);

    runtime
        .evaluate_multipath_rebuild(&signal, &policy())
        .unwrap_or_else(|e| unreachable!("first signal should evaluate: {e}"));
    advance_tick(&mut runtime, "seed-c");
    advance_tick(&mut runtime, "seed-d");
    advance_tick(&mut runtime, "seed-e");
    let later = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 4);
    let decision = runtime
        .evaluate_multipath_rebuild(&later, &policy())
        .unwrap_or_else(|e| unreachable!("later signal should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::AllowRebuild);
    assert_eq!(decision.reason, "debounce_window_elapsed");
    assert!(!decision.debounced);
    assert_eq!(decision.pending_count, 0);
}

#[test]
fn changed_reason_generation_or_fingerprint_bypasses_duplicate_suppression() {
    let mut runtime = seeded_runtime();
    let first = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);
    let reason_changed = soft_signal("capacity_pressure", 1, 0x1001, 1, 1);
    let generation_changed = soft_signal("capacity_pressure", 2, 0x1001, 1, 1);
    let fingerprint_changed = soft_signal("capacity_pressure", 2, 0x2002, 1, 1);

    runtime
        .evaluate_multipath_rebuild(&first, &policy())
        .unwrap_or_else(|e| unreachable!("first signal should evaluate: {e}"));
    let by_reason = runtime
        .evaluate_multipath_rebuild(&reason_changed, &policy())
        .unwrap_or_else(|e| unreachable!("reason change should evaluate: {e}"));
    let by_generation = runtime
        .evaluate_multipath_rebuild(&generation_changed, &policy())
        .unwrap_or_else(|e| unreachable!("generation change should evaluate: {e}"));
    let by_fingerprint = runtime
        .evaluate_multipath_rebuild(&fingerprint_changed, &policy())
        .unwrap_or_else(|e| unreachable!("fingerprint change should evaluate: {e}"));

    assert_eq!(by_reason.action, MeshMultipathRebuildAction::AllowRebuild);
    assert_eq!(by_reason.reason, "reason_changed");
    assert_eq!(
        by_generation.action,
        MeshMultipathRebuildAction::AllowRebuild
    );
    assert_eq!(by_generation.reason, "generation_changed");
    assert_eq!(
        by_fingerprint.action,
        MeshMultipathRebuildAction::AllowRebuild
    );
    assert_eq!(by_fingerprint.reason, "fingerprint_changed");
    assert!(by_fingerprint.fingerprint_changed);
}

#[test]
fn stale_telemetry_fails_closed_instead_of_using_debounce() {
    let mut runtime = seeded_runtime();
    for source in ["seed-c", "seed-d", "seed-e", "seed-f", "seed-g"] {
        advance_tick(&mut runtime, source);
    }
    let stale = soft_signal("demand_rebuild_recommended", 1, 0x1001, 1, 1);

    let decision = runtime
        .evaluate_multipath_rebuild(&stale, &policy())
        .unwrap_or_else(|e| unreachable!("stale signal should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::FailClosed);
    assert_eq!(decision.reason, "stale_telemetry");
    assert!(decision.stale);
    assert!(!decision.rebuild_allowed);
}

#[test]
fn urgent_failover_bypasses_debounce() {
    let mut runtime = seeded_runtime();
    let first = soft_signal("demand_rebuild_recommended", 10, 0xfeed_face, 3, 1);
    let urgent = urgent_signal(1);

    runtime
        .evaluate_multipath_rebuild(&first, &policy())
        .unwrap_or_else(|e| unreachable!("first signal should evaluate: {e}"));
    let decision = runtime
        .evaluate_multipath_rebuild(&urgent, &policy())
        .unwrap_or_else(|e| unreachable!("urgent signal should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::AllowRebuild);
    assert_eq!(decision.reason, "urgent_failover");
    assert!(decision.rebuild_allowed);
    assert!(!decision.debounced);
}

#[test]
fn hard_safety_signal_fails_closed_without_debounce_delay() {
    let mut runtime = seeded_runtime();
    let first = soft_signal("demand_rebuild_recommended", 10, 0xfeed_face, 3, 1);
    let hard = hard_signal(1);

    runtime
        .evaluate_multipath_rebuild(&first, &policy())
        .unwrap_or_else(|e| unreachable!("first signal should evaluate: {e}"));
    let decision = runtime
        .evaluate_multipath_rebuild(&hard, &policy())
        .unwrap_or_else(|e| unreachable!("hard signal should evaluate: {e}"));

    assert_eq!(decision.action, MeshMultipathRebuildAction::FailClosed);
    assert_eq!(decision.reason, "hard_safety_fail_closed");
    assert!(!decision.rebuild_allowed);
    assert!(!decision.debounced);
}

#[test]
fn rebuild_control_diagnostics_are_aggregate_and_redacted() {
    let mut runtime = seeded_runtime();
    let signal = soft_signal("demand_rebuild_recommended", 7009, 0xdead_beef, 5, 1);

    let decision = runtime
        .evaluate_multipath_rebuild(&signal, &policy())
        .unwrap_or_else(|e| unreachable!("signal should evaluate: {e}"));
    let explain = decision.explain.join("|");
    let debug_signal = format!("{signal:?}");
    let debug_decision = format!("{decision:?}");

    assert!(explain.contains("multipath_rebuild_privacy=aggregate_only"));
    assert!(explain.contains("multipath_rebuild_generation_changed=true"));
    assert!(!explain.contains("dead_beef"));
    assert!(!explain.contains("0xdead"));
    assert!(!explain.contains("198.51.100.31"));
    assert!(!explain.contains("node-a"));
    assert!(!explain.contains("7009"));
    assert!(!debug_signal.contains("dead_beef"));
    assert!(!debug_signal.contains("0xdead"));
    assert!(debug_signal.contains("<redacted>"));
    assert!(!debug_decision.contains("dead_beef"));
    assert!(!debug_decision.contains("198.51.100.31"));
}

#[test]
fn runtime_debug_redacts_rebuild_state_fingerprint() {
    let mut runtime = seeded_runtime();
    let signal = soft_signal("demand_rebuild_recommended", 7009, 0xdead_beef, 5, 1);

    runtime
        .evaluate_multipath_rebuild(&signal, &policy())
        .unwrap_or_else(|e| unreachable!("signal should evaluate: {e}"));
    let debug = format!("{runtime:?}");

    assert!(debug.contains("schedule_fingerprint"));
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("dead_beef"));
    assert!(!debug.contains("3735928559"));
    assert!(!debug.contains("198.51.100.31"));
    assert!(!debug.contains("node-a"));
}

#[test]
fn invalid_rebuild_policy_and_reason_are_rejected() {
    assert!(MeshMultipathRebuildPolicy::new(0, 4).is_err());
    assert!(MeshMultipathRebuildPolicy::new(3, 0).is_err());
    assert!(
        MeshMultipathRebuildSignal::soft("Raw Endpoint 198.51.100.31:443", 1, 1, 1, 1).is_err()
    );
    assert!(MeshMultipathRebuildSignal::soft(" capacity_pressure", 1, 1, 1, 1).is_err());
    assert!(MeshMultipathRebuildSignal::soft("capacity_pressure\n", 1, 1, 1, 1).is_err());
    assert!(MeshMultipathRebuildSignal::soft("route_7009", 1, 1, 1, 1).is_err());
    assert!(MeshMultipathRebuildSignal::soft("peer_123", 1, 1, 1, 1).is_err());
    assert!(MeshMultipathRebuildSignal::soft("dead_beef", 1, 1, 1, 1).is_err());
    assert!(MeshMultipathRebuildSignal::soft("payload_secret", 1, 1, 1, 1).is_err());
    assert!(MeshMultipathRebuildSignal::soft("capacity_pressure", 1, 1, 1, 2).is_ok());
}
