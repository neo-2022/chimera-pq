use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use self::contract::validate_live_transit_lane_document_contract;
use crate::peer_egress::handshake::establish_secure_peer_client;
use crate::peer_egress::lane_binding::{
    TransitLaneDocument, TransitLaneRegistration, load_transit_lane_document,
    load_transit_lane_registrations,
};
use crate::peer_egress::net::{connect_tcp, tune_tcp};
use crate::peer_egress::options::Options;
use crate::peer_egress::protocol::redacted_log_reason;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
use crate::peer_egress::transit_dispatch::{
    SharedTransitNextHopDispatcher, TransitNextHopDispatcher,
};
use std::io::Write;

const LIVE_TRANSIT_LANE_POLL_INTERVAL: Duration = Duration::from_millis(250);
const LIVE_TRANSIT_LANE_RETRY_INTERVAL: Duration = Duration::from_secs(1);
type LiveTransitLaneSnapshot = Result<Arc<TransitLaneDocument>, String>;
const LIVE_BINDING_RELOAD_INDEX_SAMPLE_COUNT: usize = 100;

mod contract;

#[cfg(test)]
#[path = "live_bindings_extra_tests.rs"]
mod extra_tests;

#[derive(Clone)]
pub struct LiveTransitLaneRegistry {
    snapshot: Arc<Mutex<LiveTransitLaneSnapshot>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveBindingReloadIndexPerfResult {
    pub iterations: usize,
    pub spawn_count: usize,
    pub ops_per_sec: f64,
    pub p95_ns: u128,
}

impl LiveTransitLaneRegistry {
    pub fn start(
        options: &Options,
        dispatcher: SharedTransitNextHopDispatcher,
    ) -> Result<Self, String> {
        let initial = Arc::new(load_live_transit_lane_document(
            options.transit_lane_bindings_file.as_deref(),
            options.discovery_configured(),
        )?);
        validate_live_transit_lane_document_contract(options, &initial)?;
        let snapshot = Arc::new(Mutex::new(Ok(Arc::clone(&initial))));
        let registry = Self {
            snapshot: snapshot.clone(),
        };
        let Some(path) = options.transit_lane_bindings_file.clone() else {
            return Ok(registry);
        };

        let worker_options = options.clone();
        thread::spawn(move || {
            watch_live_transit_lane_registrations(
                path,
                worker_options,
                dispatcher,
                snapshot,
                initial,
            );
        });
        Ok(registry)
    }

