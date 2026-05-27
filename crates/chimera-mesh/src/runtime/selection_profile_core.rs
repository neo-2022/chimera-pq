use super::*;

pub(super) fn resolve_path_profile(
    override_profile: Option<MeshPathProfile>,
    profile_state: &MeshProfileState,
    current_tick: u64,
    profile_hysteresis_ticks: u64,
    avg_load_score: u8,
    avg_reliability_score: u8,
) -> (MeshPathProfile, &'static str) {
    if let Some(profile) = override_profile {
        return (profile, "manual_override");
    }
    if profile_state.active_profile == MeshPathProfile::Resilient {
        let Some(clear_tick) = profile_state.degrade_cleared_since_tick else {
            return (MeshPathProfile::Resilient, "auto:degraded_active");
        };
        let elapsed = current_tick.saturating_sub(clear_tick);
        if elapsed < profile_hysteresis_ticks {
            return (MeshPathProfile::Resilient, "auto:hysteresis_hold");
        }
    }
    if avg_load_score <= 15 && avg_reliability_score >= 95 {
        return (MeshPathProfile::Fast, "auto:fast_signals");
    }
    (MeshPathProfile::Balanced, "auto:balanced_signals")
}

pub(super) fn score_for_profile(peer: &MeshPeerState, profile: MeshPathProfile) -> i32 {
    match profile {
        MeshPathProfile::Fast => (peer.reliability_score as i32) - (peer.load_score as i32 * 2),
        MeshPathProfile::Balanced => peer_priority(peer),
        MeshPathProfile::Resilient => (peer.reliability_score as i32 * 3) - peer.load_score as i32,
    }
}

pub(super) fn profile_label(profile: MeshPathProfile) -> &'static str {
    match profile {
        MeshPathProfile::Fast => "fast",
        MeshPathProfile::Balanced => "balanced",
        MeshPathProfile::Resilient => "resilient",
    }
}

pub(super) fn runtime_peer_signal_averages(peers: &BTreeMap<String, MeshPeerState>) -> (u8, u8) {
    if peers.is_empty() {
        return (100, 0);
    }
    let sum_load: usize = peers.values().map(|peer| peer.load_score as usize).sum();
    let sum_reliability: usize = peers
        .values()
        .map(|peer| peer.reliability_score as usize)
        .sum();
    let count = peers.len();
    let avg_load = (sum_load / count) as u8;
    let avg_reliability = (sum_reliability / count) as u8;
    (avg_load, avg_reliability)
}
