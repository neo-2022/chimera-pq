use chimera_bootstrap::peer_update::{
    PeerUpdateStateAdvertisement, PeerUpdateStatePublishAction, decide_peer_update_state_publish,
    parse_existing_peer_update_state,
};
use chimera_carrier::peer_egress::lane_binding::{
    TransitLaneDocument, parse_transit_lane_document, render_transit_lane_document,
    transit_lane_document_from_mesh_plan,
};
use chimera_carrier::peer_egress::live_bindings::live_binding_reload_index_perf_smoke;
use chimera_mesh::{
    MeshCarrierLaneBinding, MeshDiscoveryRecord, MeshJoinRequest, MeshMultipathFlowAction,
    MeshMultipathFlowKey, MeshMultipathLane, MeshMultipathLaneRole, MeshMultipathMode,
    MeshMultipathRebuildPolicy, MeshMultipathSchedule, MeshPathPolicy, MeshPeerHealth,
    MeshPeerPerformance, MeshPeerTablePolicy, MeshRouteBindingId, MeshRuntime, plan_multipath_flow,
};
use std::time::{Duration, Instant};

use crate::Language;

const DEFAULT_ITERATIONS: usize = 100_000;
const ACTIVE_BINDING_COUNT: usize = 16;
const PATH_PLANNER_PEER_COUNT: usize = 64;
const PATH_PLANNER_MAX_ITERATIONS: usize = 10_000;
const LIVE_DPS_PLAN_PATH_MAX_ITERATIONS: usize = 10_000;
const DISCOVERY_REBUILD_MAX_ITERATIONS: usize = 10_000;
const DISCOVERY_UPDATE_NOOP_MAX_ITERATIONS: usize = 10_000;
const STATUS_EXPLAIN_MAX_ITERATIONS: usize = 10_000;
const DISCOVERY_REBUILD_SOURCE: &str = "metadata-perf-rebuild";
const DISCOVERY_UPDATE_NOOP_SOURCE: &str = "metadata-perf-discovery-noop";
const PLAN_SNAPSHOT_ACCESS_MAX_ITERATIONS: usize = 10_000;
const LANE_DOCUMENT_RENDER_PARSE_MAX_ITERATIONS: usize = 10_000;
const PEER_UPDATE_STATE_PUBLISH_MAX_ITERATIONS: usize = 10_000;
const SAMPLE_COUNT: usize = 200;
const PEER_UPDATE_STATE_SHA256: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const LIVE_DPS_PLAN_PATH_PAYLOAD: &str = concat!(
    "mesh_allowed_regions=eu;",
    "mesh_max_peers=1;",
    "mesh_max_selected_per_region=1;",
    "mesh_min_distinct_regions=1;",
    "mesh_traffic_class=gaming_fps;",
    "mesh_multipath_mode=standby_only;",
    "mesh_continuity_policy=same_egress_only;",
    "mesh_route_binding_id=7005"
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MetadataPerfOptions {
    pub(crate) iterations: usize,
    pub(crate) min_fast_ops: Option<u64>,
    pub(crate) json_output: bool,
}

impl Default for MetadataPerfOptions {
    fn default() -> Self {
        Self {
            iterations: DEFAULT_ITERATIONS,
            min_fast_ops: None,
            json_output: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MetadataPerfResult {
    pub(crate) iterations: usize,
    pub(crate) active_bindings: usize,
    pub(crate) fast_sorted_ops_per_sec: f64,
    pub(crate) slow_sorted_fallback_ops_per_sec: f64,
    pub(crate) fast_p95_ns: u128,
    pub(crate) slow_sorted_fallback_p95_ns: u128,
    pub(crate) fast_vs_fallback_speedup_pct: f64,
    pub(crate) path_planner_iterations: usize,
    pub(crate) path_planner_peer_count: usize,
    pub(crate) path_planner_candidate_snapshot_ops_per_sec: f64,
    pub(crate) path_planner_candidate_snapshot_p95_ns: u128,
    pub(crate) live_dps_plan_path_from_payload_iterations: usize,
    pub(crate) live_dps_plan_path_from_payload_peer_count: usize,
    pub(crate) live_dps_plan_path_from_payload_ops_per_sec: f64,
    pub(crate) live_dps_plan_path_from_payload_p95_ns: u128,
    pub(crate) live_dps_plan_core_from_payload_ops_per_sec: f64,
    pub(crate) live_dps_plan_core_from_payload_p95_ns: u128,
    pub(crate) status_explain_iterations: usize,
    pub(crate) status_explain_peer_count: usize,
    pub(crate) status_explain_ops_per_sec: f64,
    pub(crate) status_explain_p95_ns: u128,
    pub(crate) discovery_rebuild_iterations: usize,
    pub(crate) discovery_rebuild_peer_count: usize,
    pub(crate) discovery_rebuild_fingerprint_ops_per_sec: f64,
    pub(crate) discovery_rebuild_fingerprint_p95_ns: u128,
    pub(crate) discovery_update_noop_iterations: usize,
    pub(crate) discovery_update_noop_ops_per_sec: f64,
    pub(crate) discovery_update_noop_p95_ns: u128,
    pub(crate) lane_document_plan_snapshot_iterations: usize,
    pub(crate) lane_document_plan_snapshot_borrowed_ops_per_sec: f64,
    pub(crate) lane_document_plan_snapshot_borrowed_p95_ns: u128,
    pub(crate) lane_document_plan_snapshot_owned_ops_per_sec: f64,
    pub(crate) lane_document_plan_snapshot_owned_p95_ns: u128,
    pub(crate) lane_document_render_parse_iterations: usize,
    pub(crate) lane_document_render_parse_ops_per_sec: f64,
    pub(crate) lane_document_render_parse_p95_ns: u128,
    pub(crate) peer_update_state_publish_iterations: usize,
    pub(crate) peer_update_state_publish_noop_ops_per_sec: f64,
    pub(crate) peer_update_state_publish_noop_p95_ns: u128,
    pub(crate) peer_update_state_publish_changed_generation_ops_per_sec: f64,
    pub(crate) peer_update_state_publish_changed_generation_p95_ns: u128,
    pub(crate) live_binding_reload_index_iterations: usize,
    pub(crate) live_binding_reload_index_spawn_count: usize,
    pub(crate) live_binding_reload_index_ops_per_sec: f64,
    pub(crate) live_binding_reload_index_p95_ns: u128,
}

pub(crate) fn run_metadata_perf_smoke(lang: Language, args: &[String]) -> i32 {
    let options = match parse_metadata_perf_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("Metadata perf options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] metadata-perf-smoke [--iterations <n>] [--min-fast-ops <n>] [--json]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций metadata-perf: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] metadata-perf-smoke [--iterations <n>] [--min-fast-ops <n>] [--json]"
                    );
                }
            }
            return 2;
        }
    };
    let result = execute_metadata_perf_smoke(options);
    if let Some(min_fast_ops) = options.min_fast_ops
        && result.fast_sorted_ops_per_sec < min_fast_ops as f64
    {
        eprintln!(
            "metadata perf failed: fast sorted ops/sec {:.0} is below required minimum {}",
            result.fast_sorted_ops_per_sec, min_fast_ops
        );
        return 1;
    }
    if options.json_output {
        println!("{}", render_metadata_perf_json(&result));
        return 0;
    }
    match lang {
        Language::En => {
            println!("Metadata performance check: ok");
            println!("Hot path: multipath_flow_lane_selection");
            println!("Hot path: path_planner_candidate_snapshot");
            println!("Hot path: discovery_rebuild_fingerprint");
            println!("Hot path: discovery_update_noop_dirty_set");
            println!("Hot path: lane_document_plan_snapshot_access");
            println!("Hot path: lane_document_render_parse");
            println!("Hot path: peer_update_state_publish_generation");
            println!("Hot path: live_binding_reload_index");
            println!("Hot path: live_dps_plan_path_from_payload");
            println!("Hot path: live_dps_plan_core_from_payload");
            println!("Hot path: status_explain");
            println!("Scope: hot metadata only");
            println!("Iterations: {}", result.iterations);
            println!(
                "Path planner iterations: {} over {} peers",
                result.path_planner_iterations, result.path_planner_peer_count
            );
            println!(
                "Fast sorted path: {:.0} ops/sec, p95 {} ns",
                result.fast_sorted_ops_per_sec, result.fast_p95_ns
            );
            println!(
                "Slow sorted fallback: {:.0} ops/sec, p95 {} ns",
                result.slow_sorted_fallback_ops_per_sec, result.slow_sorted_fallback_p95_ns
            );
            println!(
                "Fast vs fallback: {:.2}% faster",
                result.fast_vs_fallback_speedup_pct
            );
            println!(
                "Path planner candidate snapshot: {:.0} ops/sec, p95 {} ns",
                result.path_planner_candidate_snapshot_ops_per_sec,
                result.path_planner_candidate_snapshot_p95_ns
            );
            println!(
                "Discovery rebuild fingerprint: {:.0} ops/sec, p95 {} ns",
                result.discovery_rebuild_fingerprint_ops_per_sec,
                result.discovery_rebuild_fingerprint_p95_ns
            );
            println!(
                "Discovery update no-op dirty-set: {:.0} ops/sec, p95 {} ns",
                result.discovery_update_noop_ops_per_sec, result.discovery_update_noop_p95_ns
            );
            println!(
                "Lane document plan snapshot borrowed: {:.0} ops/sec, p95 {} ns",
                result.lane_document_plan_snapshot_borrowed_ops_per_sec,
                result.lane_document_plan_snapshot_borrowed_p95_ns
            );
            println!(
                "Lane document plan snapshot owned: {:.0} ops/sec, p95 {} ns",
                result.lane_document_plan_snapshot_owned_ops_per_sec,
                result.lane_document_plan_snapshot_owned_p95_ns
            );
            println!(
                "Lane document render/parse: {:.0} ops/sec, p95 {} ns",
                result.lane_document_render_parse_ops_per_sec,
                result.lane_document_render_parse_p95_ns
            );
            println!(
                "Peer update state publish no-op: {:.0} ops/sec, p95 {} ns",
                result.peer_update_state_publish_noop_ops_per_sec,
                result.peer_update_state_publish_noop_p95_ns
            );
            println!(
                "Peer update state publish changed generation: {:.0} ops/sec, p95 {} ns",
                result.peer_update_state_publish_changed_generation_ops_per_sec,
                result.peer_update_state_publish_changed_generation_p95_ns
            );
            println!(
                "Live binding reload index: {:.0} ops/sec, p95 {} ns, spawn_count {}",
                result.live_binding_reload_index_ops_per_sec,
                result.live_binding_reload_index_p95_ns,
                result.live_binding_reload_index_spawn_count
            );
            println!(
                "Live DPS plan path iterations: {} over {} peers",
                result.live_dps_plan_path_from_payload_iterations,
                result.live_dps_plan_path_from_payload_peer_count
            );
            println!(
                "Live DPS plan path from payload: {:.0} ops/sec, p95 {} ns",
                result.live_dps_plan_path_from_payload_ops_per_sec,
                result.live_dps_plan_path_from_payload_p95_ns
            );
            println!(
                "Live DPS plan core from payload: {:.0} ops/sec, p95 {} ns",
                result.live_dps_plan_core_from_payload_ops_per_sec,
                result.live_dps_plan_core_from_payload_p95_ns
            );
            println!(
                "Status explain: {:.0} ops/sec, p95 {} ns over {} peers",
                result.status_explain_ops_per_sec,
                result.status_explain_p95_ns,
                result.status_explain_peer_count
            );
            println!("Transit payload: opaque sealed payload untouched");
            println!("Network state: not modified");
        }
        Language::Ru => {
            println!("Проверка скорости metadata: ok");
            println!("Горячий путь: multipath_flow_lane_selection");
            println!("Горячий путь: path_planner_candidate_snapshot");
            println!("Горячий путь: discovery_rebuild_fingerprint");
            println!("Горячий путь: discovery_update_noop_dirty_set");
            println!("Горячий путь: lane_document_plan_snapshot_access");
            println!("Горячий путь: lane_document_render_parse");
            println!("Горячий путь: peer_update_state_publish_generation");
            println!("Горячий путь: live_binding_reload_index");
            println!("Горячий путь: live_dps_plan_path_from_payload");
            println!("Горячий путь: live_dps_plan_core_from_payload");
            println!("Горячий путь: status_explain");
            println!("Область: только служебная metadata");
            println!("Итераций: {}", result.iterations);
            println!(
                "Path planner итераций: {} по {} peers",
                result.path_planner_iterations, result.path_planner_peer_count
            );
            println!(
                "Быстрый sorted path: {:.0} ops/сек, p95 {} нс",
                result.fast_sorted_ops_per_sec, result.fast_p95_ns
            );
            println!(
                "Slow fallback: {:.0} ops/сек, p95 {} нс",
                result.slow_sorted_fallback_ops_per_sec, result.slow_sorted_fallback_p95_ns
            );
            println!(
                "Быстрый путь быстрее fallback на {:.2}%",
                result.fast_vs_fallback_speedup_pct
            );
            println!(
                "Path planner candidate snapshot: {:.0} ops/сек, p95 {} нс",
                result.path_planner_candidate_snapshot_ops_per_sec,
                result.path_planner_candidate_snapshot_p95_ns
            );
            println!(
                "Discovery rebuild fingerprint: {:.0} ops/сек, p95 {} нс",
                result.discovery_rebuild_fingerprint_ops_per_sec,
                result.discovery_rebuild_fingerprint_p95_ns
            );
            println!(
                "Discovery update no-op dirty-set: {:.0} ops/сек, p95 {} нс",
                result.discovery_update_noop_ops_per_sec, result.discovery_update_noop_p95_ns
            );
            println!(
                "Lane document plan snapshot borrowed: {:.0} ops/сек, p95 {} нс",
                result.lane_document_plan_snapshot_borrowed_ops_per_sec,
                result.lane_document_plan_snapshot_borrowed_p95_ns
            );
            println!(
                "Lane document plan snapshot owned: {:.0} ops/сек, p95 {} нс",
                result.lane_document_plan_snapshot_owned_ops_per_sec,
                result.lane_document_plan_snapshot_owned_p95_ns
            );
            println!(
                "Lane document render/parse: {:.0} ops/сек, p95 {} нс",
                result.lane_document_render_parse_ops_per_sec,
                result.lane_document_render_parse_p95_ns
            );
            println!(
                "Peer update state publish no-op: {:.0} ops/сек, p95 {} нс",
                result.peer_update_state_publish_noop_ops_per_sec,
                result.peer_update_state_publish_noop_p95_ns
            );
            println!(
                "Peer update state publish changed generation: {:.0} ops/сек, p95 {} нс",
                result.peer_update_state_publish_changed_generation_ops_per_sec,
                result.peer_update_state_publish_changed_generation_p95_ns
            );
            println!(
                "Live binding reload index: {:.0} ops/сек, p95 {} нс, spawn_count {}",
                result.live_binding_reload_index_ops_per_sec,
                result.live_binding_reload_index_p95_ns,
                result.live_binding_reload_index_spawn_count
            );
            println!(
                "Live DPS plan path iterations: {} по {} peers",
                result.live_dps_plan_path_from_payload_iterations,
                result.live_dps_plan_path_from_payload_peer_count
            );
            println!(
                "Live DPS plan path from payload: {:.0} ops/сек, p95 {} нс",
                result.live_dps_plan_path_from_payload_ops_per_sec,
                result.live_dps_plan_path_from_payload_p95_ns
            );
            println!(
                "Live DPS plan core from payload: {:.0} ops/сек, p95 {} нс",
                result.live_dps_plan_core_from_payload_ops_per_sec,
                result.live_dps_plan_core_from_payload_p95_ns
            );
            println!(
                "Status explain: {:.0} ops/сек, p95 {} нс по {} peers",
                result.status_explain_ops_per_sec,
                result.status_explain_p95_ns,
                result.status_explain_peer_count
            );
            println!("Transit payload: opaque sealed payload untouched");
            println!("Состояние сети: не изменялось");
        }
    }
    0
}

