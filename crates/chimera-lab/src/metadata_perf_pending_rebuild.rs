use chimera_mesh::{
    MeshDiscoveryRecord, MeshJoinRequest, MeshMultipathRebuildPolicy, MeshPathPolicy,
    MeshPeerPerformance, MeshPeerTablePolicy, MeshRuntime, MultipathDemand, MultipathMode,
};
use std::time::{Duration, Instant};

pub(crate) struct PendingRebuildMeasurements {
    pub(crate) full_plan: PendingRebuildMeasurement,
    pub(crate) core_plan: PendingRebuildMeasurement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingRebuildMeasurement {
    pub(crate) total_elapsed: Duration,
    pub(crate) p95_ns: u128,
}

pub(crate) fn measure_pending_rebuild_plans(
    peer_count: usize,
    discovery_source: &str,
    sample_count: usize,
    batch_size: usize,
) -> PendingRebuildMeasurements {
    let (runtime, request, path_policy, rebuild_policy) =
        pending_rebuild_fixture(peer_count, discovery_source);
    PendingRebuildMeasurements {
        full_plan: measure_pending_rebuild_plan_path(
            &runtime,
            &request,
            &path_policy,
            &rebuild_policy,
            sample_count,
            batch_size,
        ),
        core_plan: measure_pending_rebuild_plan_core(
            &runtime,
            &request,
            &path_policy,
            &rebuild_policy,
            sample_count,
            batch_size,
        ),
    }
}

fn pending_rebuild_fixture(
    peer_count: usize,
    discovery_source: &str,
) -> (
    MeshRuntime,
    MeshJoinRequest,
    MeshPathPolicy,
    MeshMultipathRebuildPolicy,
) {
    let mut runtime = MeshRuntime::bootstrap("metadata-perf", "seed-a")
        .unwrap_or_else(|error| unreachable!("{error}"));
    runtime
        .set_peer_table_policy(MeshPeerTablePolicy {
            stale_after_ticks: 1_000_000,
            ..MeshPeerTablePolicy::default()
        })
        .unwrap_or_else(|error| unreachable!("{error}"));
    let regions = ["eu", "us", "ap", "EU"];
    let records: Vec<MeshDiscoveryRecord> = (0..peer_count)
        .map(|index| MeshDiscoveryRecord {
            node_id: format!("perf-rebuild-node-{index:03}"),
            endpoint: format!("198.51.101.{}:443", (index % 200) + 1),
            region: regions[index % regions.len()].to_string(),
            load_score: ((index * 11) % 70) as u8,
            reliability_score: (70 + ((index * 3) % 30)) as u8,
        })
        .collect();
    runtime
        .merge_discovery(discovery_source, &records)
        .unwrap_or_else(|error| unreachable!("{error}"));
    let request = MeshJoinRequest {
        namespace: "metadata-perf".to_string(),
        node_name: "metadata-perf-pending-rebuild".to_string(),
        invite_token: None,
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string(), "us".to_string(), "ap".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 70,
        max_load_score: 80,
        max_peers: 8,
        prefer_region_diversity: true,
        max_selected_per_region: 4,
        min_distinct_regions: 3,
        path_profile_override: None,
        multipath_mode: Some(MultipathMode::FlowShard),
        multipath_demand: Some(MultipathDemand::Bulk),
        connect_fallback_ports: vec![443, 8443],
    };
    let rebuild_policy =
        MeshMultipathRebuildPolicy::new(1, 1).unwrap_or_else(|error| unreachable!("{error}"));
    let (_plan, _decision) = runtime
        .plan_path_core_with_pending_multipath_rebuild(&request, &policy, &rebuild_policy)
        .unwrap_or_else(|error| unreachable!("{error}"));
    runtime
        .update_peer_performance(&[
            MeshPeerPerformance {
                node_id: "perf-rebuild-node-000".to_string(),
                latency_ms: Some(250),
                throughput_mbps: Some(40),
            },
            MeshPeerPerformance {
                node_id: "perf-rebuild-node-001".to_string(),
                latency_ms: Some(20),
                throughput_mbps: Some(900),
            },
        ])
        .unwrap_or_else(|error| unreachable!("{error}"));
    if runtime.pending_multipath_rebuild_signal().is_none() {
        unreachable!("pending rebuild fixture must start with a pending signal");
    }
    (runtime, request, policy, rebuild_policy)
}

fn measure_pending_rebuild_plan_path(
    runtime: &MeshRuntime,
    request: &MeshJoinRequest,
    path_policy: &MeshPathPolicy,
    rebuild_policy: &MeshMultipathRebuildPolicy,
    sample_count: usize,
    batch_size: usize,
) -> PendingRebuildMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for sample in 0..sample_count {
        let batch_start = Instant::now();
        for offset in 0..batch_size {
            let _sequence = sample.saturating_mul(batch_size).saturating_add(offset);
            let mut runtime = runtime.clone();
            let (plan, decision) = runtime
                .plan_path_with_pending_multipath_rebuild(request, path_policy, rebuild_policy)
                .unwrap_or_else(|error| unreachable!("{error}"));
            checksum = checksum.wrapping_add(plan.selected_peers.len());
            checksum = checksum.wrapping_add(plan.explain.len());
            checksum = checksum.wrapping_add(decision.is_some() as usize);
            checksum = checksum.wrapping_add(plan.multipath_schedule.active_lane_count);
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    finish_measurement(
        samples,
        total_start.elapsed(),
        checksum,
        "live pending rebuild plan path",
    )
}

fn measure_pending_rebuild_plan_core(
    runtime: &MeshRuntime,
    request: &MeshJoinRequest,
    path_policy: &MeshPathPolicy,
    rebuild_policy: &MeshMultipathRebuildPolicy,
    sample_count: usize,
    batch_size: usize,
) -> PendingRebuildMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for sample in 0..sample_count {
        let batch_start = Instant::now();
        for offset in 0..batch_size {
            let _sequence = sample.saturating_mul(batch_size).saturating_add(offset);
            let mut runtime = runtime.clone();
            let (plan, decision) = runtime
                .plan_path_core_with_pending_multipath_rebuild(request, path_policy, rebuild_policy)
                .unwrap_or_else(|error| unreachable!("{error}"));
            checksum = checksum.wrapping_add(plan.selected_peers.len());
            checksum = checksum.wrapping_add(decision.is_some() as usize);
            checksum = checksum.wrapping_add(plan.multipath_schedule.active_lane_count);
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    finish_measurement(
        samples,
        total_start.elapsed(),
        checksum,
        "live pending rebuild plan core",
    )
}

fn finish_measurement(
    mut samples: Vec<u128>,
    total_elapsed: Duration,
    checksum: usize,
    label: &str,
) -> PendingRebuildMeasurement {
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == usize::MAX {
        eprintln!("{label} metadata perf checksum guard tripped");
    }
    PendingRebuildMeasurement {
        total_elapsed,
        p95_ns,
    }
}
