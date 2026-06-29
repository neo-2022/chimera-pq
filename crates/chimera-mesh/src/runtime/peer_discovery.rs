use super::*;
#[path = "peer_discovery_update.rs"]
mod update;
use update::{apply_existing_peer_update, existing_peer_update_context};

impl MeshRuntime {
    pub fn merge_discovery(
        &mut self,
        source: &str,
        records: &[MeshDiscoveryRecord],
    ) -> Result<(), String> {
        let source = source.trim();
        validate_source_name(source, "mesh discovery source")?;
        let mut batch_seen: BTreeSet<&str> = BTreeSet::new();
        for record in records {
            record.validate()?;
            if !batch_seen.insert(record.node_id.as_str()) {
                return Err("mesh discovery batch contains duplicate node_id".to_string());
            }
        }

        let before_fingerprint = self.rebuild_trigger_fingerprint();
        self.tick = self.tick.saturating_add(1);
        self.sources.insert(source.to_string());
        let mut affected_peer_count = 0_usize;
        for record in records {
            let previous_meta = self.peer_meta.get(&record.node_id).cloned();
            if self.peers.contains_key(&record.node_id) {
                if let Some(ctx) =
                    existing_peer_update_context(self, record, previous_meta.as_ref())
                {
                    affected_peer_count = affected_peer_count
                        .saturating_add(usize::from(apply_existing_peer_update(self, record, ctx)));
                }
                continue;
            }
            affected_peer_count = affected_peer_count.saturating_add(1);
            self.peers.insert(
                record.node_id.clone(),
                MeshPeerState {
                    node_id: record.node_id.clone(),
                    endpoint: record.endpoint.clone(),
                    region: record.region.clone(),
                    reliability_score: record.reliability_score,
                    load_score: record.load_score,
                    latency_ms: None,
                    throughput_mbps: None,
                    selection_score: 0,
                },
            );
            let identity_marker = self.allocate_peer_identity_marker();
            self.peer_meta.insert(
                record.node_id.clone(),
                MeshPeerMeta {
                    identity_marker,
                    last_seen_tick: self.tick,
                    update_events: 1,
                    replacement_events: 0,
                    hold_events: 0,
                    degraded_events: 0,
                    churn_block_events: 0,
                    threshold_block_events: 0,
                    last_effective_replacement_threshold: self
                        .table_policy
                        .replacement_min_score_delta,
                    endpoint_generation: 0,
                    update_bootstrap_url: None,
                },
            );
        }
        let stale_peer_dropped = self.evict_stale_peers() > 0;
        let stale_health_dropped = self.evict_stale_health() > 0;
        self.enforce_peer_table_limits();
        let table_enforcement_dropped = self.last_table_enforcement_report.dropped_total > 0;
        let dirty_scope_ambiguous =
            table_enforcement_dropped || stale_peer_dropped || stale_health_dropped;
        let (dirty_scope, affected_peer_count) =
            if dirty_scope_ambiguous || affected_peer_count == 0 {
                (MeshMultipathRebuildDirtyScope::Unknown, 0)
            } else {
                (MeshMultipathRebuildDirtyScope::PeerSet, affected_peer_count)
            };
        self.mark_pending_multipath_rebuild_with_dirty_scope(
            MeshMultipathRebuildTriggerCause::PeerTableChanged,
            before_fingerprint,
            dirty_scope,
            affected_peer_count,
        )?;
        Ok(())
    }
}
