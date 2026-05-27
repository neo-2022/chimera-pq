use super::payload_utils::has_mesh_policy_key;
use super::*;
use crate::policy::{MultipathMode, TrafficClass, traffic_hints_from_dps_payload};

pub(super) fn apply_dps_traffic_hints_adaptation(payload: &str, policy: &mut MeshPathPolicy) {
    let Ok(hints) = traffic_hints_from_dps_payload(payload) else {
        return;
    };

    if let Some(class) = hints.traffic_class {
        if !has_mesh_policy_key(payload, "mesh_require_min_reliability") {
            policy.require_min_reliability = match class {
                TrafficClass::ControlDns => 95,
                TrafficClass::WebInteractive => 85,
                TrafficClass::CdnStatic => 80,
                TrafficClass::BufferedStreaming => 75,
                TrafficClass::BulkTransfer => 60,
                TrafficClass::CloudSyncBackup => 60,
                TrafficClass::ArtifactDownload => 70,
                TrafficClass::GamingFps => 90,
                TrafficClass::RealtimeInteractive => 85,
                TrafficClass::Messaging => 80,
                TrafficClass::AuthSensitive => 95,
                TrafficClass::P2pRestricted => 65,
                TrafficClass::BackgroundTelemetry => 50,
                TrafficClass::ControlHealth => 95,
            };
        }
        if !has_mesh_policy_key(payload, "mesh_max_load_score") {
            policy.max_load_score = match class {
                TrafficClass::ControlDns => 45,
                TrafficClass::WebInteractive => 60,
                TrafficClass::CdnStatic => 75,
                TrafficClass::BufferedStreaming => 70,
                TrafficClass::BulkTransfer => 85,
                TrafficClass::CloudSyncBackup => 90,
                TrafficClass::ArtifactDownload => 85,
                TrafficClass::GamingFps => 40,
                TrafficClass::RealtimeInteractive => 55,
                TrafficClass::Messaging => 75,
                TrafficClass::AuthSensitive => 50,
                TrafficClass::P2pRestricted => 80,
                TrafficClass::BackgroundTelemetry => 95,
                TrafficClass::ControlHealth => 50,
            };
        }
        if !has_mesh_policy_key(payload, "mesh_max_peers") {
            policy.max_peers = match class {
                TrafficClass::ControlDns => 1,
                TrafficClass::WebInteractive => 1,
                TrafficClass::CdnStatic => 2,
                TrafficClass::BufferedStreaming => 2,
                TrafficClass::BulkTransfer => 3,
                TrafficClass::CloudSyncBackup => 3,
                TrafficClass::ArtifactDownload => 3,
                TrafficClass::GamingFps => 1,
                TrafficClass::RealtimeInteractive => 1,
                TrafficClass::Messaging => 1,
                TrafficClass::AuthSensitive => 1,
                TrafficClass::P2pRestricted => 2,
                TrafficClass::BackgroundTelemetry => 1,
                TrafficClass::ControlHealth => 1,
            };
        }
    }

    if let Some(mode) = hints.multipath_mode {
        if !has_mesh_policy_key(payload, "mesh_max_peers") {
            let min_peers = match mode {
                MultipathMode::Off | MultipathMode::StandbyOnly => 1,
                MultipathMode::FlowShard => 2,
                MultipathMode::AggregateBuffered => 3,
            };
            policy.max_peers = policy.max_peers.max(min_peers);
        }
        if !has_mesh_policy_key(payload, "mesh_max_selected_per_region") {
            policy.max_selected_per_region = match mode {
                MultipathMode::Off | MultipathMode::StandbyOnly => 1,
                MultipathMode::FlowShard | MultipathMode::AggregateBuffered => {
                    policy.max_selected_per_region.max(2)
                }
            };
        }
    }

    if !has_mesh_policy_key(payload, "mesh_max_selected_per_region")
        && policy.max_selected_per_region > policy.max_peers
    {
        policy.max_selected_per_region = policy.max_peers;
    }
}
