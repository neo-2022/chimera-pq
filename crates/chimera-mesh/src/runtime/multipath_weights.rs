use super::MeshPeerState;

pub(super) fn active_lane_weights(active_peers: &[&MeshPeerState]) -> Vec<u8> {
    if active_peers.is_empty() {
        return Vec::new();
    }
    let scores: Vec<u16> = active_peers
        .iter()
        .map(|peer| lane_weight_score(peer))
        .collect();
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
    reliability
        .saturating_add(load_headroom)
        .saturating_add(selected_score / 4)
        .max(1)
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
