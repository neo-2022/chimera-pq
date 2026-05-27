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
        self.tick = self.tick.saturating_add(1);
        self.sources.insert(source.to_string());
        let mut batch_seen: BTreeSet<&str> = BTreeSet::new();
        for record in records {
            record.validate()?;
            if !batch_seen.insert(record.node_id.as_str()) {
                return Err("mesh discovery batch contains duplicate node_id".to_string());
            }
            let previous_meta = self.peer_meta.get(&record.node_id).cloned();
            if self.peers.contains_key(&record.node_id) {
                if let Some(ctx) =
                    existing_peer_update_context(self, record, previous_meta.as_ref())
                {
                    apply_existing_peer_update(self, record, ctx);
                }
                continue;
            }
            self.peers.insert(
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
            self.peer_meta.insert(
                record.node_id.clone(),
                MeshPeerMeta {
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
                },
            );
        }
        self.evict_stale_peers();
        self.evict_stale_health();
        self.enforce_peer_table_limits();
        Ok(())
    }
}
