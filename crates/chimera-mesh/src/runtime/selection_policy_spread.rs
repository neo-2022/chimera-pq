use std::collections::BTreeMap;

use crate::model::MeshPeerState;
use crate::policy::MeshPathProfile;

use super::validate::normalize_region_key;

pub(super) fn apply_resilient_region_spread_bonus(
    candidates: &mut [MeshPeerState],
    profile: MeshPathProfile,
    weight: u8,
) -> (bool, i32) {
    if profile != MeshPathProfile::Resilient || candidates.is_empty() {
        return (false, 0);
    }
    let mut region_counts: BTreeMap<String, usize> = BTreeMap::new();
    for peer in &*candidates {
        *region_counts
            .entry(normalize_region_key(&peer.region))
            .or_insert(0) += 1;
    }
    let max_region_population = region_counts.values().copied().max().unwrap_or(1);
    let mut bonus_total = 0i32;
    for peer in &mut *candidates {
        let region_population = region_counts
            .get(&normalize_region_key(&peer.region))
            .copied()
            .unwrap_or(max_region_population);
        let rarity_delta = max_region_population.saturating_sub(region_population);
        let spread_bonus = (rarity_delta as i32) * i32::from(weight);
        peer.selection_score = peer.selection_score.saturating_add(spread_bonus);
        bonus_total = bonus_total.saturating_add(spread_bonus);
    }
    (bonus_total > 0, bonus_total)
}