    pub fn snapshot(&self) -> Result<Arc<TransitLaneDocument>, String> {
        self.snapshot
            .lock()
            .map_err(|_| "sealed transit lane runtime snapshot lock poisoned".to_string())?
            .clone()
    }
}

#[derive(Clone)]
struct LiveTransitLaneWorker {
    registration: TransitLaneRegistration,
    cancel: Arc<AtomicBool>,
}

pub fn load_live_transit_lane_registrations(
    transit_lane_bindings_file: Option<&str>,
    discovery_configured: bool,
) -> Result<Vec<TransitLaneRegistration>, String> {
    match transit_lane_bindings_file {
        Some(path) => {
            if discovery_configured && !std::path::Path::new(path).exists() {
                return Ok(Vec::new());
            }
            load_transit_lane_registrations(path)
        }
        None => Ok(Vec::new()),
    }
}

pub fn load_live_transit_lane_document(
    transit_lane_bindings_file: Option<&str>,
    discovery_configured: bool,
) -> Result<TransitLaneDocument, String> {
    match transit_lane_bindings_file {
        Some(path) => {
            if discovery_configured && !std::path::Path::new(path).exists() {
                return Ok(TransitLaneDocument::new(Vec::new(), None));
            }
            load_transit_lane_document(path)
        }
        None => Ok(TransitLaneDocument::new(Vec::new(), None)),
    }
}

pub fn live_binding_reload_index_perf_smoke(
    iterations: usize,
) -> Result<LiveBindingReloadIndexPerfResult, String> {
    let iterations = iterations.max(1);
    let sample_count = iterations.clamp(1, LIVE_BINDING_RELOAD_INDEX_SAMPLE_COUNT);
    let batch_size = iterations.div_ceil(sample_count);
    let measured_iterations = sample_count.saturating_mul(batch_size);
    let dispatcher = Arc::new(TransitNextHopDispatcher::default());
    let initial_desired = live_binding_reload_index_fixture("198.51.100.61")?;
    let changed_desired = live_binding_reload_index_fixture("198.51.100.162")?;
    let initial_document = TransitLaneDocument::new(initial_desired.clone(), None);
    let changed_document = TransitLaneDocument::new(changed_desired, None);
    let snapshot = Arc::new(Mutex::new(Ok(Arc::new(initial_document.clone()))));
    let mut workers = initial_desired
        .iter()
        .cloned()
        .map(|registration| {
            let cancel = Arc::new(AtomicBool::new(false));
            (
                registration.binding(),
                LiveTransitLaneWorker {
                    registration,
                    cancel,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut spawn_count = 0usize;
    let mut samples = Vec::with_capacity(sample_count);
    let total_start = Instant::now();

    for sample in 0..sample_count {
        let batch_start = Instant::now();
        for offset in 0..batch_size {
            let next = if (sample + offset) % 2 == 0 {
                Ok(initial_document.clone())
            } else {
                Ok(changed_document.clone())
            };
            apply_live_transit_lane_reload(
                &snapshot,
                &mut workers,
                &dispatcher,
                next,
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

    Ok(LiveBindingReloadIndexPerfResult {
        iterations: measured_iterations,
        spawn_count,
        ops_per_sec,
        p95_ns,
    })
}

fn watch_live_transit_lane_registrations(
    path: String,
    options: Options,
    dispatcher: SharedTransitNextHopDispatcher,
    snapshot: Arc<Mutex<LiveTransitLaneSnapshot>>,
    initial: Arc<TransitLaneDocument>,
) {
    let mut workers = BTreeMap::new();
    reconcile_live_transit_lane_workers(
        &mut workers,
        initial.registrations(),
        &dispatcher,
        |registration, cancel| {
            spawn_live_transit_lane_worker(
                options.clone(),
                registration.clone(),
                dispatcher.clone(),
                cancel,
            )
        },
    );

    loop {
        thread::sleep(LIVE_TRANSIT_LANE_POLL_INTERVAL);
        match load_live_transit_lane_document(Some(&path), options.discovery_configured()) {
            Ok(document) => {
                match validate_live_transit_lane_document_contract(&options, &document) {
                    Ok(()) => apply_live_transit_lane_reload(
                        &snapshot,
                        &mut workers,
                        &dispatcher,
                        Ok(document),
                        |registration, cancel| {
                            spawn_live_transit_lane_worker(
                                options.clone(),
                                registration.clone(),
                                dispatcher.clone(),
                                cancel,
                            )
                        },
                    ),
                    Err(error) => apply_live_transit_lane_reload(
                        &snapshot,
                        &mut workers,
                        &dispatcher,
                        Err(error),
                        |registration, cancel| {
                            spawn_live_transit_lane_worker(
                                options.clone(),
                                registration.clone(),
                                dispatcher.clone(),
                                cancel,
                            )
                        },
                    ),
                }
            }
            Err(error) => {
                apply_live_transit_lane_reload(
                    &snapshot,
                    &mut workers,
                    &dispatcher,
                    Err(error),
                    |registration, cancel| {
                        spawn_live_transit_lane_worker(
                            options.clone(),
                            registration.clone(),
                            dispatcher.clone(),
                            cancel,
                        )
                    },
                );
            }
        }
    }
}

fn replace_live_transit_lane_snapshot(
    snapshot: &Arc<Mutex<LiveTransitLaneSnapshot>>,
    next: Result<Arc<TransitLaneDocument>, String>,
) {
    let Ok(mut guard) = snapshot.lock() else {
        return;
    };
    *guard = match next {
        Ok(document) => Ok(document),
        Err(error) => Err(error),
    };
}

fn live_transit_lane_snapshot_matches_document(
    snapshot: &Arc<Mutex<LiveTransitLaneSnapshot>>,
    document: &TransitLaneDocument,
) -> bool {
    let Ok(guard) = snapshot.lock() else {
        return false;
    };
    matches!(&*guard, Ok(current) if current.as_ref() == document)
}

fn live_transit_lane_snapshot_matches_error(
    snapshot: &Arc<Mutex<LiveTransitLaneSnapshot>>,
    error: &str,
) -> bool {
    let Ok(guard) = snapshot.lock() else {
        return false;
    };
    matches!(&*guard, Err(current) if current == error)
}

fn apply_live_transit_lane_reload<F>(
    snapshot: &Arc<Mutex<LiveTransitLaneSnapshot>>,
    workers: &mut BTreeMap<TransitPathBinding, LiveTransitLaneWorker>,
    dispatcher: &SharedTransitNextHopDispatcher,
    next: Result<TransitLaneDocument, String>,
    spawn_worker: F,
) where
    F: FnMut(&TransitLaneRegistration, Arc<AtomicBool>),
{
    match next {
        Ok(document) => {
            if live_transit_lane_snapshot_matches_document(snapshot, &document) {
                return;
            }
            let document = Arc::new(document);
            replace_live_transit_lane_snapshot(snapshot, Ok(Arc::clone(&document)));
            reconcile_live_transit_lane_workers(
                workers,
                document.registrations(),
                dispatcher,
                spawn_worker,
            );
        }
        Err(error) => {
            if live_transit_lane_snapshot_matches_error(snapshot, &error) {
                return;
            }
            replace_live_transit_lane_snapshot(snapshot, Err(error));
            clear_live_transit_lane_workers(workers, dispatcher);
        }
    }
}

fn reconcile_live_transit_lane_workers<F>(
    workers: &mut BTreeMap<TransitPathBinding, LiveTransitLaneWorker>,
    desired: &[TransitLaneRegistration],
    dispatcher: &SharedTransitNextHopDispatcher,
    mut spawn_worker: F,
) where
    F: FnMut(&TransitLaneRegistration, Arc<AtomicBool>),
{
    if desired_registrations_are_sorted_unique(desired) {
        reconcile_sorted_unique_live_transit_lane_workers(
            workers,
            desired,
            dispatcher,
            spawn_worker,
        );
        return;
    }

    workers.retain(|binding, worker| {
        if last_desired_registration_for_binding(desired, *binding)
            .is_some_and(|registration| worker.registration == *registration)
        {
            return true;
        }
        worker.cancel.store(true, Ordering::Relaxed);
        let _ = dispatcher.clear_binding(*binding);
        false
    });

    for (index, registration) in desired.iter().enumerate() {
        let binding = registration.binding();
        if has_later_desired_registration_for_binding(desired, index, binding) {
            continue;
        }
        if workers
            .get(&binding)
            .is_some_and(|worker| worker.registration == *registration)
        {
            continue;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let registration = registration.clone();
        spawn_worker(&registration, cancel.clone());
        workers.insert(
            binding,
            LiveTransitLaneWorker {
                registration,
                cancel,
            },
        );
    }
}

fn reconcile_sorted_unique_live_transit_lane_workers<F>(
    workers: &mut BTreeMap<TransitPathBinding, LiveTransitLaneWorker>,
    desired: &[TransitLaneRegistration],
    dispatcher: &SharedTransitNextHopDispatcher,
    mut spawn_worker: F,
) where
    F: FnMut(&TransitLaneRegistration, Arc<AtomicBool>),
{
    workers.retain(|binding, worker| {
        if sorted_desired_registration_for_binding(desired, *binding)
            .is_some_and(|registration| worker.registration == *registration)
        {
            return true;
        }
        worker.cancel.store(true, Ordering::Relaxed);
        let _ = dispatcher.clear_binding(*binding);
        false
    });

    for registration in desired {
        let binding = registration.binding();
        if workers
            .get(&binding)
            .is_some_and(|worker| worker.registration == *registration)
        {
            continue;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let registration = registration.clone();
        spawn_worker(&registration, cancel.clone());
        workers.insert(
            binding,
            LiveTransitLaneWorker {
                registration,
                cancel,
            },
        );
    }
}

fn desired_registrations_are_sorted_unique(desired: &[TransitLaneRegistration]) -> bool {
    desired
        .windows(2)
        .all(|pair| pair[0].binding() < pair[1].binding())
}

fn sorted_desired_registration_for_binding(
    desired: &[TransitLaneRegistration],
    binding: TransitPathBinding,
) -> Option<&TransitLaneRegistration> {
    desired
        .binary_search_by(|registration| registration.binding().cmp(&binding))
        .ok()
        .and_then(|index| desired.get(index))
}

fn last_desired_registration_for_binding(
    desired: &[TransitLaneRegistration],
    binding: TransitPathBinding,
) -> Option<&TransitLaneRegistration> {
    desired
        .iter()
        .rev()
        .find(|registration| registration.binding() == binding)
}

fn has_later_desired_registration_for_binding(
    desired: &[TransitLaneRegistration],
    index: usize,
    binding: TransitPathBinding,
) -> bool {
    desired.get(index.saturating_add(1)..).is_some_and(|tail| {
        tail.iter()
            .any(|registration| registration.binding() == binding)
    })
}

fn clear_live_transit_lane_workers(
    workers: &mut BTreeMap<TransitPathBinding, LiveTransitLaneWorker>,
    dispatcher: &SharedTransitNextHopDispatcher,
) {
    for (binding, worker) in std::mem::take(workers) {
        worker.cancel.store(true, Ordering::Relaxed);
        let _ = dispatcher.clear_binding(binding);
    }
}

fn spawn_live_transit_lane_worker(
    options: Options,
    registration: TransitLaneRegistration,
    dispatcher: SharedTransitNextHopDispatcher,
    cancel: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = dispatcher.clear_binding(registration.binding());
                return;
            }
            if let Err(error) = outbound_transit_lane_registration_worker(
                &options,
                &registration,
                dispatcher.clone(),
                &cancel,
            ) {
                if cancel.load(Ordering::Relaxed) {
                    let _ = dispatcher.clear_binding(registration.binding());
                    return;
                }
                eprintln!(
                    "event=weave_transit_lane_worker_error reason_class={}",
                    redacted_log_reason(&error)
                );
                thread::sleep(LIVE_TRANSIT_LANE_RETRY_INTERVAL);
            }
        }
    });
}

fn outbound_transit_lane_registration_worker(
    options: &Options,
    registration: &TransitLaneRegistration,
    dispatcher: SharedTransitNextHopDispatcher,
    cancel: &AtomicBool,
) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        return Ok(());
    }
    let mut peer = connect_tcp(registration.endpoint(), options.connect_timeout_ms)
        .map_err(|error| format!("connect sealed transit lane failed: {error}"))?;
    tune_tcp(&peer)?;
    eprintln!("event=outbound_transit_lane_connected endpoint=<redacted>");
    peer.write_all(b"CHIMERA-PEER-EGRESS/1\n")
        .map_err(|error| format!("write handshake failed: {error}"))?;
    peer.write_all(options.token.as_bytes())
        .and_then(|_| peer.write_all(b"\n"))
        .map_err(|error| format!("write token failed: {error}"))?;
    let peer = establish_secure_peer_client(peer, &options.token, options.aead)?;
    let ticket = dispatcher.register(registration.binding(), peer)?;
    eprintln!("event=outbound_transit_lane_registered binding=<opaque>");
    wait_until_transit_lane_claimed(&dispatcher, ticket, cancel)?;
    eprintln!("event=outbound_transit_lane_claimed binding=<opaque>");
    Ok(())
}

fn live_binding_reload_index_fixture(
    endpoint_prefix: &str,
) -> Result<Vec<TransitLaneRegistration>, String> {
    let mut registrations = Vec::with_capacity(8);
    for lane in 1..=8u16 {
        registrations.push(TransitLaneRegistration::new(
            TransitPathBinding::new(
                TransitRouteId::new(61).unwrap_or_else(|error| unreachable!("{error}")),
                TransitLaneId::new(lane).unwrap_or_else(|error| unreachable!("{error}")),
            ),
            format!("{}.{lane}:443", endpoint_prefix),
        )?);
    }
    Ok(registrations)
}

fn wait_until_transit_lane_claimed(
    dispatcher: &SharedTransitNextHopDispatcher,
    ticket: crate::peer_egress::transit_dispatch::TransitNextHopTicket,
    cancel: &AtomicBool,
) -> Result<(), String> {
    while dispatcher.contains_ticket(ticket)? {
        if cancel.load(Ordering::Relaxed) {
            let _ = dispatcher.clear_ticket(ticket)?;
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

#[cfg(test)]
#[path = "live_bindings_tests/mod.rs"]
mod tests;
