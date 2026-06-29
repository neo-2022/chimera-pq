use super::{
    LiveTransitLaneWorker, TransitLaneDocument, apply_live_transit_lane_reload,
    live_binding_reload_index_perf_smoke, reconcile_live_transit_lane_workers,
};
use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::options::AeadSuite;
use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
use crate::peer_egress::transit_dispatch::TransitNextHopDispatcher;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

mod reload;
mod workers;

fn binding(route: u64, lane: u16) -> TransitPathBinding {
    TransitPathBinding::new(
        TransitRouteId::new(route).unwrap_or_else(|error| unreachable!("{error}")),
        TransitLaneId::new(lane).unwrap_or_else(|error| unreachable!("{error}")),
    )
}

fn registration(route: u64, lane: u16, endpoint: &str) -> Result<TransitLaneRegistration, String> {
    TransitLaneRegistration::new(binding(route, lane), endpoint.to_string())
}

fn test_peer_stream() -> Result<SecurePeerStream, String> {
    let transcript = chimera_crypto::TranscriptHash::from_messages(&[b"live-bindings-test"]);
    let secrets = chimera_crypto::derive_traffic_secrets(
        chimera_crypto::SuiteId(AeadSuite::Chacha20Poly1305.suite_id()),
        &transcript,
        &[29_u8; 32],
    )
    .map_err(|error| format!("test secrets derive failed: {error}"))?;
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("bind test listener failed: {error}"))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("read listener addr failed: {error}"))?;
    let client = std::net::TcpStream::connect(addr)
        .map_err(|error| format!("connect test client failed: {error}"))?;
    let (server, _) = listener
        .accept()
        .map_err(|error| format!("accept test peer failed: {error}"))?;
    drop(server);
    Ok(SecurePeerStream {
        stream: client,
        send_secret: secrets.initiator_to_responder().clone(),
        recv_secret: secrets.responder_to_initiator().clone(),
        send_packet: 0,
        recv_packet: 0,
        aead: AeadSuite::Chacha20Poly1305,
    })
}

#[test]
#[ignore]
fn reload_noop_fast_path_perf_smoke() -> Result<(), String> {
    const DEFAULT_ITERATIONS: usize = 100_000;
    const SAMPLE_COUNT: usize = 100;

    let iterations = std::env::var("CHIMERA_LIVE_BINDING_RELOAD_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_ITERATIONS);
    let sample_count = iterations.clamp(1, SAMPLE_COUNT);
    let batch_size = iterations.div_ceil(sample_count);
    let measured_iterations = sample_count.saturating_mul(batch_size);
    let dispatcher = Arc::new(TransitNextHopDispatcher::default());
    let registration = registration(51, 10, "198.51.100.51:443")?;
    let document = TransitLaneDocument::new(vec![registration.clone()], None);
    let snapshot = Arc::new(Mutex::new(Ok(Arc::new(document.clone()))));
    let cancel = Arc::new(AtomicBool::new(false));
    let mut workers = BTreeMap::from([(
        registration.binding(),
        LiveTransitLaneWorker {
            registration: registration.clone(),
            cancel: cancel.clone(),
        },
    )]);
    let mut spawn_count = 0usize;
    let mut samples = Vec::with_capacity(sample_count);
    let total_start = std::time::Instant::now();

    for _sample in 0..sample_count {
        let batch_start = std::time::Instant::now();
        for _offset in 0..batch_size {
            apply_live_transit_lane_reload(
                &snapshot,
                &mut workers,
                &dispatcher,
                Ok(document.clone()),
                |_registration, _cancel| {
                    spawn_count = spawn_count.saturating_add(1);
                },
            );
        }
        samples.push(batch_start.elapsed().as_nanos() / batch_size as u128);
    }

    let total_elapsed = total_start.elapsed();
    samples.sort_unstable();
    let p95_index = ((samples.len().saturating_sub(1)) * 95) / 100;
    let p95_ns = samples.get(p95_index).copied().unwrap_or(0);
    let ops_per_sec = if total_elapsed.as_secs_f64() <= 0.0 {
        0.0
    } else {
        measured_iterations as f64 / total_elapsed.as_secs_f64()
    };

    assert_eq!(spawn_count, 0);
    assert_eq!(workers.len(), 1);
    assert!(!cancel.load(Ordering::Relaxed));
    println!(
        "{{\"status\":\"ok\",\"kind\":\"live_binding_reload_perf_smoke\",\"iterations\":{},\"spawn_count\":{},\"ops_per_sec\":{:.0},\"p95_ns\":{},\"network_state\":\"not_modified\"}}",
        measured_iterations, spawn_count, ops_per_sec, p95_ns
    );
    Ok(())
}

#[test]
#[ignore]
fn reload_changed_document_reconcile_perf_smoke() -> Result<(), String> {
    let iterations = std::env::var("CHIMERA_LIVE_BINDING_RELOAD_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(100_000);
    let result = live_binding_reload_index_perf_smoke(iterations)?;
    println!(
        "{{\"status\":\"ok\",\"kind\":\"live_binding_reload_index_perf_smoke\",\"iterations\":{},\"spawn_count\":{},\"ops_per_sec\":{:.0},\"p95_ns\":{},\"network_state\":\"not_modified\"}}",
        result.iterations, result.spawn_count, result.ops_per_sec, result.p95_ns
    );
    Ok(())
}
