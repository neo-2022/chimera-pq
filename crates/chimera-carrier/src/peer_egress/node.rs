use std::io::Read;
use std::thread;
use std::time::Duration;

use crate::peer_egress::handshake::{authenticate_peer, establish_secure_peer_server};
use crate::peer_egress::live_bindings::LiveTransitLaneRegistry;
use crate::peer_egress::modes::{
    handle_local_client_with_peer_pool_and_first_byte,
    handle_local_client_with_registrations_and_first_byte, outbound_peer_worker_with_next_hop,
};
use crate::peer_egress::net::{bind_reuse_listener, tune_tcp};
use crate::peer_egress::options::{LOCAL_MAGIC, Options, write_resolved_state_file};
use crate::peer_egress::pool::new_shared_pool;
use crate::peer_egress::protocol::redacted_log_reason;
use crate::peer_egress::startup_contract::validate_node_startup_contract;
use crate::peer_egress::transit::{
    BoundPeerTransitPolicy, PeerTransitPolicy, relay_local_bound_sealed_transit_to_next_hop,
    relay_local_sealed_transit_to_next_hop, relay_local_sealed_transit_with_registrations,
};
use crate::peer_egress::transit_binding::BOUND_TRANSIT_MAGIC;
use crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher;

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

pub fn run_node(options: Options) -> Result<(), String> {
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
    let peer_pool = new_shared_pool();
    let transit_dispatcher = new_shared_transit_dispatcher();
    let live_transit_lane_registry =
        LiveTransitLaneRegistry::start(&options, transit_dispatcher.clone())?;
    let ingress_pool = peer_pool.clone();
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
                Ok(peer) => {
                    eprintln!("event=weave_peer_ingress_authenticated");
                    if let Err(error) = ingress_pool.push(peer) {
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

    if startup_contract.outbound_bootstrap_configured {
        for _ in 0..options.pool {
            let worker = options.clone();
            let outbound_pool = peer_pool.clone();
            let outbound_dispatcher = transit_dispatcher.clone();
            thread::spawn(move || {
                loop {
                    if let Err(error) = outbound_peer_worker_with_next_hop(
                        &worker,
                        Some(outbound_pool.clone()),
                        Some(outbound_dispatcher.clone()),
                    ) {
                        eprintln!(
                            "event=weave_outbound_peer_worker_error reason_class={}",
                            redacted_log_reason(&error)
                        );
                        thread::sleep(Duration::from_secs(1));
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
                        Ok(transit_lane_registrations) if transit_lane_registrations.is_empty() => {
                            relay_local_sealed_transit_to_next_hop(
                                local,
                                pool_transit_policy,
                                peer_pool,
                                first[0],
                            )
                        }
                        Ok(transit_lane_registrations) => {
                            relay_local_sealed_transit_with_registrations(
                                local,
                                transit_lane_registrations.as_slice(),
                                Some(transit_dispatcher),
                                first[0],
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
                    if let Err(error) = relay_local_bound_sealed_transit_to_next_hop(
                        local,
                        bound_policy,
                        Some(bound_dispatcher),
                        first[0],
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
                        Ok(transit_lane_registrations) if transit_lane_registrations.is_empty() => {
                            handle_local_client_with_peer_pool_and_first_byte(
                                local, peer_pool, first[0],
                            )
                        }
                        Ok(transit_lane_registrations) => {
                            handle_local_client_with_registrations_and_first_byte(
                                local,
                                transit_lane_registrations.as_slice(),
                                transit_dispatcher,
                                first[0],
                            )
                        }
                        Err(error) => Err(error),
                    };
                    if let Err(error) = result {
                        eprintln!(
                            "event=weave_local_ingress_error reason_class={}",
                            redacted_log_reason(&error)
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
