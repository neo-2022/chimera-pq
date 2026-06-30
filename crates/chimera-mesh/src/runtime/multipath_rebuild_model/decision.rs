use super::{
    MeshMultipathRebuildAction, MeshMultipathRebuildDirtyScope, MeshMultipathRebuildSignal,
    REBUILD_CONTROL_POLICY, REBUILD_CONTROL_PRIVACY,
};

#[derive(Clone, PartialEq, Eq)]
pub struct MeshMultipathRebuildDecision {
    pub action: MeshMultipathRebuildAction,
    pub reason: String,
    pub signal_reason: String,
    pub rebuild_allowed: bool,
    pub debounced: bool,
    pub stale: bool,
    pub generation_changed: bool,
    pub fingerprint_changed: bool,
    pub pending_count: u64,
    pub dirty_scope: MeshMultipathRebuildDirtyScope,
    pub affected_peer_count: usize,
}

impl std::fmt::Debug for MeshMultipathRebuildDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshMultipathRebuildDecision")
            .field("action", &self.action)
            .field("reason", &self.reason)
            .field("signal_reason", &self.signal_reason)
            .field("rebuild_allowed", &self.rebuild_allowed)
            .field("debounced", &self.debounced)
            .field("stale", &self.stale)
            .field("generation_changed", &self.generation_changed)
            .field("fingerprint_changed", &self.fingerprint_changed)
            .field("pending_count", &self.pending_count)
            .field("dirty_scope", &self.dirty_scope)
            .field("affected_peer_count", &self.affected_peer_count)
            .field("policy", &REBUILD_CONTROL_POLICY)
            .field("privacy", &REBUILD_CONTROL_PRIVACY)
            .finish()
    }
}

impl MeshMultipathRebuildDecision {
    pub fn append_explain_to(&self, explain: &mut Vec<String>) {
        explain.reserve(13);
        explain.push(format!("multipath_rebuild_action={}", self.action.as_str()));
        explain.push(format!("multipath_rebuild_reason={}", self.reason));
        explain.push(format!(
            "multipath_rebuild_signal_reason={}",
            self.signal_reason
        ));
        explain.push(format!(
            "multipath_rebuild_allowed={}",
            self.rebuild_allowed
        ));
        explain.push(format!("multipath_rebuild_debounced={}", self.debounced));
        explain.push(format!("multipath_rebuild_stale={}", self.stale));
        explain.push(format!(
            "multipath_rebuild_generation_changed={}",
            self.generation_changed
        ));
        explain.push(format!(
            "multipath_rebuild_fingerprint_changed={}",
            self.fingerprint_changed
        ));
        explain.push(format!(
            "multipath_rebuild_dirty_scope={}",
            self.dirty_scope.as_str()
        ));
        explain.push(format!(
            "multipath_rebuild_affected_peer_count={}",
            self.affected_peer_count
        ));
        explain.push(format!(
            "multipath_rebuild_pending_count={}",
            self.pending_count
        ));
        explain.push(format!("multipath_rebuild_policy={REBUILD_CONTROL_POLICY}"));
        explain.push(format!(
            "multipath_rebuild_privacy={REBUILD_CONTROL_PRIVACY}"
        ));
    }

    pub fn explain(&self) -> Vec<String> {
        let mut explain = Vec::with_capacity(13);
        self.append_explain_to(&mut explain);
        explain
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::runtime) struct MeshMultipathRebuildChanges {
    pub generation_changed: bool,
    pub fingerprint_changed: bool,
}

pub(in crate::runtime) fn build_decision(
    action: MeshMultipathRebuildAction,
    reason: &str,
    signal: &MeshMultipathRebuildSignal,
    changes: MeshMultipathRebuildChanges,
    stale: bool,
    debounced: bool,
    pending_count: u64,
) -> MeshMultipathRebuildDecision {
    let rebuild_allowed = action == MeshMultipathRebuildAction::AllowRebuild;
    MeshMultipathRebuildDecision {
        action,
        reason: reason.to_string(),
        signal_reason: signal.reason().to_string(),
        rebuild_allowed,
        debounced,
        stale,
        generation_changed: changes.generation_changed,
        fingerprint_changed: changes.fingerprint_changed,
        pending_count,
        dirty_scope: signal.dirty_scope(),
        affected_peer_count: signal.affected_peer_count(),
    }
}
