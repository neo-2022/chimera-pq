use super::*;

impl MeshRuntime {
    pub fn update_health_state(&mut self, health: &[MeshPeerHealth]) -> Result<(), String> {
        let mut validated = Vec::with_capacity(health.len());
        for item in health {
            validate_runtime_node_id(&item.node_id, "mesh health node_id")?;
            if !self.peers.contains_key(&item.node_id) {
                return Err("mesh health state references unknown node".to_string());
            }
            validated.push(item.clone());
        }
        let before_fingerprint = self.rebuild_trigger_fingerprint();
        let urgent = validated
            .iter()
            .any(|item| !item.healthy || item.cooldown_active);
        let mut affected_peer_count = 0_usize;
        for item in validated {
            let changed = self
                .health_state
                .get(&item.node_id)
                .map(|meta| meta.health != item)
                .unwrap_or(true);
            affected_peer_count = affected_peer_count.saturating_add(usize::from(changed));
            self.health_state.insert(
                item.node_id.clone(),
                MeshHealthMeta {
                    health: item,
                    last_updated_tick: self.tick,
                },
            );
        }
        self.refresh_profile_state_after_health_update();
        if urgent {
            self.mark_pending_multipath_rebuild_with_dirty_scope(
                MeshMultipathRebuildTriggerCause::UrgentFailover,
                before_fingerprint,
                MeshMultipathRebuildDirtyScope::PeerSet,
                affected_peer_count,
            )?;
        } else {
            self.mark_pending_multipath_rebuild_with_dirty_scope(
                MeshMultipathRebuildTriggerCause::PeerHealthChanged,
                before_fingerprint,
                MeshMultipathRebuildDirtyScope::PeerSet,
                affected_peer_count,
            )?;
        }
        Ok(())
    }

    pub(super) fn evict_stale_health(&mut self) -> usize {
        let stale_after = self.table_policy.stale_after_ticks;
        let now = self.tick;
        let stale: Vec<String> = self
            .health_state
            .iter()
            .filter_map(|(node, meta)| {
                let age = now.saturating_sub(meta.last_updated_tick);
                if age > stale_after {
                    Some(node.clone())
                } else {
                    None
                }
            })
            .collect();
        let removed_count = stale.len();
        for node in stale {
            self.health_state.remove(&node);
        }
        self.refresh_profile_state_after_health_update();
        removed_count
    }

    pub(super) fn refresh_profile_state_after_health_update(&mut self) {
        let degraded_present = self
            .health_state
            .values()
            .any(|meta| !meta.health.healthy || meta.health.cooldown_active);
        if degraded_present {
            self.profile_state.active_profile = MeshPathProfile::Resilient;
            self.profile_state.degrade_cleared_since_tick = None;
            return;
        }
        if self.profile_state.active_profile == MeshPathProfile::Resilient
            && self.profile_state.degrade_cleared_since_tick.is_none()
        {
            self.profile_state.degrade_cleared_since_tick = Some(self.tick);
        }
    }

    pub(super) fn evict_stale_peers(&mut self) -> usize {
        let stale_after = self.table_policy.stale_after_ticks;
        let now = self.tick;
        let stale: Vec<String> = self
            .peer_meta
            .iter()
            .filter_map(|(node, meta)| {
                let age = now.saturating_sub(meta.last_seen_tick);
                if age > stale_after {
                    Some(node.clone())
                } else {
                    None
                }
            })
            .collect();
        let removed_count = stale.len();
        for node in stale {
            self.peers.remove(&node);
            self.peer_meta.remove(&node);
            self.health_state.remove(&node);
        }
        removed_count
    }
}
