use super::*;

impl MeshRuntime {
    pub fn update_health_state(&mut self, health: &[MeshPeerHealth]) -> Result<(), String> {
        for item in health {
            validate_runtime_node_id(&item.node_id, "mesh health node_id")?;
            self.health_state.insert(
                item.node_id.clone(),
                MeshHealthMeta {
                    health: item.clone(),
                    last_updated_tick: self.tick,
                },
            );
        }
        self.refresh_profile_state_after_health_update();
        Ok(())
    }

    pub(super) fn evict_stale_health(&mut self) {
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
        for node in stale {
            self.health_state.remove(&node);
        }
        self.refresh_profile_state_after_health_update();
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

    pub(super) fn evict_stale_peers(&mut self) {
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
        for node in stale {
            self.peers.remove(&node);
            self.peer_meta.remove(&node);
            self.health_state.remove(&node);
        }
    }
}
