use super::super::CandidateSlot;

pub(super) fn select_with_region_diversity<'a>(
    candidates: Vec<CandidateSlot<'a>>,
    max_peers: usize,
    max_selected_per_region: usize,
) -> (Vec<CandidateSlot<'a>>, usize) {
    let mut selected: Vec<CandidateSlot<'a>> = Vec::with_capacity(max_peers.min(candidates.len()));
    let mut used_regions: Vec<String> = Vec::with_capacity(candidates.len());
    let mut backlog: Vec<CandidateSlot<'a>> = Vec::with_capacity(candidates.len());
    let mut region_counts: Vec<(String, usize)> = Vec::with_capacity(candidates.len());
    let mut region_cap_rejections = 0usize;

    for candidate in candidates {
        if selected.len() >= max_peers {
            break;
        }
        let region_key = candidate.normalized_region.as_str();
        if !try_reserve_region_slot(&mut region_counts, region_key, max_selected_per_region) {
            region_cap_rejections = region_cap_rejections.saturating_add(1);
            continue;
        }
        if used_regions.iter().any(|region| region == region_key) {
            release_region_slot(&mut region_counts, region_key);
            backlog.push(candidate);
        } else {
            used_regions.push(region_key.to_owned());
            selected.push(candidate);
        }
    }

    if selected.len() < max_peers {
        for candidate in backlog {
            if selected.len() >= max_peers {
                break;
            }
            if !try_reserve_region_slot(
                &mut region_counts,
                &candidate.normalized_region,
                max_selected_per_region,
            ) {
                region_cap_rejections = region_cap_rejections.saturating_add(1);
                continue;
            }
            selected.push(candidate);
        }
    }
    (selected, region_cap_rejections)
}

pub(super) fn select_by_score_with_region_cap<'a>(
    candidates: Vec<CandidateSlot<'a>>,
    max_peers: usize,
    max_selected_per_region: usize,
) -> (Vec<CandidateSlot<'a>>, usize) {
    let mut selected: Vec<CandidateSlot<'a>> = Vec::with_capacity(max_peers.min(candidates.len()));
    let mut region_counts: Vec<(String, usize)> = Vec::with_capacity(candidates.len());
    let mut region_cap_rejections = 0usize;

    for candidate in candidates {
        if selected.len() >= max_peers {
            break;
        }
        let region_key = candidate.normalized_region.as_str();
        if !try_reserve_region_slot(&mut region_counts, region_key, max_selected_per_region) {
            region_cap_rejections = region_cap_rejections.saturating_add(1);
            continue;
        }
        selected.push(candidate);
    }
    (selected, region_cap_rejections)
}

fn try_reserve_region_slot(
    region_counts: &mut Vec<(String, usize)>,
    region_key: &str,
    max_selected_per_region: usize,
) -> bool {
    if let Some((_, count)) = region_counts
        .iter_mut()
        .find(|(region, _)| region.as_str() == region_key)
    {
        if *count >= max_selected_per_region {
            return false;
        }
        *count += 1;
        true
    } else {
        region_counts.push((region_key.to_owned(), 1));
        true
    }
}

fn release_region_slot(region_counts: &mut [(String, usize)], region_key: &str) {
    if let Some((_, count)) = region_counts
        .iter_mut()
        .find(|(region, _)| region.as_str() == region_key)
    {
        *count = count.saturating_sub(1);
    }
}
