use std::collections::{BTreeMap, BTreeSet};

use crate::model::MeshPeerState;

use super::validate::normalize_region_key;

pub(super) fn select_with_region_diversity(
    candidates: Vec<MeshPeerState>,
    max_peers: usize,
    max_selected_per_region: usize,
) -> (Vec<MeshPeerState>, usize) {
    let mut selected: Vec<MeshPeerState> = Vec::new();
    let mut used_regions: BTreeSet<String> = BTreeSet::new();
    let mut backlog: Vec<MeshPeerState> = Vec::new();
    let mut region_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut region_cap_rejections = 0usize;

    for peer in candidates {
        if selected.len() >= max_peers {
            break;
        }
        let region_key = normalize_region_key(&peer.region);
        if !try_reserve_region_slot(&mut region_counts, &region_key, max_selected_per_region) {
            region_cap_rejections = region_cap_rejections.saturating_add(1);
            continue;
        }
        if used_regions.insert(region_key.clone()) {
            selected.push(peer);
        } else {
            release_region_slot(&mut region_counts, &region_key);
            backlog.push(peer);
        }
    }

    if selected.len() < max_peers {
        for peer in backlog {
            if selected.len() >= max_peers {
                break;
            }
            if !try_reserve_region_slot(
                &mut region_counts,
                &normalize_region_key(&peer.region),
                max_selected_per_region,
            ) {
                region_cap_rejections = region_cap_rejections.saturating_add(1);
                continue;
            }
            selected.push(peer);
        }
    }
    (selected, region_cap_rejections)
}

pub(super) fn select_by_score_with_region_cap(
    candidates: Vec<MeshPeerState>,
    max_peers: usize,
    max_selected_per_region: usize,
) -> (Vec<MeshPeerState>, usize) {
    let mut selected: Vec<MeshPeerState> = Vec::new();
    let mut region_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut region_cap_rejections = 0usize;

    for peer in candidates {
        if selected.len() >= max_peers {
            break;
        }
        if !try_reserve_region_slot(
            &mut region_counts,
            &normalize_region_key(&peer.region),
            max_selected_per_region,
        ) {
            region_cap_rejections = region_cap_rejections.saturating_add(1);
            continue;
        }
        selected.push(peer);
    }
    (selected, region_cap_rejections)
}

fn try_reserve_region_slot(
    region_counts: &mut BTreeMap<String, usize>,
    region_key: &str,
    max_selected_per_region: usize,
) -> bool {
    let count = region_counts.entry(region_key.to_string()).or_insert(0);
    if *count >= max_selected_per_region {
        return false;
    }
    *count += 1;
    true
}

fn release_region_slot(region_counts: &mut BTreeMap<String, usize>, region_key: &str) {
    if let Some(count) = region_counts.get_mut(region_key) {
        *count = count.saturating_sub(1);
    }
}
