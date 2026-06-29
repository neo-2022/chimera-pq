use crate::model::{MeshDiscoveryRecord, MeshPathPlan, MeshPeerState, MeshPublishedEndpointUpdate};

impl std::fmt::Debug for MeshDiscoveryRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshDiscoveryRecord")
            .field("node_id", &"<redacted>")
            .field("endpoint", &"<redacted>")
            .field("region", &self.region)
            .field("load_score", &self.load_score)
            .field("reliability_score", &self.reliability_score)
            .finish()
    }
}

impl std::fmt::Debug for MeshPeerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshPeerState")
            .field("node_id", &"<redacted>")
            .field("endpoint", &"<redacted>")
            .field("region", &self.region)
            .field("reliability_score", &self.reliability_score)
            .field("load_score", &self.load_score)
            .field("latency_ms", &self.latency_ms)
            .field("throughput_mbps", &self.throughput_mbps)
            .field("selection_score", &self.selection_score)
            .finish()
    }
}

impl std::fmt::Debug for MeshPublishedEndpointUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshPublishedEndpointUpdate")
            .field("node_id", &"<redacted>")
            .field("endpoint", &"<redacted>")
            .field(
                "update_bootstrap_url",
                &self.update_bootstrap_url.as_ref().map(|_| "<redacted>"),
            )
            .field("endpoint_generation", &self.endpoint_generation)
            .finish()
    }
}

impl std::fmt::Debug for MeshPathPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshPathPlan")
            .field("namespace", &"<redacted>")
            .field("join_mode", &self.join_mode)
            .field("selected_peer_count", &self.selected_peers.len())
            .field("multipath_schedule", &self.multipath_schedule)
            .field("explain_line_count", &self.explain.len())
            .finish()
    }
}
