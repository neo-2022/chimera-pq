use std::io::Read;
use std::thread;
use std::time::Duration;

use crate::peer_egress::handshake::{authenticate_peer, establish_secure_peer_server};
use crate::peer_egress::lane_binding::load_transit_lane_registrations;
use crate::peer_egress::modes::{
    handle_local_client_with_first_byte, outbound_peer_worker_with_next_hop,
    outbound_transit_lane_registration_worker,
};
use crate::peer_egress::net::{bind_reuse_listener, tune_tcp};
use crate::peer_egress::options::{LOCAL_MAGIC, Options, write_resolved_state_file};
use crate::peer_egress::pool::{SharedPeerPool, UniquePeerPop, new_shared_pool};
use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::protocol::redacted_log_reason;
use crate::peer_egress::startup_contract::validate_node_startup_contract;
use crate::peer_egress::transit::{
    BoundPeerTransitPolicy, relay_local_bound_sealed_transit_to_next_hop,
    relay_local_sealed_transit,
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

fn pop_next_local_transit_next_hop(peer_pool: &SharedPeerPool) -> Result<SecurePeerStream, String> {
    match peer_pool.try_pop_unique()? {
        UniquePeerPop::Ready(peer) => Ok(peer),
        UniquePeerPop::Unavailable => Err("sealed local transit next hop unavailable".to_string()),
        UniquePeerPop::Ambiguous => {
            Err("sealed local transit next hop ambiguous without path binding".to_string())
        }
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
        eprintln!("event=weave_node_state_write_failed reason={error}");
    }

    let token = options.token.clone();
    let aead = options.aead;
    let peer_pool = new_shared_pool();
    let transit_dispatcher = new_shared_transit_dispatcher();
    let transit_lane_registrations = match &options.transit_lane_bindings_file {
        Some(path) => load_transit_lane_registrations(path)?,
        None => Vec::new(),
    };
    let ingress_pool = peer_pool.clone();
    thread::spawn(move || {
        for incoming in peer_listener.incoming() {
            let Ok(mut stream) = incoming else {
                continue;
            };
            if let Err(error) = tune_tcp(&stream) {
                eprintln!("event=weave_peer_socket_tune_failed reason={error}");
            }
            match authenticate_peer(&mut stream, &token)
                .and_then(|_| establish_secure_peer_server(stream, &token, aead))
            {
                Ok(peer) => {
                    eprintln!("event=weave_peer_ingress_authenticated");
                    if let Err(error) = ingress_pool.push(peer) {
                        eprintln!("event=weave_peer_pool_push_failed reason={error}");
                    }
                }
                Err(error) => {
                    eprintln!("event=weave_peer_ingress_auth_failed reason={error}");
                }
            }
        }
    });

    for registration in transit_lane_registrations {
        let worker = options.clone();
        let dispatcher = transit_dispatcher.clone();
        thread::spawn(move || {
            loop {
                if let Err(error) = outbound_transit_lane_registration_worker(
                    &worker,
                    &registration,
                    dispatcher.clone(),
                ) {
                    eprintln!(
                        "event=weave_transit_lane_worker_error reason_class={}",
                        redacted_log_reason(&error)
                    );
                    thread::sleep(Duration::from_secs(1));
                }
            }
        });
    }

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
                let peer = match pop_next_local_transit_next_hop(&peer_pool) {
                    Ok(peer) => peer,
                    Err(error) => {
                        eprintln!(
                            "event=weave_local_ingress_transit_rejected reason_class={}",
                            redacted_log_reason(&error)
                        );
                        continue;
                    }
                };
                eprintln!("event=weave_local_ingress_transit_branch");
                thread::spawn(move || {
                    if let Err(error) = relay_local_sealed_transit(local, peer, first[0]) {
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
                let Ok(peer) = peer_pool.pop_wait() else {
                    continue;
                };
                eprintln!("event=weave_local_ingress_paired_with_peer");
                thread::spawn(move || {
                    if let Err(error) = handle_local_client_with_first_byte(local, peer, first[0]) {
                        eprintln!(
                            "event=weave_local_ingress_error reason_class={}",
                            redacted_log_reason(&error)
                        );
                    }
                });
            }
            Err(error) => {
                eprintln!("event=weave_local_ingress_peek_failed reason={error}");
                continue;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{LocalIngressBranch, classify_local_ingress, pop_next_local_transit_next_hop};
    use crate::peer_egress::options::AeadSuite;
    use crate::peer_egress::options::LOCAL_MAGIC;
    use crate::peer_egress::pool::new_shared_pool;
    use crate::peer_egress::protocol::SecurePeerStream;

    fn test_peer_stream() -> Result<SecurePeerStream, String> {
        let transcript = chimera_crypto::TranscriptHash::from_messages(&[b"local-transit-test"]);
        let secrets = chimera_crypto::derive_traffic_secrets(
            chimera_crypto::SuiteId(AeadSuite::Chacha20Poly1305.suite_id()),
            &transcript,
            &[7_u8; 32],
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
    fn local_sealed_transit_requires_available_next_hop() -> Result<(), String> {
        let pool = new_shared_pool();
        let error = match pop_next_local_transit_next_hop(&pool) {
            Ok(_) => return Err("missing local transit next hop must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("unavailable"));
        Ok(())
    }

    #[test]
    fn local_sealed_transit_accepts_single_next_hop() -> Result<(), String> {
        let pool = new_shared_pool();
        pool.push(test_peer_stream()?)?;
        let peer = pop_next_local_transit_next_hop(&pool)?;
        drop(peer);
        assert!(pool.try_pop()?.is_none());
        Ok(())
    }

    #[test]
    fn local_sealed_transit_rejects_ambiguous_next_hop() -> Result<(), String> {
        let pool = new_shared_pool();
        pool.push(test_peer_stream()?)?;
        pool.push(test_peer_stream()?)?;
        let error = match pop_next_local_transit_next_hop(&pool) {
            Ok(_) => return Err("ambiguous local transit next hop must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("ambiguous"));
        assert!(pool.try_pop()?.is_some());
        assert!(pool.try_pop()?.is_some());
        Ok(())
    }
}
