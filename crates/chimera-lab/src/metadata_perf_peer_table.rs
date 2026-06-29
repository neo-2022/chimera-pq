use chimera_mesh::{MeshDiscoveryRecord, MeshPeerTablePolicy, MeshRuntime};
use std::time::Instant;

use super::ScheduleMeasurement;

pub(super) fn measure_enforcement_noop(
    peer_count: usize,
    sample_count: usize,
    batch_size: usize,
) -> ScheduleMeasurement {
    let mut runtime = peer_table_enforcement_noop_fixture(peer_count);
    let policy = runtime.peer_table_policy_snapshot();
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for _sample in 0..sample_count {
        let batch_start = Instant::now();
        for _offset in 0..batch_size {
            runtime
                .set_peer_table_policy(policy.clone())
                .unwrap_or_else(|error| unreachable!("{error}"));
            let report = runtime.peer_table_last_enforcement_report();
            if report.dropped_total != 0 {
                unreachable!("peer table enforcement no-op metric measured drop path");
            }
            checksum = checksum.wrapping_add(runtime.peer_count());
            checksum = checksum.wrapping_add(report.total_peers_after);
            checksum = checksum.wrapping_add(report.effective_target_distinct_regions);
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    finish_measurement(samples, total_start.elapsed(), checksum)
}

fn peer_table_enforcement_noop_fixture(peer_count: usize) -> MeshRuntime {
    let mut runtime = MeshRuntime::bootstrap("metadata-perf", "seed-a")
        .unwrap_or_else(|error| unreachable!("{error}"));
    let policy = MeshPeerTablePolicy {
        max_entries: peer_count * 2,
        max_entries_per_region: peer_count * 2,
        stale_after_ticks: 1_000_000,
        target_distinct_regions: 1,
        ..MeshPeerTablePolicy::default()
    };
    runtime
        .set_peer_table_policy(policy)
        .unwrap_or_else(|error| unreachable!("{error}"));
    let records: Vec<MeshDiscoveryRecord> = (0..peer_count)
        .map(|index| MeshDiscoveryRecord {
            node_id: format!("perf-table-node-{index:03}"),
            endpoint: format!("198.51.103.{}:443", (index % 200) + 1),
            region: "eu".to_string(),
            load_score: ((index * 5) % 70) as u8,
            reliability_score: (75 + ((index * 7) % 25)) as u8,
        })
        .collect();
    runtime
        .merge_discovery("metadata-perf-table", &records)
        .unwrap_or_else(|error| unreachable!("{error}"));
    runtime
}

fn finish_measurement(
    mut samples: Vec<u128>,
    total_elapsed: std::time::Duration,
    checksum: usize,
) -> ScheduleMeasurement {
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == usize::MAX {
        eprintln!("peer table enforcement metadata perf checksum guard tripped");
    }
    ScheduleMeasurement {
        total_elapsed,
        p95_ns,
    }
}
