use super::*;
use crate::runtime::health_state_utils::unhealthy_node_ids_from_health_state;
use crate::runtime::preemptive_antiflap::{ShadowAntiFlapGuard, apply_preemptive_antiflap};

impl MeshRuntime {
    pub(super) fn unhealthy_node_ids(&self) -> BTreeSet<String> {
        unhealthy_node_ids_from_health_state(&self.health_state)
    }

    pub(super) fn apply_shadow_antiflap_guard(
        &self,
        shadow: &mut crate::preemptive::ShadowRuntimeDecision,
    ) -> ShadowAntiFlapGuard {
        apply_preemptive_antiflap(shadow, &self.peer_meta, self.tick, &self.table_policy)
    }
}
