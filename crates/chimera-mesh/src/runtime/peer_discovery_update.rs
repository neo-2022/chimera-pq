use super::*;

pub(super) struct ExistingPeerUpdateContext {
    pub(super) unchanged_record: bool,
    pub(super) existing_score: i32,
    pub(super) incoming_score: i32,
    pub(super) degraded: bool,
    pub(super) effective_replacement_min_score_delta: i32,
    pub(super) previous_updates: u64,
    pub(super) previous_replacements: u64,
    pub(super) previous_holds: u64,
    pub(super) previous_degraded: u64,
    pub(super) previous_churn_blocks: u64,
    pub(super) previous_threshold_blocks: u64,
    pub(super) previous_identity_marker: u64,
    pub(super) previous_endpoint_generation: u64,
    pub(super) previous_update_bootstrap_url: Option<String>,
    pub(super) preserve_existing_endpoint: bool,
}

pub(super) fn existing_peer_update_context(
    runtime: &MeshRuntime,
    record: &MeshDiscoveryRecord,
    previous_meta: Option<&MeshPeerMeta>,
) -> Option<ExistingPeerUpdateContext> {
    let existing = runtime.peers.get(&record.node_id)?;
    let existing_score = peer_priority(existing);
    let incoming_score = (record.reliability_score as i32 * 2) - record.load_score as i32;
    let degraded = runtime
        .health_state
        .get(&record.node_id)
        .map(|meta| !meta.health.healthy || meta.health.cooldown_active)
        .unwrap_or(false);
    let effective_replacement_min_score_delta = if degraded {
        runtime.table_policy.degraded_replacement_min_score_delta
    } else {
        runtime.table_policy.replacement_min_score_delta
    };
    let age_since_seen =
        previous_meta.map_or(0, |meta| runtime.tick.saturating_sub(meta.last_seen_tick));
    let within_window = age_since_seen <= runtime.table_policy.stability_window_ticks;
    let preserve_existing_endpoint = previous_meta
        .is_some_and(|meta| meta.endpoint_generation > 0 && existing.endpoint != record.endpoint);
    let unchanged_record = previous_meta.is_some()
        && within_window
        && usable_peer_metadata_matches(existing, record, preserve_existing_endpoint);
    Some(ExistingPeerUpdateContext {
        unchanged_record,
        existing_score,
        incoming_score,
        degraded,
        effective_replacement_min_score_delta,
        previous_updates: if within_window {
            previous_meta.map_or(0, |meta| meta.update_events)
        } else {
            0
        },
        previous_replacements: if within_window {
            previous_meta.map_or(0, |meta| meta.replacement_events)
        } else {
            0
        },
        previous_holds: if within_window {
            previous_meta.map_or(0, |meta| meta.hold_events)
        } else {
            0
        },
        previous_degraded: if within_window {
            previous_meta.map_or(0, |meta| meta.degraded_events)
        } else {
            0
        },
        previous_churn_blocks: if within_window {
            previous_meta.map_or(0, |meta| meta.churn_block_events)
        } else {
            0
        },
        previous_threshold_blocks: if within_window {
            previous_meta.map_or(0, |meta| meta.threshold_block_events)
        } else {
            0
        },
        previous_identity_marker: previous_meta.map_or(0, |meta| meta.identity_marker),
        previous_endpoint_generation: previous_meta.map_or(0, |meta| meta.endpoint_generation),
        previous_update_bootstrap_url: previous_meta
            .and_then(|meta| meta.update_bootstrap_url.clone()),
        preserve_existing_endpoint,
    })
}

fn usable_peer_metadata_matches(
    existing: &MeshPeerState,
    record: &MeshDiscoveryRecord,
    preserve_existing_endpoint: bool,
) -> bool {
    (preserve_existing_endpoint || existing.endpoint == record.endpoint)
        && existing.region == record.region
        && existing.reliability_score == record.reliability_score
        && existing.load_score == record.load_score
}

pub(super) fn apply_existing_peer_update(
    runtime: &mut MeshRuntime,
    record: &MeshDiscoveryRecord,
    ctx: ExistingPeerUpdateContext,
) -> bool {
    if ctx.unchanged_record {
        if let Some(meta) = runtime.peer_meta.get_mut(&record.node_id) {
            meta.last_seen_tick = runtime.tick;
        }
        return false;
    }

    let score_gain = ctx.incoming_score.saturating_sub(ctx.existing_score);
    let churn_replacement_allowed =
        ctx.previous_replacements < runtime.table_policy.max_replacements_per_window;

    if score_gain >= ctx.effective_replacement_min_score_delta && churn_replacement_allowed {
        if let Some(existing) = runtime.peers.get_mut(&record.node_id) {
            if !ctx.preserve_existing_endpoint {
                existing.endpoint = record.endpoint.clone();
            }
            existing.region = record.region.clone();
            existing.reliability_score = record.reliability_score;
            existing.load_score = record.load_score;
        }
        runtime.peer_meta.insert(
            record.node_id.clone(),
            MeshPeerMeta {
                identity_marker: ctx.previous_identity_marker,
                last_seen_tick: runtime.tick,
                update_events: ctx.previous_updates.saturating_add(1),
                replacement_events: ctx.previous_replacements.saturating_add(1),
                hold_events: ctx.previous_holds,
                degraded_events: ctx
                    .previous_degraded
                    .saturating_add(u64::from(ctx.degraded)),
                churn_block_events: ctx.previous_churn_blocks,
                threshold_block_events: ctx.previous_threshold_blocks,
                last_effective_replacement_threshold: ctx.effective_replacement_min_score_delta,
                endpoint_generation: ctx.previous_endpoint_generation,
                update_bootstrap_url: ctx.previous_update_bootstrap_url.clone(),
            },
        );
        return true;
    }

    let blocked_by_churn =
        score_gain >= ctx.effective_replacement_min_score_delta && !churn_replacement_allowed;
    let blocked_by_threshold = score_gain < ctx.effective_replacement_min_score_delta;
    runtime.peer_meta.insert(
        record.node_id.clone(),
        MeshPeerMeta {
            identity_marker: ctx.previous_identity_marker,
            last_seen_tick: runtime.tick,
            update_events: ctx.previous_updates.saturating_add(1),
            replacement_events: ctx.previous_replacements,
            hold_events: ctx.previous_holds.saturating_add(1),
            degraded_events: ctx
                .previous_degraded
                .saturating_add(u64::from(ctx.degraded)),
            churn_block_events: ctx
                .previous_churn_blocks
                .saturating_add(u64::from(blocked_by_churn)),
            threshold_block_events: ctx
                .previous_threshold_blocks
                .saturating_add(u64::from(blocked_by_threshold)),
            last_effective_replacement_threshold: ctx.effective_replacement_min_score_delta,
            endpoint_generation: ctx.previous_endpoint_generation,
            update_bootstrap_url: ctx.previous_update_bootstrap_url,
        },
    );
    true
}
