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
    pub policy: String,
    pub privacy: String,
    pub explain: Vec<String>,
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
            .field("policy", &self.policy)
            .field("privacy", &self.privacy)
            .finish()
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
    let explain = vec![
        format!("multipath_rebuild_action={}", action.as_str()),
        format!("multipath_rebuild_reason={reason}"),
        format!("multipath_rebuild_signal_reason={}", signal.reason()),
        format!("multipath_rebuild_allowed={rebuild_allowed}"),
        format!("multipath_rebuild_debounced={debounced}"),
        format!("multipath_rebuild_stale={stale}"),
        format!(
            "multipath_rebuild_generation_changed={}",
            changes.generation_changed
        ),
        format!(
            "multipath_rebuild_fingerprint_changed={}",
            changes.fingerprint_changed
        ),
        format!(
            "multipath_rebuild_dirty_scope={}",
            signal.dirty_scope().as_str()
        ),
        format!(
            "multipath_rebuild_affected_peer_count={}",
            signal.affected_peer_count()
        ),
        format!("multipath_rebuild_pending_count={pending_count}"),
        format!("multipath_rebuild_policy={REBUILD_CONTROL_POLICY}"),
        format!("multipath_rebuild_privacy={REBUILD_CONTROL_PRIVACY}"),
    ];
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
        policy: REBUILD_CONTROL_POLICY.to_string(),
        privacy: REBUILD_CONTROL_PRIVACY.to_string(),
        explain,
    }
}
