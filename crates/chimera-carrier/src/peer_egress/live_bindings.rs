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
        let initial =
            load_live_transit_lane_document(options.transit_lane_bindings_file.as_deref())?;
        validate_live_transit_lane_document_contract(options, &initial)?;
        let snapshot = Arc::new(Mutex::new(Ok(Arc::new(initial.clone()))));
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
                initial.registrations().to_vec(),
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
) -> Result<Vec<TransitLaneRegistration>, String> {
    match transit_lane_bindings_file {
        Some(path) => load_transit_lane_registrations(path),
        None => Ok(Vec::new()),
    }
}

pub fn load_live_transit_lane_document(
    transit_lane_bindings_file: Option<&str>,
) -> Result<TransitLaneDocument, String> {
    match transit_lane_bindings_file {
        Some(path) => load_transit_lane_document(path),
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
    let changed_document = TransitLaneDocument::new(changed_desired.clone(), None);
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
    initial: Vec<TransitLaneRegistration>,
) {
    let mut workers = BTreeMap::new();
    reconcile_live_transit_lane_workers(
        &mut workers,
        &initial,
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
        match load_transit_lane_document(&path) {
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
    let desired_by_binding = desired
        .iter()
        .map(|registration| (registration.binding(), registration))
        .collect::<BTreeMap<_, _>>();
    workers.retain(|binding, worker| {
        if desired_by_binding
            .get(binding)
            .is_some_and(|registration| worker.registration == **registration)
        {
            return true;
        }
        worker.cancel.store(true, Ordering::Relaxed);
        let _ = dispatcher.clear_binding(*binding);
        false
    });

    for (binding, registration) in desired_by_binding {
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
) -> Result<(), String> {
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
    wait_until_transit_lane_claimed(&dispatcher, ticket)?;
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
) -> Result<(), String> {
    while dispatcher.contains_ticket(ticket)? {
        thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        LiveTransitLaneWorker, TransitLaneDocument, apply_live_transit_lane_reload,
        clear_live_transit_lane_workers, live_binding_reload_index_perf_smoke,
        reconcile_live_transit_lane_workers, replace_live_transit_lane_snapshot,
    };
    use crate::peer_egress::lane_binding::TransitLaneRegistration;
    use crate::peer_egress::options::AeadSuite;
    use crate::peer_egress::protocol::SecurePeerStream;
    use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
    use crate::peer_egress::transit_dispatch::TransitNextHopDispatcher;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    fn binding(route: u64, lane: u16) -> TransitPathBinding {
        TransitPathBinding::new(
            TransitRouteId::new(route).unwrap_or_else(|error| unreachable!("{error}")),
            TransitLaneId::new(lane).unwrap_or_else(|error| unreachable!("{error}")),
        )
    }

    fn registration(
        route: u64,
        lane: u16,
        endpoint: &str,
    ) -> Result<TransitLaneRegistration, String> {
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
    fn worker_reconcile_restarts_binding_when_endpoint_changes() -> Result<(), String> {
        let dispatcher = Arc::new(TransitNextHopDispatcher::default());
        let initial = registration(7, 1, "198.51.100.7:443")?;
        let changed = registration(7, 1, "198.51.100.8:443")?;
        let old_cancel = Arc::new(AtomicBool::new(false));
        let mut workers = BTreeMap::from([(
            initial.binding(),
            LiveTransitLaneWorker {
                registration: initial.clone(),
                cancel: old_cancel.clone(),
            },
        )]);
        dispatcher.register(initial.binding(), test_peer_stream()?)?;
        let spawned = Arc::new(Mutex::new(Vec::new()));
        let spawned_log = spawned.clone();

        reconcile_live_transit_lane_workers(
            &mut workers,
            std::slice::from_ref(&changed),
            &dispatcher,
            move |registration, _cancel| {
                spawned_log
                    .lock()
                    .unwrap_or_else(|_| unreachable!("spawn log must lock"))
                    .push(registration.clone());
            },
        );

        assert!(old_cancel.load(Ordering::Relaxed));
        assert!(!dispatcher.contains_binding(initial.binding())?);
        assert_eq!(workers.len(), 1);
        assert_eq!(
            workers
                .get(&changed.binding())
                .ok_or_else(|| "updated worker missing".to_string())?
                .registration,
            changed
        );
        assert_eq!(
            spawned
                .lock()
                .unwrap_or_else(|_| unreachable!("spawn log must lock"))
                .as_slice(),
            &[changed]
        );
        Ok(())
    }

    #[test]
    fn worker_reconcile_keeps_same_binding_without_spawn_churn() -> Result<(), String> {
        let dispatcher = Arc::new(TransitNextHopDispatcher::default());
        let registration = registration(8, 2, "198.51.100.8:443")?;
        let cancel = Arc::new(AtomicBool::new(false));
        let mut workers = BTreeMap::from([(
            registration.binding(),
            LiveTransitLaneWorker {
                registration: registration.clone(),
                cancel: cancel.clone(),
            },
        )]);
        dispatcher.register(registration.binding(), test_peer_stream()?)?;
        let spawned = Arc::new(Mutex::new(Vec::new()));
        let spawned_log = spawned.clone();

        reconcile_live_transit_lane_workers(
            &mut workers,
            std::slice::from_ref(&registration),
            &dispatcher,
            move |registration, _cancel| {
                spawned_log
                    .lock()
                    .unwrap_or_else(|_| unreachable!("spawn log must lock"))
                    .push(registration.clone());
            },
        );

        assert!(!cancel.load(Ordering::Relaxed));
        assert!(dispatcher.contains_binding(registration.binding())?);
        assert_eq!(workers.len(), 1);
        assert_eq!(
            workers
                .get(&registration.binding())
                .ok_or_else(|| "worker missing".to_string())?
                .registration,
            registration
        );
        assert!(
            spawned
                .lock()
                .unwrap_or_else(|_| unreachable!("spawn log must lock"))
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn clear_workers_evicts_registered_bindings_and_sets_cancel() -> Result<(), String> {
        let dispatcher = Arc::new(TransitNextHopDispatcher::default());
        let registration = registration(11, 2, "198.51.100.11:443")?;
        let cancel = Arc::new(AtomicBool::new(false));
        let mut workers = BTreeMap::from([(
            registration.binding(),
            LiveTransitLaneWorker {
                registration: registration.clone(),
                cancel: cancel.clone(),
            },
        )]);
        dispatcher.register(registration.binding(), test_peer_stream()?)?;

        clear_live_transit_lane_workers(&mut workers, &dispatcher);

        assert!(cancel.load(Ordering::Relaxed));
        assert!(workers.is_empty());
        assert!(!dispatcher.contains_binding(registration.binding())?);
        Ok(())
    }

    #[test]
    fn reload_error_replaces_snapshot_with_fail_closed_error() -> Result<(), String> {
        let snapshot = Arc::new(Mutex::new(Ok(Arc::new(TransitLaneDocument::new(
            vec![registration(19, 3, "198.51.100.19:443")?],
            None,
        )))));

        replace_live_transit_lane_snapshot(
            &snapshot,
            Err("read sealed transit lane bindings failed: missing".to_string()),
        );

        let current = snapshot
            .lock()
            .map_err(|_| "snapshot lock poisoned".to_string())?
            .clone();
        let error = match current {
            Ok(_) => return Err("reload error must replace live snapshot".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("read sealed transit lane bindings failed"));
        Ok(())
    }

    #[test]
    fn reload_noop_keeps_snapshot_and_workers_when_document_is_unchanged() -> Result<(), String> {
        let dispatcher = Arc::new(TransitNextHopDispatcher::default());
        let registration = registration(21, 4, "198.51.100.21:443")?;
        let initial_document = TransitLaneDocument::new(vec![registration.clone()], None);
        let initial_snapshot = Arc::new(initial_document.clone());
        let snapshot = Arc::new(Mutex::new(Ok(initial_snapshot.clone())));
        let cancel = Arc::new(AtomicBool::new(false));
        let mut workers = BTreeMap::from([(
            registration.binding(),
            LiveTransitLaneWorker {
                registration: registration.clone(),
                cancel: cancel.clone(),
            },
        )]);
        dispatcher.register(registration.binding(), test_peer_stream()?)?;
        let spawned = Arc::new(Mutex::new(0usize));
        let spawned_log = spawned.clone();

        apply_live_transit_lane_reload(
            &snapshot,
            &mut workers,
            &dispatcher,
            Ok(initial_document),
            move |_registration, _cancel| {
                let Ok(mut guard) = spawned_log.lock() else {
                    unreachable!("spawn counter must lock");
                };
                *guard += 1;
            },
        );

        assert!(Arc::ptr_eq(
            snapshot
                .lock()
                .map_err(|_| "snapshot lock poisoned".to_string())?
                .as_ref()
                .map_err(|_| "snapshot must remain ok".to_string())?,
            &initial_snapshot
        ));
        assert_eq!(
            *spawned
                .lock()
                .map_err(|_| "spawn counter lock poisoned".to_string())?,
            0
        );
        assert!(!cancel.load(Ordering::Relaxed));
        assert_eq!(workers.len(), 1);
        assert_eq!(
            workers
                .get(&registration.binding())
                .ok_or_else(|| "worker missing".to_string())?
                .registration,
            registration
        );
        assert!(dispatcher.contains_binding(registration.binding())?);
        Ok(())
    }

    #[test]
    fn reload_changed_document_replaces_stale_workers_and_spawns_new_bindings() -> Result<(), String>
    {
        let dispatcher = Arc::new(TransitNextHopDispatcher::default());
        let unchanged = registration(31, 5, "198.51.100.31:443")?;
        let stale = registration(32, 6, "198.51.100.32:443")?;
        let changed = registration(32, 6, "198.51.100.132:443")?;
        let added = registration(33, 7, "198.51.100.33:443")?;
        let initial_document =
            TransitLaneDocument::new(vec![unchanged.clone(), stale.clone()], None);
        let snapshot = Arc::new(Mutex::new(Ok(Arc::new(initial_document.clone()))));
        let unchanged_cancel = Arc::new(AtomicBool::new(false));
        let stale_cancel = Arc::new(AtomicBool::new(false));
        let mut workers = BTreeMap::from([
            (
                unchanged.binding(),
                LiveTransitLaneWorker {
                    registration: unchanged.clone(),
                    cancel: unchanged_cancel.clone(),
                },
            ),
            (
                stale.binding(),
                LiveTransitLaneWorker {
                    registration: stale.clone(),
                    cancel: stale_cancel.clone(),
                },
            ),
        ]);
        dispatcher.register(unchanged.binding(), test_peer_stream()?)?;
        dispatcher.register(stale.binding(), test_peer_stream()?)?;
        let spawned = Arc::new(Mutex::new(Vec::new()));
        let spawned_log = spawned.clone();

        apply_live_transit_lane_reload(
            &snapshot,
            &mut workers,
            &dispatcher,
            Ok(TransitLaneDocument::new(
                vec![unchanged.clone(), changed.clone(), added.clone()],
                None,
            )),
            move |registration, _cancel| {
                spawned_log
                    .lock()
                    .unwrap_or_else(|_| unreachable!("spawn log must lock"))
                    .push(registration.clone());
            },
        );

        assert!(!unchanged_cancel.load(Ordering::Relaxed));
        assert!(stale_cancel.load(Ordering::Relaxed));
        assert_eq!(workers.len(), 3);
        assert_eq!(
            workers
                .get(&unchanged.binding())
                .ok_or_else(|| "unchanged worker missing".to_string())?
                .registration,
            unchanged
        );
        assert_eq!(
            workers
                .get(&changed.binding())
                .ok_or_else(|| "changed worker missing".to_string())?
                .registration,
            changed
        );
        assert_eq!(
            workers
                .get(&added.binding())
                .ok_or_else(|| "added worker missing".to_string())?
                .registration,
            added
        );
        assert!(dispatcher.contains_binding(unchanged.binding())?);
        assert!(!dispatcher.contains_binding(stale.binding())?);
        assert_eq!(
            spawned
                .lock()
                .map_err(|_| "spawn log lock poisoned".to_string())?
                .as_slice(),
            &[changed, added]
        );
        Ok(())
    }

    #[test]
    fn worker_reconcile_duplicate_desired_binding_keeps_last_registration() -> Result<(), String> {
        let dispatcher = Arc::new(TransitNextHopDispatcher::default());
        let old = registration(36, 3, "198.51.100.36:443")?;
        let stale_desired = old.clone();
        let last_desired = registration(36, 3, "198.51.100.136:443")?;
        let old_cancel = Arc::new(AtomicBool::new(false));
        let mut workers = BTreeMap::from([(
            old.binding(),
            LiveTransitLaneWorker {
                registration: old.clone(),
                cancel: old_cancel.clone(),
            },
        )]);
        dispatcher.register(old.binding(), test_peer_stream()?)?;
        let spawned = Arc::new(Mutex::new(Vec::new()));
        let spawned_log = spawned.clone();

        reconcile_live_transit_lane_workers(
            &mut workers,
            &[stale_desired, last_desired.clone()],
            &dispatcher,
            move |registration, _cancel| {
                spawned_log
                    .lock()
                    .unwrap_or_else(|_| unreachable!("spawn log must lock"))
                    .push(registration.clone());
            },
        );

        assert!(old_cancel.load(Ordering::Relaxed));
        assert!(!dispatcher.contains_binding(old.binding())?);
        assert_eq!(workers.len(), 1);
        assert_eq!(
            workers
                .get(&last_desired.binding())
                .ok_or_else(|| "last desired worker missing".to_string())?
                .registration,
            last_desired
        );
        assert_eq!(
            spawned
                .lock()
                .map_err(|_| "spawn log lock poisoned".to_string())?
                .as_slice(),
            &[last_desired]
        );
        Ok(())
    }

    #[test]
    fn reload_error_clears_workers_and_sets_fail_closed_snapshot() -> Result<(), String> {
        let dispatcher = Arc::new(TransitNextHopDispatcher::default());
        let first = registration(41, 8, "198.51.100.41:443")?;
        let second = registration(42, 9, "198.51.100.42:443")?;
        let snapshot = Arc::new(Mutex::new(Ok(Arc::new(TransitLaneDocument::new(
            vec![first.clone(), second.clone()],
            None,
        )))));
        let first_cancel = Arc::new(AtomicBool::new(false));
        let second_cancel = Arc::new(AtomicBool::new(false));
        let mut workers = BTreeMap::from([
            (
                first.binding(),
                LiveTransitLaneWorker {
                    registration: first.clone(),
                    cancel: first_cancel.clone(),
                },
            ),
            (
                second.binding(),
                LiveTransitLaneWorker {
                    registration: second.clone(),
                    cancel: second_cancel.clone(),
                },
            ),
        ]);
        dispatcher.register(first.binding(), test_peer_stream()?)?;
        dispatcher.register(second.binding(), test_peer_stream()?)?;
        let spawned = Arc::new(Mutex::new(0usize));
        let spawned_log = spawned.clone();

        apply_live_transit_lane_reload(
            &snapshot,
            &mut workers,
            &dispatcher,
            Err("read sealed transit lane bindings failed: missing".to_string()),
            move |_registration, _cancel| {
                let Ok(mut guard) = spawned_log.lock() else {
                    unreachable!("spawn counter must lock");
                };
                *guard += 1;
            },
        );

        let current = snapshot
            .lock()
            .map_err(|_| "snapshot lock poisoned".to_string())?
            .clone();
        let error = match current {
            Ok(_) => return Err("reload error must replace live snapshot".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("read sealed transit lane bindings failed"));
        assert!(workers.is_empty());
        assert!(first_cancel.load(Ordering::Relaxed));
        assert!(second_cancel.load(Ordering::Relaxed));
        assert_eq!(
            *spawned
                .lock()
                .map_err(|_| "spawn counter lock poisoned".to_string())?,
            0
        );
        assert!(!dispatcher.contains_binding(first.binding())?);
        assert!(!dispatcher.contains_binding(second.binding())?);
        Ok(())
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
}
