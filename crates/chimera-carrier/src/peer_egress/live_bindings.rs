use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::peer_egress::handshake::establish_secure_peer_client;
use crate::peer_egress::lane_binding::{TransitLaneRegistration, load_transit_lane_registrations};
use crate::peer_egress::net::{connect_tcp, tune_tcp};
use crate::peer_egress::options::Options;
use crate::peer_egress::protocol::redacted_log_reason;
use crate::peer_egress::transit_binding::TransitPathBinding;
use crate::peer_egress::transit_dispatch::SharedTransitNextHopDispatcher;
use std::io::Write;

const LIVE_TRANSIT_LANE_POLL_INTERVAL: Duration = Duration::from_millis(250);
const LIVE_TRANSIT_LANE_RETRY_INTERVAL: Duration = Duration::from_secs(1);
type LiveTransitLaneSnapshot = Result<Arc<Vec<TransitLaneRegistration>>, String>;

#[derive(Clone)]
pub struct LiveTransitLaneRegistry {
    snapshot: Arc<Mutex<LiveTransitLaneSnapshot>>,
}

impl LiveTransitLaneRegistry {
    pub fn start(
        options: &Options,
        dispatcher: SharedTransitNextHopDispatcher,
    ) -> Result<Self, String> {
        let initial =
            load_live_transit_lane_registrations(options.transit_lane_bindings_file.as_deref())?;
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
                initial,
            );
        });
        Ok(registry)
    }

    pub fn snapshot(&self) -> Result<Arc<Vec<TransitLaneRegistration>>, String> {
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
        match load_transit_lane_registrations(&path) {
            Ok(registrations) => {
                replace_live_transit_lane_snapshot(&snapshot, Ok(registrations.clone()));
                reconcile_live_transit_lane_workers(
                    &mut workers,
                    &registrations,
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
            }
            Err(error) => {
                replace_live_transit_lane_snapshot(&snapshot, Err(error));
                clear_live_transit_lane_workers(&mut workers, &dispatcher);
            }
        }
    }
}

fn replace_live_transit_lane_snapshot(
    snapshot: &Arc<Mutex<LiveTransitLaneSnapshot>>,
    next: Result<Vec<TransitLaneRegistration>, String>,
) {
    let Ok(mut guard) = snapshot.lock() else {
        return;
    };
    *guard = match next {
        Ok(registrations) => Ok(Arc::new(registrations)),
        Err(error) => Err(error),
    };
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
        .map(|registration| (registration.binding(), registration.clone()))
        .collect::<BTreeMap<_, _>>();
    let stale_bindings = workers
        .iter()
        .filter_map(|(binding, worker)| {
            (desired_by_binding.get(binding) != Some(&worker.registration))
                .then_some((*binding, worker.cancel.clone()))
        })
        .collect::<Vec<_>>();

    for (binding, cancel) in stale_bindings {
        cancel.store(true, Ordering::Relaxed);
        let _ = dispatcher.pop_for(binding);
        workers.remove(&binding);
    }

    for (binding, registration) in desired_by_binding {
        if workers
            .get(&binding)
            .is_some_and(|worker| worker.registration == registration)
        {
            continue;
        }
        let cancel = Arc::new(AtomicBool::new(false));
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
        let _ = dispatcher.pop_for(binding);
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
                let _ = dispatcher.pop_for(registration.binding());
                return;
            }
            if let Err(error) = outbound_transit_lane_registration_worker(
                &options,
                &registration,
                dispatcher.clone(),
            ) {
                if cancel.load(Ordering::Relaxed) {
                    let _ = dispatcher.pop_for(registration.binding());
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
    dispatcher.register(registration.binding(), peer)?;
    eprintln!("event=outbound_transit_lane_registered binding=<opaque>");
    wait_until_transit_lane_claimed(&dispatcher, registration.binding())?;
    eprintln!("event=outbound_transit_lane_claimed binding=<opaque>");
    Ok(())
}

fn wait_until_transit_lane_claimed(
    dispatcher: &SharedTransitNextHopDispatcher,
    binding: TransitPathBinding,
) -> Result<(), String> {
    while dispatcher.contains_binding(binding)? {
        thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        LiveTransitLaneWorker, clear_live_transit_lane_workers,
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
        let snapshot = Arc::new(Mutex::new(Ok(Arc::new(vec![registration(
            19,
            3,
            "198.51.100.19:443",
        )?]))));

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
}
