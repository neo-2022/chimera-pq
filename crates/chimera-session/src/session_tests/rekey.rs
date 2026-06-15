use crate::{RekeyPolicy, RekeyReason, RekeyState};

#[test]
fn rekey_policy_rejects_zero_thresholds() {
    assert!(
        RekeyPolicy {
            max_session_age_seconds: 0,
            max_packets_per_key: 1
        }
        .validate()
        .is_err()
    );
    assert!(
        RekeyPolicy {
            max_session_age_seconds: 1,
            max_packets_per_key: 0
        }
        .validate()
        .is_err()
    );
}

#[test]
fn rekey_triggers_by_packet_count() {
    let mut state = match RekeyState::new(
        RekeyPolicy {
            max_session_age_seconds: 60,
            max_packets_per_key: 3,
        },
        100,
    ) {
        Ok(state) => state,
        Err(error) => unreachable!("rekey state should be created: {error}"),
    };

    assert!(!state.should_rekey(100));
    state.on_packet_sent();
    state.on_packet_sent();
    assert!(!state.should_rekey(100));
    state.on_packet_sent();
    assert!(state.should_rekey(100));
    assert_eq!(
        state.rekey_reason(100),
        Some(RekeyReason::PacketLimitExceeded)
    );
}

#[test]
fn rekey_triggers_by_session_age() {
    let state = match RekeyState::new(
        RekeyPolicy {
            max_session_age_seconds: 10,
            max_packets_per_key: 100,
        },
        500,
    ) {
        Ok(state) => state,
        Err(error) => unreachable!("rekey state should be created: {error}"),
    };

    assert!(!state.should_rekey(509));
    assert!(state.should_rekey(510));
    assert!(state.should_rekey(511));
    assert_eq!(
        state.rekey_reason(510),
        Some(RekeyReason::SessionAgeExceeded)
    );
}

#[test]
fn rekey_reset_clears_triggers() {
    let mut state = match RekeyState::new(
        RekeyPolicy {
            max_session_age_seconds: 5,
            max_packets_per_key: 2,
        },
        10,
    ) {
        Ok(state) => state,
        Err(error) => unreachable!("rekey state should be created: {error}"),
    };

    state.on_packet_sent();
    state.on_packet_sent();
    assert!(state.should_rekey(12));

    state.reset_after_rekey(12);
    assert!(!state.should_rekey(12));
    assert!(!state.should_rekey(16));
    assert!(state.should_rekey(17));
}