pub(crate) fn parse_metadata_perf_options(args: &[String]) -> Result<MetadataPerfOptions, String> {
    let mut options = MetadataPerfOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--json" {
            options.json_output = true;
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        let parsed = value
            .parse::<u64>()
            .map_err(|_| format!("invalid integer for {flag}: {value}"))?;
        match flag {
            "--iterations" => {
                if parsed == 0 {
                    return Err("iterations must be > 0".to_string());
                }
                options.iterations = usize::try_from(parsed)
                    .map_err(|_| "iterations value is too large".to_string())?;
            }
            "--min-fast-ops" => options.min_fast_ops = Some(parsed),
            _ => return Err(format!("unknown option: {flag}")),
        }
        index += 2;
    }
    Ok(options)
}

pub(crate) fn execute_metadata_perf_smoke(options: MetadataPerfOptions) -> MetadataPerfResult {
    let sorted_schedule = metadata_schedule(false);
    let unsorted_schedule = metadata_schedule(true);
    let sample_count = options.iterations.clamp(1, SAMPLE_COUNT);
    let batch_size = options.iterations.div_ceil(sample_count);
    let measured_iterations = sample_count.saturating_mul(batch_size);

    let fast = measure_schedule(&sorted_schedule, sample_count, batch_size);
    let slow = measure_schedule(&unsorted_schedule, sample_count, batch_size);
    let fast_sorted_ops_per_sec = ops_per_sec(measured_iterations, fast.total_elapsed);
    let slow_sorted_fallback_ops_per_sec = ops_per_sec(measured_iterations, slow.total_elapsed);
    let fast_vs_fallback_speedup_pct = if slow_sorted_fallback_ops_per_sec <= 0.0 {
        0.0
    } else {
        ((fast_sorted_ops_per_sec - slow_sorted_fallback_ops_per_sec)
            / slow_sorted_fallback_ops_per_sec)
            * 100.0
    };
    let path_planner_iterations = measured_iterations.clamp(1, PATH_PLANNER_MAX_ITERATIONS);
    let (runtime, request, policy) = metadata_path_planner_fixture();
    let path_planner_sample_count = path_planner_iterations.clamp(1, SAMPLE_COUNT);
    let path_planner_batch_size = path_planner_iterations.div_ceil(path_planner_sample_count);
    let path_planner_measured_iterations =
        path_planner_sample_count.saturating_mul(path_planner_batch_size);
    let path_planner = measure_path_planner(
        &runtime,
        &request,
        &policy,
        path_planner_sample_count,
        path_planner_batch_size,
    );
    let mut discovery_runtime = metadata_rebuild_fixture();
    let discovery_rebuild_iterations =
        measured_iterations.clamp(1, DISCOVERY_REBUILD_MAX_ITERATIONS);
    let discovery_rebuild_sample_count = discovery_rebuild_iterations.clamp(1, SAMPLE_COUNT);
    let discovery_rebuild_batch_size =
        discovery_rebuild_iterations.div_ceil(discovery_rebuild_sample_count);
    let discovery_rebuild_measured_iterations =
        discovery_rebuild_sample_count.saturating_mul(discovery_rebuild_batch_size);
    let discovery_rebuild = measure_discovery_rebuild_trigger(
        &mut discovery_runtime,
        DISCOVERY_REBUILD_SOURCE,
        discovery_rebuild_sample_count,
        discovery_rebuild_batch_size,
    );
    let mut discovery_update_noop_runtime = metadata_discovery_update_noop_fixture();
    let discovery_update_noop_record = metadata_discovery_update_noop_record();
    let discovery_update_noop_iterations =
        measured_iterations.clamp(1, DISCOVERY_UPDATE_NOOP_MAX_ITERATIONS);
    let discovery_update_noop_sample_count =
        discovery_update_noop_iterations.clamp(1, SAMPLE_COUNT);
    let discovery_update_noop_batch_size =
        discovery_update_noop_iterations.div_ceil(discovery_update_noop_sample_count);
    let discovery_update_noop_measured_iterations =
        discovery_update_noop_sample_count.saturating_mul(discovery_update_noop_batch_size);
    let discovery_update_noop = measure_discovery_update_noop_dirty_set(
        &mut discovery_update_noop_runtime,
        DISCOVERY_UPDATE_NOOP_SOURCE,
        &discovery_update_noop_record,
        discovery_update_noop_sample_count,
        discovery_update_noop_batch_size,
    );
    let lane_document = metadata_lane_document_fixture();
    let lane_document_plan_snapshot_iterations =
        measured_iterations.clamp(1, PLAN_SNAPSHOT_ACCESS_MAX_ITERATIONS);
    let lane_document_plan_snapshot_sample_count =
        lane_document_plan_snapshot_iterations.clamp(1, SAMPLE_COUNT);
    let lane_document_plan_snapshot_batch_size =
        lane_document_plan_snapshot_iterations.div_ceil(lane_document_plan_snapshot_sample_count);
    let lane_document_plan_snapshot_measured_iterations = lane_document_plan_snapshot_sample_count
        .saturating_mul(lane_document_plan_snapshot_batch_size);
    let lane_document_plan_snapshot_borrowed = measure_plan_snapshot_borrowed(
        &lane_document,
        lane_document_plan_snapshot_sample_count,
        lane_document_plan_snapshot_batch_size,
    );
    let lane_document_plan_snapshot_owned = measure_plan_snapshot_owned(
        &lane_document,
        lane_document_plan_snapshot_sample_count,
        lane_document_plan_snapshot_batch_size,
    );
    let lane_document_render_parse_iterations =
        measured_iterations.clamp(1, LANE_DOCUMENT_RENDER_PARSE_MAX_ITERATIONS);
    let lane_document_render_parse_sample_count =
        lane_document_render_parse_iterations.clamp(1, SAMPLE_COUNT);
    let lane_document_render_parse_batch_size =
        lane_document_render_parse_iterations.div_ceil(lane_document_render_parse_sample_count);
    let lane_document_render_parse_measured_iterations = lane_document_render_parse_sample_count
        .saturating_mul(lane_document_render_parse_batch_size);
    let lane_document_render_parse = measure_lane_document_render_parse(
        &lane_document,
        lane_document_render_parse_sample_count,
        lane_document_render_parse_batch_size,
    );
    let peer_update_state_publish_iterations =
        measured_iterations.clamp(1, PEER_UPDATE_STATE_PUBLISH_MAX_ITERATIONS);
    let peer_update_state_publish_sample_count =
        peer_update_state_publish_iterations.clamp(1, SAMPLE_COUNT);
    let peer_update_state_publish_batch_size =
        peer_update_state_publish_iterations.div_ceil(peer_update_state_publish_sample_count);
    let peer_update_state_publish_measured_iterations =
        peer_update_state_publish_sample_count.saturating_mul(peer_update_state_publish_batch_size);
    let peer_update_state_publish_noop = measure_peer_update_state_publish(
        PeerUpdateStatePublishFixtureKind::Noop,
        peer_update_state_publish_sample_count,
        peer_update_state_publish_batch_size,
    );
    let peer_update_state_publish_changed = measure_peer_update_state_publish(
        PeerUpdateStatePublishFixtureKind::ChangedGeneration,
        peer_update_state_publish_sample_count,
        peer_update_state_publish_batch_size,
    );
    let live_dps_plan_path_iterations =
        measured_iterations.clamp(1, LIVE_DPS_PLAN_PATH_MAX_ITERATIONS);
    let live_dps_plan_path_sample_count = live_dps_plan_path_iterations.clamp(1, SAMPLE_COUNT);
    let live_dps_plan_path_batch_size =
        live_dps_plan_path_iterations.div_ceil(live_dps_plan_path_sample_count);
    let live_dps_plan_path_measured_iterations =
        live_dps_plan_path_sample_count.saturating_mul(live_dps_plan_path_batch_size);
    let live_dps_plan_path = measure_live_dps_plan_path(
        &runtime,
        &request,
        live_dps_plan_path_sample_count,
        live_dps_plan_path_batch_size,
    );
    let live_dps_plan_core = measure_live_dps_plan_core(
        &runtime,
        &request,
        live_dps_plan_path_sample_count,
        live_dps_plan_path_batch_size,
    );
    let status_explain_runtime = metadata_rebuild_fixture();
    let status_explain_iterations = measured_iterations.clamp(1, STATUS_EXPLAIN_MAX_ITERATIONS);
    let status_explain_sample_count = status_explain_iterations.clamp(1, SAMPLE_COUNT);
    let status_explain_batch_size = status_explain_iterations.div_ceil(status_explain_sample_count);
    let status_explain_measured_iterations =
        status_explain_sample_count.saturating_mul(status_explain_batch_size);
    let status_explain = measure_status_explain(
        &status_explain_runtime,
        status_explain_sample_count,
        status_explain_batch_size,
    );
    let live_binding_reload_index = live_binding_reload_index_perf_smoke(measured_iterations)
        .unwrap_or_else(|error| unreachable!("{error}"));

    MetadataPerfResult {
        iterations: measured_iterations,
        active_bindings: ACTIVE_BINDING_COUNT,
        fast_sorted_ops_per_sec,
        slow_sorted_fallback_ops_per_sec,
        fast_p95_ns: fast.p95_ns,
        slow_sorted_fallback_p95_ns: slow.p95_ns,
        fast_vs_fallback_speedup_pct,
        path_planner_iterations: path_planner_measured_iterations,
        path_planner_peer_count: PATH_PLANNER_PEER_COUNT,
        path_planner_candidate_snapshot_ops_per_sec: ops_per_sec(
            path_planner_measured_iterations,
            path_planner.total_elapsed,
        ),
        path_planner_candidate_snapshot_p95_ns: path_planner.p95_ns,
        discovery_rebuild_iterations: discovery_rebuild_measured_iterations,
        discovery_rebuild_peer_count: PATH_PLANNER_PEER_COUNT,
        discovery_rebuild_fingerprint_ops_per_sec: ops_per_sec(
            discovery_rebuild_measured_iterations,
            discovery_rebuild.total_elapsed,
        ),
        discovery_rebuild_fingerprint_p95_ns: discovery_rebuild.p95_ns,
        discovery_update_noop_iterations: discovery_update_noop_measured_iterations,
        discovery_update_noop_ops_per_sec: ops_per_sec(
            discovery_update_noop_measured_iterations,
            discovery_update_noop.total_elapsed,
        ),
        discovery_update_noop_p95_ns: discovery_update_noop.p95_ns,
        lane_document_plan_snapshot_iterations: lane_document_plan_snapshot_measured_iterations,
        lane_document_plan_snapshot_borrowed_ops_per_sec: ops_per_sec(
            lane_document_plan_snapshot_measured_iterations,
            lane_document_plan_snapshot_borrowed.total_elapsed,
        ),
        lane_document_plan_snapshot_borrowed_p95_ns: lane_document_plan_snapshot_borrowed.p95_ns,
        lane_document_plan_snapshot_owned_ops_per_sec: ops_per_sec(
            lane_document_plan_snapshot_measured_iterations,
            lane_document_plan_snapshot_owned.total_elapsed,
        ),
        lane_document_plan_snapshot_owned_p95_ns: lane_document_plan_snapshot_owned.p95_ns,
        lane_document_render_parse_iterations: lane_document_render_parse_measured_iterations,
        lane_document_render_parse_ops_per_sec: ops_per_sec(
            lane_document_render_parse_measured_iterations,
            lane_document_render_parse.total_elapsed,
        ),
        lane_document_render_parse_p95_ns: lane_document_render_parse.p95_ns,
        peer_update_state_publish_iterations: peer_update_state_publish_measured_iterations,
        peer_update_state_publish_noop_ops_per_sec: ops_per_sec(
            peer_update_state_publish_measured_iterations,
            peer_update_state_publish_noop.total_elapsed,
        ),
        peer_update_state_publish_noop_p95_ns: peer_update_state_publish_noop.p95_ns,
        peer_update_state_publish_changed_generation_ops_per_sec: ops_per_sec(
            peer_update_state_publish_measured_iterations,
            peer_update_state_publish_changed.total_elapsed,
        ),
        peer_update_state_publish_changed_generation_p95_ns: peer_update_state_publish_changed
            .p95_ns,
        live_dps_plan_path_from_payload_iterations: live_dps_plan_path_measured_iterations,
        live_dps_plan_path_from_payload_peer_count: PATH_PLANNER_PEER_COUNT,
        live_dps_plan_path_from_payload_ops_per_sec: ops_per_sec(
            live_dps_plan_path_measured_iterations,
            live_dps_plan_path.total_elapsed,
        ),
        live_dps_plan_path_from_payload_p95_ns: live_dps_plan_path.p95_ns,
        live_dps_plan_core_from_payload_ops_per_sec: ops_per_sec(
            live_dps_plan_path_measured_iterations,
            live_dps_plan_core.total_elapsed,
        ),
        live_dps_plan_core_from_payload_p95_ns: live_dps_plan_core.p95_ns,
        status_explain_iterations: status_explain_measured_iterations,
        status_explain_peer_count: PATH_PLANNER_PEER_COUNT,
        status_explain_ops_per_sec: ops_per_sec(
            status_explain_measured_iterations,
            status_explain.total_elapsed,
        ),
        status_explain_p95_ns: status_explain.p95_ns,
        live_binding_reload_index_iterations: live_binding_reload_index.iterations,
        live_binding_reload_index_spawn_count: live_binding_reload_index.spawn_count,
        live_binding_reload_index_ops_per_sec: live_binding_reload_index.ops_per_sec,
        live_binding_reload_index_p95_ns: live_binding_reload_index.p95_ns,
    }
}

