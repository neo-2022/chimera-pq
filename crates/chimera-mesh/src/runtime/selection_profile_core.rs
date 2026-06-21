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
    let performance_score = peer_performance_score(peer);
    match profile {
        MeshPathProfile::Fast => (peer.reliability_score as i32)
            .saturating_sub(peer.load_score as i32 * 2)
            .saturating_add(performance_score.saturating_mul(2)),
        MeshPathProfile::Balanced => peer_priority(peer).saturating_add(performance_score),
        MeshPathProfile::Resilient => (peer.reliability_score as i32 * 3)
            .saturating_sub(peer.load_score as i32)
            .saturating_add(performance_score / 2),
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

fn peer_performance_score(peer: &MeshPeerState) -> i32 {
    let throughput_score = peer
        .throughput_mbps
        .map(|mbps| mbps.min(1_000) as i32 / 5)
        .unwrap_or(0);
    let latency_score = peer
        .latency_ms
        .map(|ms| 200_i32.saturating_sub(ms.min(1_000) as i32 / 5))
        .unwrap_or(0);
    throughput_score.saturating_add(latency_score)
}

#[cfg(test)]
mod tests {
    use super::{MeshPathProfile, MeshPeerState, score_for_profile};

    fn peer(latency_ms: Option<u32>, throughput_mbps: Option<u32>) -> MeshPeerState {
        MeshPeerState {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            reliability_score: 90,
            load_score: 20,
            latency_ms,
            throughput_mbps,
            selection_score: 0,
        }
    }

    #[test]
    fn fast_profile_uses_explicit_performance_signals() {
        let slow = peer(Some(250), Some(40));
        let fast = peer(Some(30), Some(400));

        assert!(
            score_for_profile(&fast, MeshPathProfile::Fast)
                > score_for_profile(&slow, MeshPathProfile::Fast)
        );
    }

    #[test]
    fn missing_performance_preserves_legacy_balanced_score() {
        let peer = peer(None, None);

        assert_eq!(
            score_for_profile(&peer, MeshPathProfile::Balanced),
            (peer.reliability_score as i32 * 2) - peer.load_score as i32
        );
    }
}
