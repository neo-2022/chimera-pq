use super::*;

pub(super) struct PeerMaintenanceComputation {
    pub(super) drop_set: BTreeSet<String>,
    pub(super) dropped_by_region_cap: usize,
    pub(super) dropped_by_global_cap: usize,
    pub(super) protected_region_skips: usize,
    pub(super) effective_profile: MeshPathProfile,
    pub(super) effective_target_distinct_regions: usize,
    pub(super) effective_target_source: &'static str,
}

pub(super) fn compute_enforcement(
    peers: &BTreeMap<String, MeshPeerState>,
    profile_state: &MeshProfileState,
    tick: u64,
    table_policy: &MeshPeerTablePolicy,
) -> PeerMaintenanceComputation {
    let (avg_load_score, avg_reliability_score) = runtime_peer_signal_averages(peers);
    let (effective_profile, _) = resolve_path_profile(
        None,
        profile_state,
        tick,
        table_policy.profile_hysteresis_ticks,
        avg_load_score,
        avg_reliability_score,
    );
    let (effective_target_distinct_regions, effective_target_source) =
        effective_target_distinct_regions(
            table_policy.target_distinct_regions,
            table_policy.max_entries,
            effective_profile,
        );

    let mut region_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut all: Vec<(String, i32)> = peers
        .iter()
        .map(|(node_id, peer)| (node_id.clone(), peer_priority(peer)))
        .collect();
    all.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

    let mut drop_set: BTreeSet<String> = BTreeSet::new();
    let mut dropped_by_region_cap = 0usize;
    let mut dropped_by_global_cap = 0usize;
    let mut protected_region_skips = 0usize;

    for (node_id, _) in &all {
        let Some(peer) = peers.get(node_id) else {
            continue;
        };
        let count = region_counts
            .entry(normalize_region_key(&peer.region))
            .or_insert(0);
        *count += 1;
        if *count > table_policy.max_entries_per_region && drop_set.insert(node_id.clone()) {
            dropped_by_region_cap = dropped_by_region_cap.saturating_add(1);
        }
    }

    let mut kept = peers.len().saturating_sub(drop_set.len());
    if kept > table_policy.max_entries {
        let mut kept_by_region: BTreeMap<String, usize> = BTreeMap::new();
        for node_id in peers.keys() {
            if drop_set.contains(node_id) {
                continue;
            }
            let Some(peer) = peers.get(node_id) else {
                continue;
            };
            *kept_by_region
                .entry(normalize_region_key(&peer.region))
                .or_insert(0) += 1;
        }
        let mut distinct_regions = kept_by_region.len();
        while kept > table_policy.max_entries {
            let mut chosen: Option<String> = None;
            for (node_id, _) in &all {
                if drop_set.contains(node_id) {
                    continue;
                }
                let Some(peer) = peers.get(node_id) else {
                    continue;
                };
                let region = normalize_region_key(&peer.region);
                let count = kept_by_region.get(&region).copied().unwrap_or(0);
                let would_remove_last_region = count == 1;
                if would_remove_last_region && distinct_regions <= effective_target_distinct_regions
                {
                    protected_region_skips = protected_region_skips.saturating_add(1);
                    continue;
                }
                chosen = Some(node_id.clone());
                break;
            }
            let Some(node_id) = chosen.or_else(|| {
                all.iter()
                    .map(|(id, _)| id)
                    .find(|id| !drop_set.contains(*id))
                    .cloned()
            }) else {
                break;
            };
            if drop_set.insert(node_id.clone()) {
                kept = kept.saturating_sub(1);
                dropped_by_global_cap = dropped_by_global_cap.saturating_add(1);
                if let Some(peer) = peers.get(&node_id) {
                    let region = normalize_region_key(&peer.region);
                    if let Some(count) = kept_by_region.get_mut(&region) {
                        if *count > 1 {
                            *count -= 1;
                        } else {
                            kept_by_region.remove(&region);
                            distinct_regions = distinct_regions.saturating_sub(1);
                        }
                    }
                }
            }
        }
    }

    PeerMaintenanceComputation {
        drop_set,
        dropped_by_region_cap,
        dropped_by_global_cap,
        protected_region_skips,
        effective_profile,
        effective_target_distinct_regions,
        effective_target_source,
    }
}
