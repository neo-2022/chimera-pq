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

struct EnforcementCandidate<'a> {
    node_id: &'a str,
    region_index: usize,
    priority: i32,
    dropped: bool,
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

    let mut drop_set: BTreeSet<String> = BTreeSet::new();
    let mut dropped_by_region_cap = 0usize;
    let mut dropped_by_global_cap = 0usize;
    let mut protected_region_skips = 0usize;

    if peers.len() <= table_policy.max_entries && peers.len() <= table_policy.max_entries_per_region
    {
        return PeerMaintenanceComputation {
            drop_set,
            dropped_by_region_cap,
            dropped_by_global_cap,
            protected_region_skips,
            effective_profile,
            effective_target_distinct_regions,
            effective_target_source,
        };
    }

    let mut region_index_by_key: BTreeMap<String, usize> = BTreeMap::new();
    let mut all: Vec<EnforcementCandidate<'_>> = Vec::with_capacity(peers.len());
    for (node_id, peer) in peers {
        let next_region_index = region_index_by_key.len();
        let region_index = match region_index_by_key.entry(normalize_region_key(&peer.region)) {
            std::collections::btree_map::Entry::Occupied(entry) => *entry.get(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(next_region_index);
                next_region_index
            }
        };
        all.push(EnforcementCandidate {
            node_id,
            region_index,
            priority: peer_priority(peer),
            dropped: false,
        });
    }
    all.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.node_id.cmp(b.node_id))
    });

    let mut region_counts = vec![0usize; region_index_by_key.len()];
    for candidate in &mut all {
        let count = &mut region_counts[candidate.region_index];
        *count += 1;
        if *count > table_policy.max_entries_per_region && !candidate.dropped {
            candidate.dropped = true;
            drop_set.insert(candidate.node_id.to_string());
            dropped_by_region_cap = dropped_by_region_cap.saturating_add(1);
        }
    }

    let mut kept = peers.len().saturating_sub(drop_set.len());
    if kept > table_policy.max_entries {
        let mut kept_by_region = vec![0usize; region_index_by_key.len()];
        for candidate in &all {
            if !candidate.dropped {
                kept_by_region[candidate.region_index] =
                    kept_by_region[candidate.region_index].saturating_add(1);
            }
        }
        let mut distinct_regions = kept_by_region.iter().filter(|count| **count > 0).count();
        while kept > table_policy.max_entries {
            let mut chosen: Option<usize> = None;
            for (index, candidate) in all.iter().enumerate() {
                if candidate.dropped {
                    continue;
                }
                let count = kept_by_region[candidate.region_index];
                let would_remove_last_region = count == 1;
                if would_remove_last_region && distinct_regions <= effective_target_distinct_regions
                {
                    protected_region_skips = protected_region_skips.saturating_add(1);
                    continue;
                }
                chosen = Some(index);
                break;
            }
            let Some(index) =
                chosen.or_else(|| all.iter().position(|candidate| !candidate.dropped))
            else {
                break;
            };
            let candidate = &mut all[index];
            if !candidate.dropped {
                candidate.dropped = true;
                drop_set.insert(candidate.node_id.to_string());
                kept = kept.saturating_sub(1);
                dropped_by_global_cap = dropped_by_global_cap.saturating_add(1);
                let count = &mut kept_by_region[candidate.region_index];
                if *count > 1 {
                    *count -= 1;
                } else if *count == 1 {
                    *count = 0;
                    distinct_regions = distinct_regions.saturating_sub(1);
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
