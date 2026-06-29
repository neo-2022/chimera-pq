use crate::policy::MeshPathProfile;

use super::super::CandidateSlot;

pub(super) fn apply_resilient_region_spread_bonus(
    candidates: &mut [CandidateSlot<'_>],
    profile: MeshPathProfile,
    weight: u8,
) -> (bool, i32) {
    if profile != MeshPathProfile::Resilient || candidates.is_empty() {
        return (false, 0);
    }
    let mut region_counts: Vec<(String, usize)> = Vec::with_capacity(candidates.len());
    for candidate in &*candidates {
        let region_key = candidate.normalized_region.as_str();
        if let Some((_, count)) = region_counts
            .iter_mut()
            .find(|(region, _)| region.as_str() == region_key)
        {
            *count += 1;
        } else {
            region_counts.push((region_key.to_owned(), 1));
        }
    }
    let max_region_population = region_counts
        .iter()
        .map(|(_, count)| *count)
        .max()
        .unwrap_or(1);
    let mut bonus_total = 0_i32;
    for candidate in &mut *candidates {
        let region_population = region_counts
            .iter()
            .find(|(region, _)| region.as_str() == candidate.normalized_region.as_str())
            .map(|(_, count)| *count)
            .unwrap_or(max_region_population);
        let rarity_delta = max_region_population.saturating_sub(region_population);
        let spread_bonus = (rarity_delta as i32) * i32::from(weight);
        candidate.selection_score = candidate.selection_score.saturating_add(spread_bonus);
        bonus_total = bonus_total.saturating_add(spread_bonus);
    }
    (bonus_total > 0, bonus_total)
}