pub(crate) fn render_metadata_perf_json(result: &MetadataPerfResult) -> String {
    format!(
        "{{\"status\":\"ok\",\"kind\":\"metadata_perf_smoke\",\"hot_paths\":[\"multipath_flow_lane_selection\",\"path_planner_candidate_snapshot\",\"discovery_rebuild_fingerprint\",\"discovery_update_noop_dirty_set\",\"lane_document_plan_snapshot_access\",\"lane_document_render_parse\",\"peer_update_state_publish_generation\",\"live_binding_reload_index\",\"live_dps_plan_path_from_payload\",\"live_dps_plan_core_from_payload\",\"status_explain\"],\"scope\":\"hot_metadata_only\",\"transit_payload_policy\":\"opaque_sealed_payload_untouched\",\"iterations\":{},\"active_bindings\":{},\"fast_sorted_ops_per_sec\":{:.0},\"slow_sorted_fallback_ops_per_sec\":{:.0},\"fast_p95_ns\":{},\"slow_sorted_fallback_p95_ns\":{},\"fast_vs_fallback_speedup_pct\":{:.2},\"path_planner_iterations\":{},\"path_planner_peer_count\":{},\"path_planner_candidate_snapshot_ops_per_sec\":{:.0},\"path_planner_candidate_snapshot_p95_ns\":{},\"discovery_rebuild_iterations\":{},\"discovery_rebuild_peer_count\":{},\"discovery_rebuild_fingerprint_ops_per_sec\":{:.0},\"discovery_rebuild_fingerprint_p95_ns\":{},\"discovery_update_noop_iterations\":{},\"discovery_update_noop_ops_per_sec\":{:.0},\"discovery_update_noop_p95_ns\":{},\"lane_document_plan_snapshot_iterations\":{},\"lane_document_plan_snapshot_borrowed_ops_per_sec\":{:.0},\"lane_document_plan_snapshot_borrowed_p95_ns\":{},\"lane_document_plan_snapshot_owned_ops_per_sec\":{:.0},\"lane_document_plan_snapshot_owned_p95_ns\":{},\"lane_document_render_parse_iterations\":{},\"lane_document_render_parse_ops_per_sec\":{:.0},\"lane_document_render_parse_p95_ns\":{},\"peer_update_state_publish_iterations\":{},\"peer_update_state_publish_noop_ops_per_sec\":{:.0},\"peer_update_state_publish_noop_p95_ns\":{},\"peer_update_state_publish_changed_generation_ops_per_sec\":{:.0},\"peer_update_state_publish_changed_generation_p95_ns\":{},\"live_dps_plan_path_from_payload_iterations\":{},\"live_dps_plan_path_from_payload_peer_count\":{},\"live_dps_plan_path_from_payload_ops_per_sec\":{:.0},\"live_dps_plan_path_from_payload_p95_ns\":{},\"live_dps_plan_core_from_payload_ops_per_sec\":{:.0},\"live_dps_plan_core_from_payload_p95_ns\":{},\"status_explain_iterations\":{},\"status_explain_peer_count\":{},\"status_explain_ops_per_sec\":{:.0},\"status_explain_p95_ns\":{},\"live_binding_reload_index_iterations\":{},\"live_binding_reload_index_spawn_count\":{},\"live_binding_reload_index_ops_per_sec\":{:.0},\"live_binding_reload_index_p95_ns\":{},\"network_state\":\"not_modified\"}}",
        result.iterations,
        result.active_bindings,
        result.fast_sorted_ops_per_sec,
        result.slow_sorted_fallback_ops_per_sec,
        result.fast_p95_ns,
        result.slow_sorted_fallback_p95_ns,
        result.fast_vs_fallback_speedup_pct,
        result.path_planner_iterations,
        result.path_planner_peer_count,
        result.path_planner_candidate_snapshot_ops_per_sec,
        result.path_planner_candidate_snapshot_p95_ns,
        result.discovery_rebuild_iterations,
        result.discovery_rebuild_peer_count,
        result.discovery_rebuild_fingerprint_ops_per_sec,
        result.discovery_rebuild_fingerprint_p95_ns,
        result.discovery_update_noop_iterations,
        result.discovery_update_noop_ops_per_sec,
        result.discovery_update_noop_p95_ns,
        result.lane_document_plan_snapshot_iterations,
        result.lane_document_plan_snapshot_borrowed_ops_per_sec,
        result.lane_document_plan_snapshot_borrowed_p95_ns,
        result.lane_document_plan_snapshot_owned_ops_per_sec,
        result.lane_document_plan_snapshot_owned_p95_ns,
        result.lane_document_render_parse_iterations,
        result.lane_document_render_parse_ops_per_sec,
        result.lane_document_render_parse_p95_ns,
        result.peer_update_state_publish_iterations,
        result.peer_update_state_publish_noop_ops_per_sec,
        result.peer_update_state_publish_noop_p95_ns,
        result.peer_update_state_publish_changed_generation_ops_per_sec,
        result.peer_update_state_publish_changed_generation_p95_ns,
        result.live_dps_plan_path_from_payload_iterations,
        result.live_dps_plan_path_from_payload_peer_count,
        result.live_dps_plan_path_from_payload_ops_per_sec,
        result.live_dps_plan_path_from_payload_p95_ns,
        result.live_dps_plan_core_from_payload_ops_per_sec,
        result.live_dps_plan_core_from_payload_p95_ns,
        result.status_explain_iterations,
        result.status_explain_peer_count,
        result.status_explain_ops_per_sec,
        result.status_explain_p95_ns,
        result.live_binding_reload_index_iterations,
        result.live_binding_reload_index_spawn_count,
        result.live_binding_reload_index_ops_per_sec,
        result.live_binding_reload_index_p95_ns
    )
}

