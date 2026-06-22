use super::*;

const FINGERPRINT_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FINGERPRINT_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct MeshMultipathRebuildFingerprint(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MeshMultipathRebuildTriggerCause {
    PeerTableChanged,
    PeerHealthChanged,
    PeerPerformanceChanged,
    UrgentFailover,
}

impl MeshMultipathRebuildTriggerCause {
    fn reason(self) -> &'static str {
        match self {
            Self::PeerTableChanged => "peer_table_changed",
            Self::PeerHealthChanged => "peer_health_changed",
            Self::PeerPerformanceChanged => "peer_performance_changed",
            Self::UrgentFailover => "urgent_failover",
        }
    }
}

impl std::fmt::Debug for MeshMultipathRebuildFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

impl MeshRuntime {
    pub fn pending_multipath_rebuild_signal(&self) -> Option<&MeshMultipathRebuildSignal> {
        self.pending_multipath_rebuild.as_ref()
    }

    pub(crate) fn take_pending_multipath_rebuild_signal(
        &mut self,
    ) -> Option<MeshMultipathRebuildSignal> {
        self.pending_multipath_rebuild.take()
    }

    pub(super) fn restore_pending_multipath_rebuild_signal(
        &mut self,
        signal: MeshMultipathRebuildSignal,
    ) {
        self.pending_multipath_rebuild = Some(signal);
    }

    pub(super) fn rebuild_trigger_fingerprint(&self) -> MeshMultipathRebuildFingerprint {
        let mut state = FINGERPRINT_OFFSET;
        fold_u64(&mut state, self.peers.len() as u64);
        fold_u64(&mut state, self.health_state.len() as u64);
        fold_u64(&mut state, self.region_distribution().len() as u64);
        fold_u64(
            &mut state,
            self.last_table_enforcement_report.total_peers_after as u64,
        );
        fold_u64(
            &mut state,
            self.last_table_enforcement_report.dropped_total as u64,
        );
        fold_u64(
            &mut state,
            self.last_table_enforcement_report.dropped_by_region_cap as u64,
        );
        fold_u64(
            &mut state,
            self.last_table_enforcement_report.dropped_by_global_cap as u64,
        );

        let mut degraded_count = 0_u64;
        for meta in self.health_state.values() {
            if !meta.health.healthy || meta.health.cooldown_active {
                degraded_count = degraded_count.saturating_add(1);
            }
        }
        fold_u64(&mut state, degraded_count);

        for (region, count) in self.region_distribution() {
            fold_str(&mut state, &region);
            fold_u64(&mut state, count as u64);
        }

        for (idx, peer) in self.peers.values().enumerate() {
            fold_u64(&mut state, idx as u64);
            fold_str(&mut state, &peer.region);
            fold_u64(&mut state, u64::from(peer.reliability_score));
            fold_u64(&mut state, u64::from(peer.load_score));
            fold_u64(
                &mut state,
                peer.latency_ms.map_or(0, |ms| u64::from(ms.min(60_000))),
            );
            fold_u64(
                &mut state,
                peer.throughput_mbps
                    .map_or(0, |mbps| u64::from(mbps.min(1_000_000))),
            );
            let health_flags = self
                .health_state
                .get(&peer.node_id)
                .map(|meta| {
                    u64::from(!meta.health.healthy) | (u64::from(meta.health.cooldown_active) << 1)
                })
                .unwrap_or(0);
            fold_u64(&mut state, health_flags);
            if let Some(meta) = self.peer_meta.get(&peer.node_id) {
                fold_u64(&mut state, meta.identity_marker);
                fold_u64(&mut state, meta.update_events);
                fold_u64(&mut state, meta.replacement_events);
                fold_u64(&mut state, meta.hold_events);
                fold_u64(&mut state, meta.degraded_events);
                fold_u64(&mut state, meta.churn_block_events);
                fold_u64(&mut state, meta.threshold_block_events);
                fold_u64(
                    &mut state,
                    meta.last_effective_replacement_threshold.unsigned_abs() as u64,
                );
                fold_u64(
                    &mut state,
                    u64::from(meta.last_effective_replacement_threshold.is_negative()),
                );
            }
        }

        MeshMultipathRebuildFingerprint(state)
    }

    pub(super) fn mark_pending_multipath_rebuild(
        &mut self,
        cause: MeshMultipathRebuildTriggerCause,
        before: MeshMultipathRebuildFingerprint,
    ) -> Result<(), String> {
        let after = self.rebuild_trigger_fingerprint();
        if before == after {
            return Ok(());
        }
        let reason = cause.reason();
        let signal = if cause == MeshMultipathRebuildTriggerCause::UrgentFailover {
            MeshMultipathRebuildSignal::urgent_failover(
                reason, self.tick, after.0, self.tick, self.tick,
            )?
        } else {
            MeshMultipathRebuildSignal::soft(reason, self.tick, after.0, self.tick, self.tick)?
        };
        self.merge_pending_multipath_rebuild_signal(signal);
        Ok(())
    }

    fn merge_pending_multipath_rebuild_signal(&mut self, signal: MeshMultipathRebuildSignal) {
        let Some(existing) = self.pending_multipath_rebuild.as_ref() else {
            self.pending_multipath_rebuild = Some(signal);
            return;
        };
        if rebuild_signal_priority(signal.urgency()) >= rebuild_signal_priority(existing.urgency())
        {
            self.pending_multipath_rebuild = Some(signal);
        }
    }
}

fn rebuild_signal_priority(urgency: MeshMultipathRebuildUrgency) -> u8 {
    match urgency {
        MeshMultipathRebuildUrgency::Soft => 0,
        MeshMultipathRebuildUrgency::UrgentFailover => 1,
        MeshMultipathRebuildUrgency::HardSafety => 2,
    }
}

fn fold_u64(state: &mut u64, value: u64) {
    for byte in value.to_le_bytes() {
        *state ^= u64::from(byte);
        *state = state.wrapping_mul(FINGERPRINT_PRIME);
    }
}

fn fold_str(state: &mut u64, value: &str) {
    fold_u64(state, value.len() as u64);
    for byte in value.bytes() {
        *state ^= u64::from(byte);
        *state = state.wrapping_mul(FINGERPRINT_PRIME);
    }
}
