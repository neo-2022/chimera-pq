use super::*;
#[path = "peer_maintenance_enforcement.rs"]
mod enforcement;

impl MeshRuntime {
    pub(super) fn enforce_peer_table_limits(&mut self) {
        let total_peers_before = self.peers.len();
        let enforcement = enforcement::compute_enforcement(
            &self.peers,
            &self.profile_state,
            self.tick,
            &self.table_policy,
        );

        for node_id in enforcement.drop_set {
            self.peers.remove(&node_id);
            self.peer_meta.remove(&node_id);
            self.health_state.remove(&node_id);
        }
        let total_peers_after = self.peers.len();
        self.last_table_enforcement_report = MeshPeerTableEnforcementReport {
            tick: self.tick,
            total_peers_before,
            total_peers_after,
            dropped_total: total_peers_before.saturating_sub(total_peers_after),
            dropped_by_region_cap: enforcement.dropped_by_region_cap,
            dropped_by_global_cap: enforcement.dropped_by_global_cap,
            protected_region_skips: enforcement.protected_region_skips,
            effective_profile: profile_label(enforcement.effective_profile).to_string(),
            effective_target_distinct_regions: enforcement.effective_target_distinct_regions,
            effective_target_source: enforcement.effective_target_source.to_string(),
        };
    }
}