struct ScheduleMeasurement {
    total_elapsed: Duration,
    p95_ns: u128,
}

fn measure_schedule(
    schedule: &MeshMultipathSchedule,
    sample_count: usize,
    batch_size: usize,
) -> ScheduleMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for sample in 0..sample_count {
        let batch_start = Instant::now();
        for offset in 0..batch_size {
            let flow_index = sample.saturating_mul(batch_size).saturating_add(offset);
            let key = MeshMultipathFlowKey::from_stable_hash(flow_hash(flow_index));
            let plan = plan_multipath_flow(schedule, key);
            if plan.action == MeshMultipathFlowAction::Assigned {
                checksum = checksum.wrapping_add(plan.selected_lane_id.unwrap_or(usize::MAX));
            }
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    let total_elapsed = total_start.elapsed();
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == usize::MAX {
        eprintln!("metadata perf checksum guard tripped");
    }
    ScheduleMeasurement {
        total_elapsed,
        p95_ns,
    }
}

fn measure_path_planner(
    runtime: &MeshRuntime,
    request: &MeshJoinRequest,
    policy: &MeshPathPolicy,
    sample_count: usize,
    batch_size: usize,
) -> ScheduleMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for _sample in 0..sample_count {
        let batch_start = Instant::now();
        for _offset in 0..batch_size {
            let plan = runtime
                .plan_path(request, policy)
                .unwrap_or_else(|error| unreachable!("{error}"));
            checksum = checksum.wrapping_add(plan.selected_peers.len());
            checksum = checksum.wrapping_add(plan.explain.len());
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    let total_elapsed = total_start.elapsed();
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == usize::MAX {
        eprintln!("path planner metadata perf checksum guard tripped");
    }
    ScheduleMeasurement {
        total_elapsed,
        p95_ns,
    }
}

fn measure_discovery_rebuild_trigger(
    runtime: &mut MeshRuntime,
    source: &str,
    sample_count: usize,
    batch_size: usize,
) -> ScheduleMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for sample in 0..sample_count {
        let batch_start = Instant::now();
        for offset in 0..batch_size {
            let _sequence = sample.saturating_mul(batch_size).saturating_add(offset);
            runtime
                .merge_discovery(source, &[])
                .unwrap_or_else(|error| unreachable!("{error}"));
            checksum = checksum.wrapping_add(runtime.peer_count());
            checksum = checksum.wrapping_add(runtime.health_state_count());
            checksum = checksum.wrapping_add(runtime.source_count());
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    let total_elapsed = total_start.elapsed();
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == usize::MAX {
        eprintln!("discovery rebuild metadata perf checksum guard tripped");
    }
    ScheduleMeasurement {
        total_elapsed,
        p95_ns,
    }
}

fn metadata_discovery_update_noop_record() -> MeshDiscoveryRecord {
    MeshDiscoveryRecord {
        node_id: "perf-discovery-noop-node".to_string(),
        endpoint: "198.51.102.10:443".to_string(),
        region: "eu".to_string(),
        load_score: 10,
        reliability_score: 95,
    }
}

fn metadata_discovery_update_noop_fixture() -> MeshRuntime {
    let mut runtime = MeshRuntime::bootstrap("metadata-perf", "seed-a")
        .unwrap_or_else(|error| unreachable!("{error}"));
    runtime
        .set_peer_table_policy(MeshPeerTablePolicy {
            stale_after_ticks: 1_000_000,
            stability_window_ticks: 1_000_000,
            ..MeshPeerTablePolicy::default()
        })
        .unwrap_or_else(|error| unreachable!("{error}"));
    let record = metadata_discovery_update_noop_record();
    runtime
        .merge_discovery(DISCOVERY_UPDATE_NOOP_SOURCE, std::slice::from_ref(&record))
        .unwrap_or_else(|error| unreachable!("{error}"));
    if runtime.pending_multipath_rebuild_signal().is_none() {
        unreachable!("discovery update no-op fixture must start from a real peer-table rebuild");
    }
    let request = MeshJoinRequest {
        namespace: "metadata-perf".to_string(),
        node_name: "metadata-perf-discovery-noop".to_string(),
        invite_token: None,
    };
    let mut policy = MeshPathPolicy::default_auto();
    policy.allowed_regions = vec!["eu".to_string()];
    policy.max_peers = 1;
    policy.max_selected_per_region = 1;
    let rebuild_policy =
        MeshMultipathRebuildPolicy::new(1, 1).unwrap_or_else(|error| unreachable!("{error}"));
    let (_plan, _decision) = runtime
        .plan_path_with_pending_multipath_rebuild(&request, &policy, &rebuild_policy)
        .unwrap_or_else(|error| unreachable!("{error}"));
    if runtime.pending_multipath_rebuild_signal().is_some() {
        unreachable!("discovery update no-op fixture must start without pending rebuild");
    }
    runtime
}

fn measure_discovery_update_noop_dirty_set(
    runtime: &mut MeshRuntime,
    source: &str,
    record: &MeshDiscoveryRecord,
    sample_count: usize,
    batch_size: usize,
) -> ScheduleMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for sample in 0..sample_count {
        let batch_start = Instant::now();
        for offset in 0..batch_size {
            let _sequence = sample.saturating_mul(batch_size).saturating_add(offset);
            runtime
                .merge_discovery(source, std::slice::from_ref(record))
                .unwrap_or_else(|error| unreachable!("{error}"));
            if runtime.pending_multipath_rebuild_signal().is_some() {
                unreachable!("identical discovery update must not set pending rebuild");
            }
            checksum = checksum.wrapping_add(runtime.peer_count());
            checksum = checksum.wrapping_add(runtime.source_count());
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    let total_elapsed = total_start.elapsed();
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == usize::MAX {
        eprintln!("discovery update no-op metadata perf checksum guard tripped");
    }
    ScheduleMeasurement {
        total_elapsed,
        p95_ns,
    }
}

fn measure_live_dps_plan_path(
    runtime: &MeshRuntime,
    request: &MeshJoinRequest,
    sample_count: usize,
    batch_size: usize,
) -> ScheduleMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for _sample in 0..sample_count {
        let batch_start = Instant::now();
        for _offset in 0..batch_size {
            let plan = runtime
                .plan_path_from_dps_payload(request, LIVE_DPS_PLAN_PATH_PAYLOAD)
                .unwrap_or_else(|error| unreachable!("{error}"));
            checksum = checksum.wrapping_add(plan.selected_peers.len());
            checksum = checksum.wrapping_add(plan.explain.len());
            checksum = checksum.wrapping_add(plan.multipath_schedule.carrier_lane_bindings.len());
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    let total_elapsed = total_start.elapsed();
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == usize::MAX {
        eprintln!("live dps plan path metadata perf checksum guard tripped");
    }
    ScheduleMeasurement {
        total_elapsed,
        p95_ns,
    }
}

fn measure_live_dps_plan_core(
    runtime: &MeshRuntime,
    request: &MeshJoinRequest,
    sample_count: usize,
    batch_size: usize,
) -> ScheduleMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for _sample in 0..sample_count {
        let batch_start = Instant::now();
        for _offset in 0..batch_size {
            let plan = runtime
                .plan_path_core_from_dps_payload(request, LIVE_DPS_PLAN_PATH_PAYLOAD)
                .unwrap_or_else(|error| unreachable!("{error}"));
            checksum = checksum.wrapping_add(plan.selected_peers.len());
            checksum = checksum.wrapping_add(plan.multipath_schedule.carrier_lane_bindings.len());
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    let total_elapsed = total_start.elapsed();
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == usize::MAX {
        eprintln!("live dps plan core metadata perf checksum guard tripped");
    }
    ScheduleMeasurement {
        total_elapsed,
        p95_ns,
    }
}

fn measure_status_explain(
    runtime: &MeshRuntime,
    sample_count: usize,
    batch_size: usize,
) -> ScheduleMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for sample in 0..sample_count {
        let batch_start = Instant::now();
        for offset in 0..batch_size {
            let _sequence = sample.saturating_mul(batch_size).saturating_add(offset);
            let lines = runtime.status_explain();
            checksum = checksum.wrapping_add(lines.len());
            checksum = checksum.wrapping_add(lines.first().map_or(0, |line| line.len()));
            checksum = checksum.wrapping_add(lines.last().map_or(0, |line| line.len()));
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    let total_elapsed = total_start.elapsed();
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == usize::MAX {
        eprintln!("status explain metadata perf checksum guard tripped");
    }
    ScheduleMeasurement {
        total_elapsed,
        p95_ns,
    }
}

fn metadata_path_planner_fixture() -> (MeshRuntime, MeshJoinRequest, MeshPathPolicy) {
    let mut runtime = MeshRuntime::bootstrap("metadata-perf", "seed-a")
        .unwrap_or_else(|error| unreachable!("{error}"));
    let regions = ["eu", "us", "ap", "EU"];
    let records: Vec<MeshDiscoveryRecord> = (0..PATH_PLANNER_PEER_COUNT)
        .map(|index| MeshDiscoveryRecord {
            node_id: format!("perf-node-{index:03}"),
            endpoint: format!("198.51.100.{}:443", (index % 200) + 1),
            region: regions[index % regions.len()].to_string(),
            load_score: ((index * 7) % 70) as u8,
            reliability_score: (75 + ((index * 5) % 25)) as u8,
        })
        .collect();
    runtime
        .merge_discovery("seed-b", &records)
        .unwrap_or_else(|error| unreachable!("{error}"));
    let request = MeshJoinRequest {
        namespace: "metadata-perf".to_string(),
        node_name: "metadata-perf-requester".to_string(),
        invite_token: None,
    };
    let policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string(), "us".to_string(), "ap".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 75,
        max_load_score: 80,
        max_peers: 8,
        prefer_region_diversity: true,
        max_selected_per_region: 4,
        min_distinct_regions: 3,
        path_profile_override: None,
        multipath_mode: None,
        multipath_demand: None,
        connect_fallback_ports: vec![443, 8443],
    };
    (runtime, request, policy)
}

fn metadata_rebuild_fixture() -> MeshRuntime {
    let mut runtime = MeshRuntime::bootstrap("metadata-perf", "seed-a")
        .unwrap_or_else(|error| unreachable!("{error}"));
    let table_policy = MeshPeerTablePolicy {
        stale_after_ticks: 1_000_000,
        ..MeshPeerTablePolicy::default()
    };
    runtime
        .set_peer_table_policy(table_policy)
        .unwrap_or_else(|error| unreachable!("{error}"));
    let regions = ["eu", "us", "ap", "EU"];
    let records: Vec<MeshDiscoveryRecord> = (0..PATH_PLANNER_PEER_COUNT)
        .map(|index| MeshDiscoveryRecord {
            node_id: format!("perf-rebuild-node-{index:03}"),
            endpoint: format!("198.51.101.{}:443", (index % 200) + 1),
            region: regions[index % regions.len()].to_string(),
            load_score: ((index * 11) % 70) as u8,
            reliability_score: (70 + ((index * 3) % 30)) as u8,
        })
        .collect();
    runtime
        .merge_discovery(DISCOVERY_REBUILD_SOURCE, &records)
        .unwrap_or_else(|error| unreachable!("{error}"));
    runtime
        .update_health_state(&[
            MeshPeerHealth {
                node_id: "perf-rebuild-node-000".to_string(),
                healthy: false,
                cooldown_active: true,
            },
            MeshPeerHealth {
                node_id: "perf-rebuild-node-001".to_string(),
                healthy: true,
                cooldown_active: false,
            },
            MeshPeerHealth {
                node_id: "perf-rebuild-node-002".to_string(),
                healthy: false,
                cooldown_active: false,
            },
            MeshPeerHealth {
                node_id: "perf-rebuild-node-003".to_string(),
                healthy: true,
                cooldown_active: false,
            },
        ])
        .unwrap_or_else(|error| unreachable!("{error}"));
    runtime
        .update_peer_performance(&[
            MeshPeerPerformance {
                node_id: "perf-rebuild-node-000".to_string(),
                latency_ms: Some(15),
                throughput_mbps: Some(150),
            },
            MeshPeerPerformance {
                node_id: "perf-rebuild-node-001".to_string(),
                latency_ms: Some(25),
                throughput_mbps: Some(250),
            },
            MeshPeerPerformance {
                node_id: "perf-rebuild-node-002".to_string(),
                latency_ms: Some(35),
                throughput_mbps: Some(350),
            },
            MeshPeerPerformance {
                node_id: "perf-rebuild-node-003".to_string(),
                latency_ms: Some(45),
                throughput_mbps: Some(450),
            },
        ])
        .unwrap_or_else(|error| unreachable!("{error}"));
    runtime
}

fn metadata_lane_document_fixture() -> TransitLaneDocument {
    let mut runtime = MeshRuntime::bootstrap("metadata-perf", "seed-a")
        .unwrap_or_else(|error| unreachable!("{error}"));
    runtime
        .merge_discovery(
            "seed-b",
            &[
                MeshDiscoveryRecord {
                    node_id: "lane-doc-node-a".to_string(),
                    endpoint: "198.51.100.31:443".to_string(),
                    region: "eu".to_string(),
                    load_score: 20,
                    reliability_score: 90,
                },
                MeshDiscoveryRecord {
                    node_id: "lane-doc-node-b".to_string(),
                    endpoint: "198.51.100.32:443".to_string(),
                    region: "eu".to_string(),
                    load_score: 22,
                    reliability_score: 91,
                },
            ],
        )
        .unwrap_or_else(|error| unreachable!("{error}"));
    let plan = runtime
        .plan_path_from_dps_payload(
            &MeshJoinRequest {
                namespace: "metadata-perf".to_string(),
                node_name: "metadata-perf-lane-doc".to_string(),
                invite_token: None,
            },
            concat!(
                "mesh_allowed_regions=eu;",
                "mesh_max_peers=2;",
                "mesh_max_selected_per_region=2;",
                "mesh_multipath_mode=flow_shard;",
                "mesh_route_binding_id=7004"
            ),
        )
        .unwrap_or_else(|error| unreachable!("{error}"));
    transit_lane_document_from_mesh_plan(&plan).unwrap_or_else(|error| unreachable!("{error}"))
}

fn measure_plan_snapshot_borrowed(
    document: &TransitLaneDocument,
    sample_count: usize,
    batch_size: usize,
) -> ScheduleMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for _sample in 0..sample_count {
        let batch_start = Instant::now();
        for _offset in 0..batch_size {
            let plan = document
                .require_mesh_path_plan_ref()
                .unwrap_or_else(|error| unreachable!("{error}"));
            checksum = checksum.wrapping_add(plan.selected_peers.len());
            checksum = checksum.wrapping_add(plan.multipath_schedule.carrier_lane_bindings.len());
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    let total_elapsed = total_start.elapsed();
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == usize::MAX {
        eprintln!("lane document borrowed access metadata perf checksum guard tripped");
    }
    ScheduleMeasurement {
        total_elapsed,
        p95_ns,
    }
}

fn measure_plan_snapshot_owned(
    document: &TransitLaneDocument,
    sample_count: usize,
    batch_size: usize,
) -> ScheduleMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for _sample in 0..sample_count {
        let batch_start = Instant::now();
        for _offset in 0..batch_size {
            let plan = document
                .require_mesh_path_plan()
                .unwrap_or_else(|error| unreachable!("{error}"));
            checksum = checksum.wrapping_add(plan.selected_peers.len());
            checksum = checksum.wrapping_add(plan.multipath_schedule.carrier_lane_bindings.len());
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    let total_elapsed = total_start.elapsed();
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == usize::MAX {
        eprintln!("lane document owned access metadata perf checksum guard tripped");
    }
    ScheduleMeasurement {
        total_elapsed,
        p95_ns,
    }
}

fn measure_lane_document_render_parse(
    document: &TransitLaneDocument,
    sample_count: usize,
    batch_size: usize,
) -> ScheduleMeasurement {
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0usize;
    let total_start = Instant::now();
    for _sample in 0..sample_count {
        let batch_start = Instant::now();
        for _offset in 0..batch_size {
            let rendered = render_transit_lane_document(document)
                .unwrap_or_else(|error| unreachable!("{error}"));
            let parsed = parse_transit_lane_document(&rendered)
                .unwrap_or_else(|error| unreachable!("{error}"));
            checksum = checksum.wrapping_add(rendered.len());
            checksum = checksum.wrapping_add(parsed.registrations().len());
            checksum = checksum.wrapping_add(
                parsed
                    .mesh_path_plan_ref()
                    .map_or(0, |plan| plan.explain.len()),
            );
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    let total_elapsed = total_start.elapsed();
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == usize::MAX {
        eprintln!("lane document render/parse metadata perf checksum guard tripped");
    }
    ScheduleMeasurement {
        total_elapsed,
        p95_ns,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeerUpdateStatePublishFixtureKind {
    Noop,
    ChangedGeneration,
}

fn measure_peer_update_state_publish(
    kind: PeerUpdateStatePublishFixtureKind,
    sample_count: usize,
    batch_size: usize,
) -> ScheduleMeasurement {
    let existing_text = peer_update_state_fixture_json(1_000, 7);
    let existing = match parse_existing_peer_update_state(&existing_text)
        .unwrap_or_else(|error| unreachable!("{error}"))
    {
        Some(existing) => existing,
        None => unreachable!("peer update state fixture must parse"),
    };
    let mut samples = Vec::with_capacity(sample_count);
    let mut checksum = 0u64;
    let total_start = Instant::now();
    for sample in 0..sample_count {
        let batch_start = Instant::now();
        for offset in 0..batch_size {
            let sequence = sample.saturating_mul(batch_size).saturating_add(offset);
            let advertisement = match kind {
                PeerUpdateStatePublishFixtureKind::Noop => {
                    peer_update_state_advertisement("198.51.100.10:443", 1_010)
                }
                PeerUpdateStatePublishFixtureKind::ChangedGeneration => {
                    let port = 444 + (sequence % 2);
                    peer_update_state_advertisement_for_port(port, 1_010)
                }
            };
            let decision = decide_peer_update_state_publish(Some(&existing), advertisement)
                .unwrap_or_else(|error| unreachable!("{error}"));
            match kind {
                PeerUpdateStatePublishFixtureKind::Noop => {
                    if decision.action != PeerUpdateStatePublishAction::Noop {
                        unreachable!("peer update state no-op metric measured changed path");
                    }
                    checksum = checksum.wrapping_add(decision.endpoint_generation);
                }
                PeerUpdateStatePublishFixtureKind::ChangedGeneration => {
                    if decision.action != PeerUpdateStatePublishAction::Changed {
                        unreachable!("peer update state changed metric measured no-op path");
                    }
                    checksum = checksum.wrapping_add(decision.endpoint_generation);
                    checksum = checksum
                        .wrapping_add(decision.body.as_deref().map_or(0, |body| body.len() as u64));
                }
            }
        }
        let elapsed = batch_start.elapsed();
        samples.push(elapsed.as_nanos() / batch_size as u128);
    }
    let total_elapsed = total_start.elapsed();
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    if checksum == u64::MAX {
        eprintln!("peer update state publish metadata perf checksum guard tripped");
    }
    ScheduleMeasurement {
        total_elapsed,
        p95_ns,
    }
}

fn peer_update_state_fixture_json(endpoint_epoch: u64, endpoint_generation: u64) -> String {
    format!(
        "{{\"kind\":\"chimera_peer_update_serve_state\",\"status\":\"ready\",\"listen\":\"198.51.100.10:443\",\"base_url\":\"https://node.invalid\",\"update_bootstrap_url\":\"https://node.invalid/chimera.sh\",\"version\":\"1.2.3\",\"sha256\":\"{PEER_UPDATE_STATE_SHA256}\",\"endpoint_epoch\":{endpoint_epoch},\"endpoint_generation\":{endpoint_generation}}}"
    )
}

fn peer_update_state_advertisement(
    listen: &str,
    endpoint_epoch: u64,
) -> PeerUpdateStateAdvertisement<'static> {
    if listen == "198.51.100.10:443" {
        return PeerUpdateStateAdvertisement {
            listen: "198.51.100.10:443",
            base_url: Some("https://node.invalid"),
            update_bootstrap_url: Some("https://node.invalid/chimera.sh"),
            version: "1.2.3",
            sha256: PEER_UPDATE_STATE_SHA256,
            endpoint_epoch,
        };
    }
    peer_update_state_advertisement_for_port(444, endpoint_epoch)
}

fn peer_update_state_advertisement_for_port(
    port: usize,
    endpoint_epoch: u64,
) -> PeerUpdateStateAdvertisement<'static> {
    match port {
        444 => PeerUpdateStateAdvertisement {
            listen: "198.51.100.10:444",
            base_url: Some("https://node-alt-a.invalid"),
            update_bootstrap_url: Some("https://node-alt-a.invalid/chimera.sh"),
            version: "1.2.3",
            sha256: PEER_UPDATE_STATE_SHA256,
            endpoint_epoch,
        },
        _ => PeerUpdateStateAdvertisement {
            listen: "198.51.100.10:445",
            base_url: Some("https://node-alt-b.invalid"),
            update_bootstrap_url: Some("https://node-alt-b.invalid/chimera.sh"),
            version: "1.2.3",
            sha256: PEER_UPDATE_STATE_SHA256,
            endpoint_epoch,
        },
    }
}

fn metadata_schedule(unsorted: bool) -> MeshMultipathSchedule {
    let route_binding_id =
        MeshRouteBindingId::new(77_001).unwrap_or_else(|error| unreachable!("{error}"));
    let mut lanes = Vec::with_capacity(ACTIVE_BINDING_COUNT);
    let mut carrier_lane_bindings = Vec::with_capacity(ACTIVE_BINDING_COUNT);
    for lane_id in 0..ACTIVE_BINDING_COUNT {
        let weight_pct = if lane_id < 4 { 7 } else { 6 };
        let capacity_weight_pct = if lane_id < 10 { 6 } else { 5 };
        lanes.push(MeshMultipathLane {
            lane_id,
            peer_node_id: format!("node-{lane_id}"),
            role: MeshMultipathLaneRole::Active,
            weight_pct,
            capacity_weight_pct,
        });
        carrier_lane_bindings.push(MeshCarrierLaneBinding {
            route_binding_id,
            lane_id,
            peer_node_id: format!("node-{lane_id}"),
            carrier_endpoint: format!("peer-{lane_id}.invalid:443"),
            role: MeshMultipathLaneRole::Active,
            weight_pct,
            capacity_weight_pct,
        });
    }
    if unsorted {
        carrier_lane_bindings.reverse();
    }
    MeshMultipathSchedule {
        mode: MeshMultipathMode::FlowShard,
        route_binding_id: Some(route_binding_id),
        lanes,
        carrier_lane_bindings,
        active_lane_count: ACTIVE_BINDING_COUNT,
        standby_lane_count: 0,
        lane_admission_requested_active_lane_count: ACTIVE_BINDING_COUNT,
        lane_admission_admitted_active_lane_count: ACTIVE_BINDING_COUNT,
        lane_admission_rejected_active_lane_count: 0,
        lane_admission_capacity_status: "within_budget".to_string(),
        active_weight_sum_pct: 100,
        active_capacity_sum_pct: 90,
        local_traffic_reserve_pct: 10,
        transit_capacity_budget_pct: 90,
        demand_policy: "bulk".to_string(),
        demand_policy_source: "benchmark".to_string(),
        demand_requested_active_lane_count: ACTIVE_BINDING_COUNT,
        demand_planned_active_lane_count: ACTIVE_BINDING_COUNT,
        demand_admitted_lane_capacity_pct: 90,
        demand_unmet_lane_count: 0,
        demand_status: "satisfied".to_string(),
        demand_rebuild_recommended: false,
        fairness_policy: "weighted_round_robin_v1".to_string(),
        execution_status: "carrier_binding_ready".to_string(),
        transit_payload_policy: "sealed_opaque_only".to_string(),
        planner_rebuild_reason: "none".to_string(),
    }
}

fn ops_per_sec(iterations: usize, elapsed: Duration) -> f64 {
    let seconds = elapsed.as_secs_f64();
    if seconds <= 0.0 {
        return 0.0;
    }
    iterations as f64 / seconds
}

fn flow_hash(index: usize) -> u64 {
    let mut value = index as u64;
    value ^= value >> 33;
    value = value.wrapping_mul(0xff51_afd7_ed55_8ccd);
    value ^= value >> 33;
    value = value.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    value ^ (value >> 33)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_perf_options_parse_defaults() {
        assert_eq!(
            parse_metadata_perf_options(&[]).unwrap_or_else(|error| unreachable!("{error}")),
            MetadataPerfOptions::default()
        );
    }

    #[test]
    fn metadata_perf_options_parse_full() {
        let args = vec![
            "--iterations".to_string(),
            "1000".to_string(),
            "--min-fast-ops".to_string(),
            "100".to_string(),
            "--json".to_string(),
        ];
        let parsed =
            parse_metadata_perf_options(&args).unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(
            parsed,
            MetadataPerfOptions {
                iterations: 1000,
                min_fast_ops: Some(100),
                json_output: true,
            }
        );
    }

    #[test]
    fn metadata_perf_options_reject_zero_iterations() {
        let args = vec!["--iterations".to_string(), "0".to_string()];
        assert!(parse_metadata_perf_options(&args).is_err());
    }

    #[test]
    fn metadata_perf_json_is_redacted_and_metadata_only() {
        let result = execute_metadata_perf_smoke(MetadataPerfOptions {
            iterations: 100,
            min_fast_ops: None,
            json_output: true,
        });
        let json = render_metadata_perf_json(&result);
        assert!(json.contains("\"kind\":\"metadata_perf_smoke\""));
        assert!(json.contains("\"scope\":\"hot_metadata_only\""));
        assert!(json.contains(
            "\"hot_paths\":[\"multipath_flow_lane_selection\",\"path_planner_candidate_snapshot\",\"discovery_rebuild_fingerprint\",\"discovery_update_noop_dirty_set\",\"lane_document_plan_snapshot_access\",\"lane_document_render_parse\",\"peer_update_state_publish_generation\",\"live_binding_reload_index\",\"live_dps_plan_path_from_payload\",\"live_dps_plan_core_from_payload\",\"status_explain\"]"
        ));
        assert!(json.contains("\"path_planner_candidate_snapshot_ops_per_sec\":"));
        assert!(json.contains("\"discovery_rebuild_fingerprint_ops_per_sec\":"));
        assert!(json.contains("\"discovery_update_noop_ops_per_sec\":"));
        assert!(json.contains("\"discovery_update_noop_p95_ns\":"));
        assert!(json.contains("\"lane_document_plan_snapshot_borrowed_ops_per_sec\":"));
        assert!(json.contains("\"lane_document_plan_snapshot_owned_ops_per_sec\":"));
        assert!(json.contains("\"lane_document_render_parse_ops_per_sec\":"));
        assert!(json.contains("\"peer_update_state_publish_noop_ops_per_sec\":"));
        assert!(json.contains("\"peer_update_state_publish_changed_generation_ops_per_sec\":"));
        assert!(json.contains("\"live_binding_reload_index_ops_per_sec\":"));
        assert!(json.contains("\"live_dps_plan_path_from_payload_ops_per_sec\":"));
        assert!(json.contains("\"live_dps_plan_core_from_payload_ops_per_sec\":"));
        assert!(json.contains("\"live_dps_plan_core_from_payload_p95_ns\":"));
        assert!(json.contains("\"status_explain_ops_per_sec\":"));
        assert!(json.contains("\"status_explain_peer_count\":64"));
        assert!(json.contains("\"live_dps_plan_path_from_payload_peer_count\":64"));
        assert!(json.contains("\"path_planner_peer_count\":64"));
        assert!(json.contains("\"discovery_rebuild_peer_count\":64"));
        assert!(json.contains("\"network_state\":\"not_modified\""));
        assert!(!json.contains("node-"));
        assert!(!json.contains("peer-"));
        assert!(!json.contains("perf-node"));
        assert!(!json.contains("perf-rebuild-node"));
        assert!(!json.contains("perf-discovery-noop-node"));
        assert!(!json.contains("198.51."));
        assert!(!json.contains("node.invalid"));
        assert!(!json.contains("node-alt"));
        assert!(!json.contains(PEER_UPDATE_STATE_SHA256));
        assert!(!json.contains("77001"));
    }
}
