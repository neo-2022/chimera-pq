use chimera_mesh::{
    continuity_policy_from_dps_payload, multipath_demand_from_dps_payload,
    multipath_mode_from_dps_payload, traffic_class_from_dps_payload,
    traffic_hints_from_dps_payload,
};
use std::time::Instant;

use super::ScheduleMeasurement;

const TRAFFIC_HINTS_PAYLOAD: &str = concat!(
    "allow=mesh;",
    "mesh_unknown_future_key=1;",
    "mesh_max_peers=2;",
    "mesh_traffic_class=gaming_fps;",
    "mesh_multipath_mode=standby_only;",
    "mesh_multipath_demand=bulk;",
    "mesh_continuity_policy=allow_flow_drain"
);

pub(super) fn measure_one_pass(sample_count: usize, batch_size: usize) -> ScheduleMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for _sample in 0..sample_count {
        let batch_start = Instant::now();
        for _offset in 0..batch_size {
            let hints = traffic_hints_from_dps_payload(TRAFFIC_HINTS_PAYLOAD)
                .unwrap_or_else(|error| unreachable!("{error}"));
            checksum = checksum.wrapping_add(usize::from(hints.has_any_hint()));
            checksum =
                checksum.wrapping_add(hints.traffic_class.map_or(0, |value| value.as_str().len()));
            checksum =
                checksum.wrapping_add(hints.multipath_mode.map_or(0, |value| value.as_str().len()));
            checksum = checksum.wrapping_add(
                hints
                    .multipath_demand
                    .map_or(0, |value| value.as_str().len()),
            );
            checksum = checksum.wrapping_add(
                hints
                    .continuity_policy
                    .map_or(0, |value| value.as_str().len()),
            );
            checksum = checksum.wrapping_add(hints.shadow_switch_mode.as_str().len());
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    finish_measurement(
        samples,
        total_start.elapsed(),
        checksum,
        "traffic hints one-pass",
    )
}

pub(super) fn measure_four_pass_baseline(
    sample_count: usize,
    batch_size: usize,
) -> ScheduleMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for _sample in 0..sample_count {
        let batch_start = Instant::now();
        for _offset in 0..batch_size {
            let traffic_class = traffic_class_from_dps_payload(TRAFFIC_HINTS_PAYLOAD)
                .unwrap_or_else(|error| unreachable!("{error}"));
            let multipath_mode = multipath_mode_from_dps_payload(TRAFFIC_HINTS_PAYLOAD)
                .unwrap_or_else(|error| unreachable!("{error}"));
            let multipath_demand = multipath_demand_from_dps_payload(TRAFFIC_HINTS_PAYLOAD)
                .unwrap_or_else(|error| unreachable!("{error}"));
            let continuity_policy = continuity_policy_from_dps_payload(TRAFFIC_HINTS_PAYLOAD)
                .unwrap_or_else(|error| unreachable!("{error}"));
            checksum = checksum.wrapping_add(usize::from(traffic_class.is_some()));
            checksum = checksum.wrapping_add(traffic_class.map_or(0, |value| value.as_str().len()));
            checksum =
                checksum.wrapping_add(multipath_mode.map_or(0, |value| value.as_str().len()));
            checksum =
                checksum.wrapping_add(multipath_demand.map_or(0, |value| value.as_str().len()));
            checksum =
                checksum.wrapping_add(continuity_policy.map_or(0, |value| value.as_str().len()));
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    finish_measurement(
        samples,
        total_start.elapsed(),
        checksum,
        "traffic hints four-pass baseline",
    )
}

fn finish_measurement(
    mut samples: Vec<u128>,
    total_elapsed: std::time::Duration,
    checksum: usize,
    label: &str,
) -> ScheduleMeasurement {
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == usize::MAX {
        eprintln!("{label} metadata perf checksum guard tripped");
    }
    ScheduleMeasurement {
        total_elapsed,
        p95_ns,
    }
}
