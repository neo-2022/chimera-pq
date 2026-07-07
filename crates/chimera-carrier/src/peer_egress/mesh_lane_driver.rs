use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use chimera_mesh::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshPathPolicy, MeshPublishedEndpointUpdate, MeshRuntime,
};

use crate::peer_egress::discovery_fetch::{
    DiscoveryFetchOptions, RemoteMeshNode, fetch_discovery_nodes,
};
use crate::peer_egress::lane_binding::write_transit_lane_document_from_mesh_plan;
use crate::peer_egress::protocol::redacted_log_reason;

#[derive(Debug, Clone)]
pub struct MeshLaneDriverOptions {
    pub namespace: String,
    pub self_node_id: String,
    pub policy_payload: String,
    pub lane_document_path: String,
    pub discovery_urls: Vec<String>,
    pub discovery_keyring: std::collections::BTreeMap<String, String>,
    pub discovery_timeout_ms: u64,
    pub poll_interval_ms: u64,
}

pub fn run_mesh_lane_driver_once(options: &MeshLaneDriverOptions) -> Result<(), String> {
    let nodes = fetch_discovery_nodes(&DiscoveryFetchOptions {
        urls: options.discovery_urls.clone(),
        keyring: options.discovery_keyring.clone(),
        revoked_key_ids: BTreeSet::new(),
        revoked_node_ids: BTreeSet::new(),
        timeout_ms: options.discovery_timeout_ms,
    })?;

    let filtered_nodes: Vec<RemoteMeshNode> = nodes
        .into_iter()
        .filter(|node| node.node_id != options.self_node_id)
        .collect();

    if filtered_nodes.is_empty() {
        return Err("mesh lane driver found no remote peers in discovery snapshot".to_string());
    }

    let mut runtime = MeshRuntime::bootstrap(&options.namespace, "mesh-discovery-driver")
        .map_err(|error| format!("mesh lane driver runtime bootstrap failed: {error}"))?;

    let records: Vec<MeshDiscoveryRecord> = filtered_nodes
        .iter()
        .map(|node| MeshDiscoveryRecord {
            node_id: node.node_id.clone(),
            endpoint: node.endpoint.clone(),
            region: node.region(),
            load_score: node.load_score(),
            reliability_score: node.reliability_score(),
        })
        .collect();

    runtime
        .merge_discovery("mesh-discovery", &records)
        .map_err(|error| format!("mesh lane driver discovery merge failed: {error}"))?;

    let endpoint_updates: Vec<MeshPublishedEndpointUpdate> = filtered_nodes
        .iter()
        .filter_map(|node| {
            node.endpoint_generation
                .map(|generation| MeshPublishedEndpointUpdate {
                    node_id: node.node_id.clone(),
                    endpoint: node.endpoint.clone(),
                    update_bootstrap_url: node.update_bootstrap_url.clone(),
                    endpoint_generation: generation,
                })
        })
        .collect();

    if !endpoint_updates.is_empty() {
        runtime
            .merge_published_endpoint_updates("mesh-discovery", &endpoint_updates)
            .map_err(|error| format!("mesh lane driver endpoint merge failed: {error}"))?;
    }

    let request = MeshJoinRequest {
        namespace: options.namespace.clone(),
        node_name: options.self_node_id.clone(),
        invite_token: None,
    };

    let _policy_check = MeshPathPolicy::from_dps_payload(&options.policy_payload)
        .map_err(|error| format!("mesh lane driver policy parse failed: {error}"))?;

    let refreshed_plan = runtime
        .plan_path_from_dps_payload(&request, &options.policy_payload)
        .map_err(|error| format!("mesh lane driver plan failed: {error}"))?;

    write_transit_lane_document_from_mesh_plan(&refreshed_plan, &options.lane_document_path)
        .map_err(|error| format!("mesh lane driver write lane document failed: {error}"))?;

    let active_lanes = refreshed_plan.multipath_schedule.active_lane_count;
    eprintln!(
        "event=mesh_lane_driver_plan_ok active_lanes={} plan_namespace={}",
        active_lanes, refreshed_plan.namespace
    );
    Ok(())
}

pub fn run_mesh_lane_driver(options: MeshLaneDriverOptions, cancel: Arc<AtomicBool>) {
    let mut backoff_ms = 1_000_u64;
    let max_backoff_ms = 60_000_u64;
    while !cancel.load(Ordering::Relaxed) {
        let start = Instant::now();
        match run_mesh_lane_driver_once(&options) {
            Ok(()) => {
                backoff_ms = 1_000;
            }
            Err(error) => {
                eprintln!(
                    "event=mesh_lane_driver_error reason_class={}",
                    redacted_log_reason(&error)
                );
                let sleep_until = start + Duration::from_millis(backoff_ms);
                while Instant::now() < sleep_until {
                    if cancel.load(Ordering::Relaxed) {
                        return;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
                continue;
            }
        }
        let elapsed = start.elapsed();
        let remaining = Duration::from_millis(options.poll_interval_ms).saturating_sub(elapsed);
        let sleep_until = Instant::now() + remaining;
        while Instant::now() < sleep_until {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
#[path = "mesh_lane_driver_tests.rs"]
mod tests;
