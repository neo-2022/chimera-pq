use super::MeshPeerState;

pub(super) fn active_lane_weights(active_peers: &[MeshPeerState]) -> Vec<u8> {
    if active_peers.is_empty() {
        return Vec::new();
    }
    let scores: Vec<u16> = active_peers.iter().map(lane_weight_score).collect();
    weights_from_scores(&scores, 100)
}

pub(super) fn capacity_weights_from_relative_weights(
    relative_weights: &[u8],
    capacity_budget_pct: u8,
) -> Vec<u8> {
    let scores: Vec<u16> = relative_weights
        .iter()
        .map(|weight| u16::from(*weight))
        .collect();
    weights_from_scores(&scores, capacity_budget_pct)
}

fn lane_weight_score(peer: &MeshPeerState) -> u16 {
    let reliability = peer.reliability_score.max(1) as u16;
    let load_headroom = 100_u16.saturating_sub(peer.load_score as u16).max(1);
    let selected_score = peer.selection_score.max(0) as u16;
    let performance_score = lane_performance_score(peer);
    reliability
        .saturating_add(load_headroom)
        .saturating_add(selected_score / 4)
        .saturating_add(performance_score)
        .max(1)
}

fn lane_performance_score(peer: &MeshPeerState) -> u16 {
    let throughput_score = peer
        .throughput_mbps
        .map(|mbps| (mbps.min(1_000) / 5) as u16)
        .unwrap_or(0);
    let latency_score = peer
        .latency_ms
        .map(|ms| 200_u16.saturating_sub((ms.min(1_000) / 5) as u16))
        .unwrap_or(0);
    throughput_score.saturating_add(latency_score)
}

fn weights_from_scores(scores: &[u16], target_pct: u8) -> Vec<u8> {
    if scores.is_empty() {
        return Vec::new();
    }
    if target_pct == 0 {
        return vec![0; scores.len()];
    }
    let total: u16 = scores.iter().sum();
    if total == 0 {
        return even_weights(scores.len(), target_pct);
    }

    let min_one = scores.len() <= target_pct as usize;
    let target = usize::from(target_pct);
    let total = usize::from(total);
    let mut weights: Vec<u8> = scores
        .iter()
        .map(|score| {
            let mut weight = (usize::from(*score) * target) / total;
            if min_one && *score > 0 {
                weight = weight.max(1);
            }
            weight.min(usize::from(u8::MAX)) as u8
        })
        .collect();
    normalize_weights_to_target(&mut weights, target_pct, min_one);
    weights
}

fn even_weights(count: usize, target_pct: u8) -> Vec<u8> {
    let target = usize::from(target_pct);
    let base = target / count;
    let remainder = target % count;
    (0..count)
        .map(|idx| {
            let extra = usize::from(idx < remainder);
            (base + extra).min(usize::from(u8::MAX)) as u8
        })
        .collect()
}

fn normalize_weights_to_target(weights: &mut [u8], target_pct: u8, min_one: bool) {
    let sum: i16 = weights.iter().map(|weight| *weight as i16).sum();
    let delta = i16::from(target_pct).saturating_sub(sum);
    if delta == 0 || weights.is_empty() {
        return;
    }
    if delta > 0 {
        if let Some(first) = weights.first_mut() {
            *first = first.saturating_add(delta as u8);
        }
        return;
    }
    let min_value = u8::from(min_one);
    let mut remaining = delta.unsigned_abs() as u8;
    for weight in weights.iter_mut().rev() {
        if remaining == 0 {
            break;
        }
        let removable = weight.saturating_sub(min_value).min(remaining);
        *weight = weight.saturating_sub(removable);
        remaining = remaining.saturating_sub(removable);
    }
}

#[cfg(test)]
mod tests {
    use super::active_lane_weights;
    use crate::model::MeshPeerState;

    fn peer(latency_ms: Option<u32>, throughput_mbps: Option<u32>) -> MeshPeerState {
        MeshPeerState {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            reliability_score: 90,
            load_score: 20,
            latency_ms,
            throughput_mbps,
            selection_score: 180,
        }
    }

    #[test]
    fn active_lane_weights_shift_capacity_toward_faster_peer() {
        let slow = peer(Some(250), Some(40));
        let fast = peer(Some(30), Some(400));
        let weights = active_lane_weights(&[slow, fast]);

        assert_eq!(
            weights.iter().map(|weight| u16::from(*weight)).sum::<u16>(),
            100
        );
        assert!(weights[1] > weights[0]);
    }

    #[test]
    fn active_lane_weights_preserve_evenish_legacy_without_performance() {
        let first = peer(None, None);
        let second = peer(None, None);
        let weights = active_lane_weights(&[first, second]);

        assert_eq!(weights, vec![50, 50]);
    }
}
