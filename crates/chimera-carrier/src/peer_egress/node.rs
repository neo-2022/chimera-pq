use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Duration;

use crate::peer_egress::aggregate_ingress::{
    AggregateTransitIngressLimits, new_shared_aggregate_transit_ingress_registry,
};
use crate::peer_egress::handshake::{authenticate_peer, establish_secure_peer_server};
use crate::peer_egress::live_bindings::LiveTransitLaneRegistry;
use crate::peer_egress::mesh_lane_driver::{
    MeshLaneDriverOptions, run_mesh_lane_driver, run_mesh_lane_driver_once,
};
use crate::peer_egress::modes::{
    handle_local_client_with_lane_document_and_first_byte,
    handle_local_client_with_peer_pool_and_first_byte,
    outbound_peer_worker_with_next_hop_lane_document_and_aggregate_ingress, serve_peer_pool_worker,
};
use crate::peer_egress::net::{bind_reuse_listener, tune_tcp};
use crate::peer_egress::options::{LOCAL_MAGIC, Options, write_resolved_state_file};
use crate::peer_egress::pool::new_shared_pool;
use crate::peer_egress::protocol::{redacted_error_fields, redacted_log_reason};
use crate::peer_egress::route_announcement_registry::{
    SharedRouteAnnouncementRegistry, local_announcements_from_options,
    new_shared_route_announcement_registry, sign_local_announcements,
};
use crate::peer_egress::startup_contract::validate_node_startup_contract;
use crate::peer_egress::transit::{BoundPeerTransitPolicy, PeerTransitPolicy};
use crate::peer_egress::transit_binding::BOUND_TRANSIT_MAGIC;
use crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher;
use crate::peer_egress::transit_local::{
    relay_local_bound_sealed_transit_to_next_hop_with_limits,
    relay_local_sealed_transit_to_next_hop_with_limits,
    relay_local_sealed_transit_with_lane_document_and_first_byte_with_limits,
};
use crate::peer_egress::wire::write_announce_message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalIngressBranch {
    SealedTransit,
    BoundSealedTransit,
    NativeConnect,
}

fn classify_local_ingress(first_byte: u8) -> LocalIngressBranch {
    if first_byte == chimera_session::FRAME_VERSION {
        LocalIngressBranch::SealedTransit
    } else if first_byte == BOUND_TRANSIT_MAGIC {
        LocalIngressBranch::BoundSealedTransit
    } else {
        LocalIngressBranch::NativeConnect
    }
}

