use super::*;
pub(crate) fn apply_discovery_update(
    existing: &mut MeshPeerState,
    record: &MeshDiscoveryRecord,
    previous_meta: Option<&MeshPeerMeta>,
    degraded: bool,
    table_policy: &MeshPeerTablePolicy,
    tick: u64,
) -> MeshPeerMeta {
    let existing_score = peer_priority(existing);
    let incoming_score = (record.reliability_score as i32 * 2) - record.load_score as i32;
    let score_gain = incoming_score.saturating_sub(existing_score);
    let effective_replacement_min_score_delta = if degraded {
        table_policy.degraded_replacement_min_score_delta
    } else {
        table_policy.replacement_min_score_delta
    };
    let previous =
        windowed_peer_meta_counters(previous_meta, tick, table_policy.stability_window_ticks);
    let churn_replacement_allowed =
        previous.replacements < table_policy.max_replacements_per_window;

    if score_gain >= effective_replacement_min_score_delta && churn_replacement_allowed {
        existing.endpoint = record.endpoint.clone();
        existing.region = record.region.clone();
        existing.reliability_score = record.reliability_score;
        existing.load_score = record.load_score;
        MeshPeerMeta {
            last_seen_tick: tick,
            update_events: previous.updates.saturating_add(1),
            replacement_events: previous.replacements.saturating_add(1),
            hold_events: previous.holds,
            degraded_events: previous.degraded.saturating_add(u64::from(degraded)),
            churn_block_events: previous.churn_blocks,
            threshold_block_events: previous.threshold_blocks,
            last_effective_replacement_threshold: effective_replacement_min_score_delta,
        }
    } else {
        let blocked_by_churn =
            score_gain >= effective_replacement_min_score_delta && !churn_replacement_allowed;
        let blocked_by_threshold = score_gain < effective_replacement_min_score_delta;
        MeshPeerMeta {
            last_seen_tick: tick,
            update_events: previous.updates.saturating_add(1),
            replacement_events: previous.replacements,
            hold_events: previous.holds.saturating_add(1),
            degraded_events: previous.degraded.saturating_add(u64::from(degraded)),
            churn_block_events: previous
                .churn_blocks
                .saturating_add(u64::from(blocked_by_churn)),
            threshold_block_events: previous
                .threshold_blocks
                .saturating_add(u64::from(blocked_by_threshold)),
            last_effective_replacement_threshold: effective_replacement_min_score_delta,
        }
    }
}

pub(crate) fn windowed_peer_meta_counters(
    previous_meta: Option<&MeshPeerMeta>,
    tick: u64,
    stability_window_ticks: u64,
) -> WindowedPeerMetaCounters {
    let age_since_seen = previous_meta.map_or(0, |meta| tick.saturating_sub(meta.last_seen_tick));
    let within_window = age_since_seen <= stability_window_ticks;
    if within_window {
        WindowedPeerMetaCounters {
            updates: previous_meta.map_or(0, |meta| meta.update_events),
            replacements: previous_meta.map_or(0, |meta| meta.replacement_events),
            holds: previous_meta.map_or(0, |meta| meta.hold_events),
            degraded: previous_meta.map_or(0, |meta| meta.degraded_events),
            churn_blocks: previous_meta.map_or(0, |meta| meta.churn_block_events),
            threshold_blocks: previous_meta.map_or(0, |meta| meta.threshold_block_events),
        }
    } else {
        WindowedPeerMetaCounters {
            updates: 0,
            replacements: 0,
            holds: 0,
            degraded: 0,
            churn_blocks: 0,
            threshold_blocks: 0,
        }
    }
}

pub(crate) fn insert_new_discovery_peer(
    peers: &mut BTreeMap<String, MeshPeerState>,
    peer_meta: &mut BTreeMap<String, MeshPeerMeta>,
    record: &MeshDiscoveryRecord,
    tick: u64,
    replacement_min_score_delta: i32,
) {
    peers.insert(
        record.node_id.clone(),
        MeshPeerState {
            node_id: record.node_id.clone(),
            endpoint: record.endpoint.clone(),
            region: record.region.clone(),
            reliability_score: record.reliability_score,
            load_score: record.load_score,
            selection_score: 0,
        },
    );
    peer_meta.insert(
        record.node_id.clone(),
        MeshPeerMeta {
            last_seen_tick: tick,
            update_events: 1,
            replacement_events: 0,
            hold_events: 0,
            degraded_events: 0,
            churn_block_events: 0,
            threshold_block_events: 0,
            last_effective_replacement_threshold: replacement_min_score_delta,
        },
    );
}

pub fn evaluate_join_mode(request: &MeshJoinRequest) -> Result<MeshJoinMode, String> {
    request.validate()?;

    match &request.invite_token {
        Some(token) if !token.trim().is_empty() => Ok(MeshJoinMode::InvitationOnly),
        _ => Ok(MeshJoinMode::PublicDiscovery),
    }
}
