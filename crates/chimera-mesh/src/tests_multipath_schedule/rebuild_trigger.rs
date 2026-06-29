use crate::{MeshMultipathRebuildDirtyScope, MeshMultipathRebuildPolicy};

mod discovery;
mod health;
mod performance;

fn assert_peer_table_changed_signal(runtime: &crate::MeshRuntime) {
    let signal = runtime
        .pending_multipath_rebuild_signal()
        .unwrap_or_else(|| unreachable!("changed discovery should set pending rebuild signal"));
    assert_eq!(signal.reason(), "peer_table_changed");
    assert_eq!(
        signal.dirty_scope(),
        MeshMultipathRebuildDirtyScope::PeerSet
    );
    assert_eq!(signal.affected_peer_count(), 1);
}

fn policy() -> MeshMultipathRebuildPolicy {
    MeshMultipathRebuildPolicy::new(3, 4)
        .unwrap_or_else(|e| unreachable!("policy should be accepted: {e}"))
}