pub fn run_node(mut options: Options) -> Result<(), String> {
    let mut dynamic_lanes_enabled = false;
    let mut lane_driver_cancel: Option<Arc<AtomicBool>> = None;
    let route_announcement_registry: SharedRouteAnnouncementRegistry =
        new_shared_route_announcement_registry();
    let mut local_announcements_to_share = local_announcements_from_options(&options);
    let signing_key = options.mesh_announcement_signing_key_bytes()?;
    sign_local_announcements(&mut local_announcements_to_share, signing_key.as_deref())?;
    if options.discovery_configured() {
        let driver_options = match build_mesh_lane_driver_options(
            &options,
            Some(route_announcement_registry.clone()),
        ) {
            Ok(value) => value,
            Err(error) => {
                eprintln!(
                    "event=mesh_lane_driver_config_error reason_class={}",
                    redacted_log_reason(&error)
                );
                Err(error)?
            }
        };
        if let Some(parent) = std::path::Path::new(&driver_options.lane_document_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let initial_driver_result = run_mesh_lane_driver_once(&driver_options);
        if let Err(ref error) = initial_driver_result {
            eprintln!(
                "event=mesh_lane_driver_initial_failed reason_class={}",
                redacted_log_reason(error)
            );
        }
        // Always enable dynamic lane reloads when discovery is configured. A
        // transient failure during the initial fetch should not leave the node
        // pinned to a stale lane document; the background driver retries on
        // its poll interval and will refresh the document once discovery is
        // reachable.
        options.allow_bound_transit = true;
        options.transit_lane_bindings_file = Some(driver_options.lane_document_path.clone());
        dynamic_lanes_enabled = true;
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_worker = cancel.clone();
        let driver_thread_options = driver_options;
        thread::spawn(move || {
            run_mesh_lane_driver(driver_thread_options, cancel_worker);
        });
        lane_driver_cancel = Some(cancel);
    }
    let startup_contract = validate_node_startup_contract(&options)?;
    let peer_listen = startup_contract.peer_listen.clone();
    let local_listen = startup_contract.local_listen.clone();
    let peer_listener = bind_reuse_listener(&peer_listen)
        .map_err(|error| format!("bind WEAVE peer ingress failed: {error}"))?;
    let local_listener = bind_reuse_listener(&local_listen)
        .map_err(|error| format!("bind WEAVE local ingress failed: {error}"))?;
    let resolved_peer_listen = peer_listener
        .local_addr()
        .map_err(|error| format!("resolve WEAVE peer ingress addr failed: {error}"))?
        .to_string();
    let resolved_local_listen = local_listener
        .local_addr()
        .map_err(|error| format!("resolve WEAVE local ingress addr failed: {error}"))?
        .to_string();
    if let Some(state_file) = &options.state_file
        && let Err(error) = write_resolved_state_file(
            state_file,
            &options.mode,
            &resolved_local_listen,
            &resolved_peer_listen,
        )
    {
        eprintln!(
            "event=weave_node_state_write_failed reason_class={}",
            redacted_log_reason(&error)
        );
    }

    let token = options.token.clone();
    let aead = options.aead;
    let local_announcements_for_ingress = local_announcements_to_share.clone();
    let _lane_driver_cancel = lane_driver_cancel;
    let peer_pool = new_shared_pool();
    let transit_dispatcher = new_shared_transit_dispatcher();
    let aggregate_ingress =
        new_shared_aggregate_transit_ingress_registry(AggregateTransitIngressLimits::default())?;
    let transit_limits = options.transit_relay_limits();
    let live_transit_lane_registry =
        LiveTransitLaneRegistry::start(&options, transit_dispatcher.clone())?;
    let peer_ingress_pool = peer_pool.clone();
    thread::spawn(move || {
        for incoming in peer_listener.incoming() {
            let Ok(mut stream) = incoming else {
                continue;
            };
            if let Err(error) = tune_tcp(&stream) {
                eprintln!(
                    "event=weave_peer_socket_tune_failed reason_class={}",
                    redacted_log_reason(&error)
                );
            }
            match authenticate_peer(&mut stream, &token)
                .and_then(|_| establish_secure_peer_server(stream, &token, aead))
            {
                Ok(mut peer) => {
                    eprintln!("event=weave_peer_ingress_authenticated");
                    if !local_announcements_for_ingress.is_empty()
                        && let Err(error) =
                            write_announce_message(&mut peer, &local_announcements_for_ingress)
                    {
                        eprintln!(
                            "event=weave_peer_ingress_announce_failed reason_class={}",
                            redacted_log_reason(&error)
                        );
                    }
                    if let Err(error) = peer_ingress_pool.push(peer) {
                        eprintln!(
                            "event=weave_peer_pool_push_failed reason_class={}",
                            redacted_log_reason(&error)
                        );
                    }
                }
                Err(error) => {
                    eprintln!(
                        "event=weave_peer_ingress_auth_failed reason_class={}",
                        redacted_log_reason(&error)
                    );
                }
            }
        }
    });

    for _ in 0..options.pool {
        let pool_worker_options = options.clone();
        let pool_worker_pool = peer_pool.clone();
        let pool_worker_dispatcher = transit_dispatcher.clone();
        let pool_worker_aggregate = aggregate_ingress.clone();
        let pool_worker_lane_registry = live_transit_lane_registry.clone();
        let pool_worker_registry = route_announcement_registry.clone();
        thread::spawn(move || {
            loop {
                let peer = match pool_worker_pool.pop_wait() {
                    Ok(peer) => peer,
                    Err(error) => {
                        eprintln!(
                            "event=peer_pool_pop_failed reason_class={}",
                            redacted_log_reason(&error)
                        );
                        thread::sleep(Duration::from_millis(500));
                        continue;
                    }
                };
                let lane_document = match pool_worker_lane_registry.snapshot() {
                    Ok(doc) => Some(doc),
                    Err(error) => {
                        eprintln!(
                            "event=peer_pool_lane_snapshot_failed reason_class={}",
                            redacted_log_reason(&error)
                        );
                        None
                    }
                };
                if let Err(error) = serve_peer_pool_worker(
                    &pool_worker_options,
                    None,
                    Some(pool_worker_dispatcher.clone()),
                    lane_document.as_deref(),
                    Some(pool_worker_aggregate.clone()),
                    Some(pool_worker_registry.clone()),
                    peer,
                ) {
                    eprintln!(
                        "event=peer_pool_worker_error {}",
                        redacted_error_fields(&error)
                    );
                }
            }
        });
    }

    if startup_contract.outbound_bootstrap_configured && !dynamic_lanes_enabled {
        for _ in 0..options.pool {
            let worker = options.clone();
            let outbound_pool = peer_pool.clone();
            let outbound_dispatcher = transit_dispatcher.clone();
            let outbound_lane_registry = live_transit_lane_registry.clone();
            let outbound_aggregate_ingress = aggregate_ingress.clone();
            thread::spawn(move || {
                loop {
                    let result = match outbound_lane_registry.snapshot() {
                        Ok(document) if document.is_empty() => {
                            outbound_peer_worker_with_next_hop_lane_document_and_aggregate_ingress(
                                &worker,
                                Some(outbound_pool.clone()),
                                Some(outbound_dispatcher.clone()),
                                None,
                                Some(outbound_aggregate_ingress.clone()),
                                None,
                            )
                        }
                        Ok(document) => {
                            outbound_peer_worker_with_next_hop_lane_document_and_aggregate_ingress(
                                &worker,
                                Some(outbound_pool.clone()),
                                Some(outbound_dispatcher.clone()),
                                Some(document.as_ref()),
                                Some(outbound_aggregate_ingress.clone()),
                                None,
                            )
                        }
                        Err(error) => Err(error),
                    };
                    if let Err(error) = result {
                        eprintln!(
                            "event=weave_outbound_peer_worker_error reason_class={}",
                            redacted_log_reason(&error)
                        );
                        thread::sleep(Duration::from_millis(500));
                    }
                }
            });
        }
    }

    println!(
        "chimera_peer_egress=node_ready local={} peer={} resolved_local={} resolved_peer={} outbound_bootstrap_configured={} pool_transit_allowed={} bound_transit_allowed={} capabilities={}",
        startup_contract.local_listen,
        startup_contract.peer_listen,
        resolved_local_listen,
        resolved_peer_listen,
        startup_contract.outbound_bootstrap_configured,
        startup_contract.pool_transit_allowed,
        startup_contract.bound_transit_allowed,
        startup_contract.capability_names().join(",")
    );
    for incoming in local_listener.incoming() {
        let Ok(local) = incoming else {
            continue;
        };
        eprintln!("event=weave_local_ingress_accepted");
        let mut local = local;
        let mut first = [0_u8; 1];
        match local.read_exact(&mut first) {
            Ok(())
                if matches!(
                    classify_local_ingress(first[0]),
                    LocalIngressBranch::SealedTransit
                ) =>
            {
                eprintln!("event=weave_local_ingress_transit_branch");
                let peer_pool = peer_pool.clone();
                let live_transit_lane_registry = live_transit_lane_registry.clone();
                let transit_dispatcher = transit_dispatcher.clone();
                let pool_transit_policy = PeerTransitPolicy::from_bool(options.allow_pool_transit);
                thread::spawn(move || {
                    let result = match live_transit_lane_registry.snapshot() {
                        Ok(transit_lane_document) if transit_lane_document.is_empty() => {
                            relay_local_sealed_transit_to_next_hop_with_limits(
                                local,
                                pool_transit_policy,
                                peer_pool,
                                first[0],
                                transit_limits,
                            )
                        }
                        Ok(transit_lane_document) => {
                            relay_local_sealed_transit_with_lane_document_and_first_byte_with_limits(
                                local,
                                &transit_lane_document,
                                Some(transit_dispatcher),
                                first[0],
                                transit_limits,
                            )
                        }
                        Err(error) => Err(error),
                    };
                    if let Err(error) = result {
                        eprintln!(
                            "event=weave_local_ingress_transit_error reason_class={}",
                            redacted_log_reason(&error)
                        );
                    }
                });
            }
            Ok(())
                if classify_local_ingress(first[0]) == LocalIngressBranch::BoundSealedTransit =>
            {
                let bound_dispatcher = transit_dispatcher.clone();
                let bound_policy = BoundPeerTransitPolicy::from_bool(options.allow_bound_transit);
                eprintln!("event=weave_local_ingress_bound_transit_branch");
                thread::spawn(move || {
                    if let Err(error) = relay_local_bound_sealed_transit_to_next_hop_with_limits(
                        local,
                        bound_policy,
                        Some(bound_dispatcher),
                        first[0],
                        transit_limits,
                    ) {
                        eprintln!(
                            "event=weave_local_ingress_bound_transit_error reason_class={}",
                            redacted_log_reason(&error)
                        );
                    }
                });
            }
            Ok(()) => {
                if classify_local_ingress(first[0]) == LocalIngressBranch::NativeConnect
                    && first[0] != LOCAL_MAGIC[0]
                {
                    eprintln!("event=weave_local_ingress_unsupported");
                    continue;
                }
                let peer_pool = peer_pool.clone();
                let live_transit_lane_registry = live_transit_lane_registry.clone();
                let transit_dispatcher = transit_dispatcher.clone();
                thread::spawn(move || {
                    let result = match live_transit_lane_registry.snapshot() {
                        Ok(transit_lane_document) if transit_lane_document.is_empty() => {
                            handle_local_client_with_peer_pool_and_first_byte(
                                local, peer_pool, first[0],
                            )
                        }
                        Ok(transit_lane_document) => {
                            handle_local_client_with_lane_document_and_first_byte(
                                local,
                                &transit_lane_document,
                                transit_dispatcher,
                                first[0],
                            )
                        }
                        Err(error) => Err(error),
                    };
                    if let Err(error) = result {
                        eprintln!(
                            "event=weave_local_ingress_error {}",
                            redacted_error_fields(&error)
                        );
                    }
                });
            }
            Err(error) => {
                eprintln!(
                    "event=weave_local_ingress_peek_failed reason_class={}",
                    redacted_log_reason(&error.to_string())
                );
                continue;
            }
        }
    }
    Ok(())
}

fn build_mesh_lane_driver_options(
    options: &Options,
    route_announcement_registry: Option<SharedRouteAnnouncementRegistry>,
) -> Result<MeshLaneDriverOptions, String> {
    let discovery_url = options
        .discovery_url
        .as_deref()
        .ok_or_else(|| "mesh lane driver requires discovery URL".to_string())?
        .to_string();
    let lane_document_path = options
        .lane_document_path
        .as_deref()
        .ok_or_else(|| "mesh lane driver requires lane document path".to_string())?
        .to_string();
    Ok(MeshLaneDriverOptions {
        namespace: options.mesh_namespace.clone(),
        self_node_id: options.mesh_self_node_id.clone(),
        policy_payload: options.mesh_policy_payload.clone(),
        lane_document_path,
        discovery_urls: discovery_url
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect(),
        discovery_keyring: options.discovery_keyring_map()?,
        discovery_timeout_ms: options.discovery_timeout_ms,
        poll_interval_ms: options.discovery_poll_interval_ms,
        route_announcement_registry,
    })
}

#[cfg(test)]
mod tests {
    use super::{LocalIngressBranch, classify_local_ingress};
    use crate::peer_egress::options::LOCAL_MAGIC;
    use crate::peer_egress::pool::new_shared_pool;

    #[test]
    fn transit_branch_is_reserved_for_frame_version() {
        assert_eq!(
            classify_local_ingress(chimera_session::FRAME_VERSION),
            LocalIngressBranch::SealedTransit
        );
    }

    #[test]
    fn native_connect_branch_handles_local_magic_prefix() {
        assert_eq!(
            classify_local_ingress(LOCAL_MAGIC[0]),
            LocalIngressBranch::NativeConnect
        );
    }

    #[test]
    fn bound_transit_branch_is_reserved_for_bound_transit_magic() {
        assert_eq!(
            classify_local_ingress(crate::peer_egress::transit_binding::BOUND_TRANSIT_MAGIC),
            LocalIngressBranch::BoundSealedTransit
        );
    }

    #[test]
    fn native_connect_branch_handles_other_bytes() {
        assert_eq!(
            classify_local_ingress(b'X'),
            LocalIngressBranch::NativeConnect
        );
    }

    #[test]
    fn local_sealed_transit_branch_has_pool_available() {
        let pool = new_shared_pool();
        assert!(matches!(pool.try_pop(), Ok(None)));
    }
}
