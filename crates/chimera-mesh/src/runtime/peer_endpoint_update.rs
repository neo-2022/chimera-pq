use super::*;

impl MeshRuntime {
    pub fn merge_published_endpoint_updates(
        &mut self,
        source: &str,
        updates: &[MeshPublishedEndpointUpdate],
    ) -> Result<(), String> {
        let source = source.trim();
        validate_source_name(source, "mesh published endpoint source")?;
        let mut batch_seen: BTreeSet<&str> = BTreeSet::new();
        for update in updates {
            update.validate()?;
            if !batch_seen.insert(update.node_id.as_str()) {
                return Err("mesh published endpoint batch contains duplicate node_id".to_string());
            }
            if let (Some(peer), Some(meta)) = (
                self.peers.get(&update.node_id),
                self.peer_meta.get(&update.node_id),
            ) {
                let same_generation = update.endpoint_generation == meta.endpoint_generation;
                let same_endpoint = peer.endpoint == update.endpoint;
                let same_update_url = meta.update_bootstrap_url == update.update_bootstrap_url;
                if same_generation && !(same_endpoint && same_update_url) {
                    return Err("mesh published endpoint generation conflict".to_string());
                }
            }
        }

        let before_fingerprint = self.rebuild_trigger_fingerprint();
        self.tick = self.tick.saturating_add(1);
        self.sources.insert(source.to_string());
        let mut affected_peer_count = 0_usize;

        for update in updates {
            let Some(peer) = self.peers.get_mut(&update.node_id) else {
                continue;
            };
            let Some(meta) = self.peer_meta.get_mut(&update.node_id) else {
                continue;
            };

            if update.endpoint_generation < meta.endpoint_generation {
                meta.last_seen_tick = self.tick;
                continue;
            }

            let same_generation = update.endpoint_generation == meta.endpoint_generation;
            let same_endpoint = peer.endpoint == update.endpoint;
            let same_update_url = meta.update_bootstrap_url == update.update_bootstrap_url;

            if same_generation && same_endpoint && same_update_url {
                meta.last_seen_tick = self.tick;
                continue;
            }

            peer.endpoint = update.endpoint.clone();
            meta.endpoint_generation = update.endpoint_generation;
            meta.update_bootstrap_url = update.update_bootstrap_url.clone();
            meta.last_seen_tick = self.tick;
            meta.update_events = meta.update_events.saturating_add(1);
            affected_peer_count = affected_peer_count.saturating_add(1);
        }

        self.mark_pending_multipath_rebuild_with_dirty_scope(
            MeshMultipathRebuildTriggerCause::PublishedEndpointChanged,
            before_fingerprint,
            if affected_peer_count == 0 {
                MeshMultipathRebuildDirtyScope::Unknown
            } else {
                MeshMultipathRebuildDirtyScope::PeerSet
            },
            affected_peer_count,
        )?;
        Ok(())
    }
}
