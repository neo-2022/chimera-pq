use super::*;

pub(super) struct ExistingPeerUpdateContext {
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
    Some(ExistingPeerUpdateContext {
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
    })
}

pub(super) fn apply_existing_peer_update(
    runtime: &mut MeshRuntime,
    record: &MeshDiscoveryRecord,
    ctx: ExistingPeerUpdateContext,
) {
    let score_gain = ctx.incoming_score.saturating_sub(ctx.existing_score);
    let churn_replacement_allowed =
        ctx.previous_replacements < runtime.table_policy.max_replacements_per_window;

    if score_gain >= ctx.effective_replacement_min_score_delta && churn_replacement_allowed {
        if let Some(existing) = runtime.peers.get_mut(&record.node_id) {
            existing.endpoint = record.endpoint.clone();
            existing.region = record.region.clone();
            existing.reliability_score = record.reliability_score;
            existing.load_score = record.load_score;
        }
        runtime.peer_meta.insert(
            record.node_id.clone(),
            MeshPeerMeta {
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
            },
        );
        return;
    }

    let blocked_by_churn =
        score_gain >= ctx.effective_replacement_min_score_delta && !churn_replacement_allowed;
    let blocked_by_threshold = score_gain < ctx.effective_replacement_min_score_delta;
    runtime.peer_meta.insert(
        record.node_id.clone(),
        MeshPeerMeta {
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
        },
    );
}
