use std::io::Write;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::peer_egress::aggregate_ingress::{
    AggregateTransitIngressLimits, SharedAggregateTransitIngressRegistry,
    new_shared_aggregate_transit_ingress_registry,
};
use crate::peer_egress::aggregate_peer_ingress::handle_aggregate_peer_ingress_shard;
use crate::peer_egress::handshake::{
    authenticate_peer, establish_secure_peer_client, establish_secure_peer_server,
};
use crate::peer_egress::lane_binding::TransitLaneDocument;
use crate::peer_egress::net::{
    bind_reuse_listener, connect_tcp, pipe_secure_peer_with_plain, tune_tcp,
};
use crate::peer_egress::options::{Options, write_resolved_state_file};
use crate::peer_egress::pool::{PeerPool, SharedPeerPool, new_shared_pool};
use crate::peer_egress::protocol::{
    SecurePeerStream, redacted_destination_label, redacted_log_reason,
};
use crate::peer_egress::transit::{BoundPeerTransitPolicy, PeerTransitPolicy};
use crate::peer_egress::transit_dispatch::{
    SharedTransitNextHopDispatcher, new_shared_transit_dispatcher,
};
use crate::peer_egress::transit_document::forward_peer_sealed_transit_with_lane_document_and_limits;
use crate::peer_egress::transit_guard::TransitRelayLimits;
use crate::peer_egress::wire::{PeerMessage, read_peer_message, write_ack_ok};
use crate::peer_egress::route_announcement_registry::{
    SharedRouteAnnouncementRegistry, merge_received_announcements,
};

pub mod lab;
#[path = "modes_local_ingress.rs"]
mod local_ingress;
#[path = "modes_reverse.rs"]
mod reverse;

pub use lab::{
    run_bench, run_download_echo, run_download_probe, run_echo, run_probe, start_side_a_runtime,
};
pub use local_ingress::{
    handle_local_client, handle_local_client_with_first_byte,
    handle_local_client_with_lane_document_and_first_byte, handle_local_client_with_peer_pool,
    handle_local_client_with_peer_pool_and_first_byte,
    handle_local_client_with_registrations_and_first_byte, read_local_connect_destination,
};
#[cfg(test)]
pub(crate) use reverse::handle_reverse_peer_with_lane_document_and_aggregate_ingress;
pub use reverse::{
    handle_reverse_local_client, handle_reverse_peer, handle_reverse_peer_with_lane_document,
};

pub fn run_side_a(options: Options) -> Result<(), String> {
    let peer_listener = bind_reuse_listener(&options.peer_listen)
        .map_err(|error| format!("bind peer listener failed: {error}"))?;
    let local_listener = bind_reuse_listener(&options.local_listen)
        .map_err(|error| format!("bind local listener failed: {error}"))?;
    let resolved_peer_listen = peer_listener
        .local_addr()
        .map_err(|error| format!("resolve peer listener addr failed: {error}"))?
        .to_string();
    let resolved_local_listen = local_listener
        .local_addr()
        .map_err(|error| format!("resolve local listener addr failed: {error}"))?
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
            "event=peer_state_write_failed reason_class={}",
            redacted_log_reason(&error)
        );
    }
    let token = options.token.clone();
    let aead = options.aead;
    let reverse_connect = options.reverse_connect;
    let peer_transit_policy = PeerTransitPolicy::from_bool(options.allow_pool_transit);
    let bound_transit_policy = BoundPeerTransitPolicy::from_bool(options.allow_bound_transit);
    let transit_dispatcher = new_shared_transit_dispatcher();
    let aggregate_ingress =
        new_shared_aggregate_transit_ingress_registry(AggregateTransitIngressLimits::default())?;
    let transit_limits = options.transit_relay_limits();
    if reverse_connect {
        let peer_pool = new_shared_pool();
        let r_pool = peer_pool;
        let r_token = token;
        let r_dispatcher = transit_dispatcher;
        let r_aggregate_ingress = aggregate_ingress;
        let r_transit_limits = transit_limits;
        thread::spawn(move || {
            for incoming in peer_listener.incoming() {
                let Ok(mut stream) = incoming else {
                    continue;
                };
                if let Err(error) = tune_tcp(&stream) {
                    eprintln!(
                        "event=peer_socket_tune_failed reason_class={}",
                        redacted_log_reason(&error)
                    );
                }
                match authenticate_peer(&mut stream, &r_token)
                    .and_then(|_| establish_secure_peer_server(stream, &r_token, aead))
                {
                    Ok(peer) => {
                        eprintln!("event=reverse_peer_authenticated");
                        let pool = r_pool.clone();
                        let policy = peer_transit_policy;
                        let bound_policy = bound_transit_policy;
                        let dispatcher = r_dispatcher.clone();
                        let aggregate_ingress = r_aggregate_ingress.clone();
                        thread::spawn(move || {
                            if let Err(error) = reverse::handle_reverse_peer_with_context(
                                peer,
                                policy,
                                bound_policy,
                                reverse::ReversePeerTransitContext::new(
                                    pool,
                                    Some(dispatcher),
                                    None,
                                    Some(aggregate_ingress),
                                    r_transit_limits,
                                ),
                            ) {
                                eprintln!(
                                    "event=reverse_peer_error reason_class={}",
                                    redacted_log_reason(&error)
                                );
                            }
                        });
                    }
                    Err(error) => {
                        eprintln!(
                            "event=reverse_peer_auth_failed reason_class={}",
                            redacted_log_reason(&error)
                        );
                    }
                }
            }
        });
        println!(
            "chimera_peer_egress=side_a_reverse_ready local={} peer={} resolved_local={} resolved_peer={}",
            options.local_listen, options.peer_listen, resolved_local_listen, resolved_peer_listen
        );
        for incoming in local_listener.incoming() {
            let Ok(local) = incoming else {
                continue;
            };
            eprintln!("event=reverse_local_ingress_accepted");
            thread::spawn(move || {
                if let Err(error) = handle_reverse_local_client(local) {
                    eprintln!(
                        "event=reverse_local_client_error reason_class={}",
                        redacted_log_reason(&error)
                    );
                }
            });
        }
    } else {
        let pool = Arc::new(PeerPool::default());
        let peer_pool = Arc::clone(&pool);
        thread::spawn(move || {
            for incoming in peer_listener.incoming() {
                let Ok(mut stream) = incoming else {
                    continue;
                };
                if let Err(error) = tune_tcp(&stream) {
                    eprintln!(
                        "event=peer_socket_tune_failed reason_class={}",
                        redacted_log_reason(&error)
                    );
                }
                match authenticate_peer(&mut stream, &token)
                    .and_then(|_| establish_secure_peer_server(stream, &token, aead))
                {
                    Ok(peer) => {
                        eprintln!("event=peer_authenticated");
                        let _ = peer_pool.push(peer);
                    }
                    Err(error) => {
                        eprintln!(
                            "event=peer_auth_failed reason_class={}",
                            redacted_log_reason(&error)
                        );
                    }
                }
            }
        });
        println!(
            "chimera_peer_egress=side_a_ready local={} peer={} resolved_local={} resolved_peer={}",
            options.local_listen, options.peer_listen, resolved_local_listen, resolved_peer_listen
        );
        for incoming in local_listener.incoming() {
            let Ok(local) = incoming else {
                continue;
            };
            eprintln!("event=local_ingress_accepted");
            let peer_pool = pool.clone();
            thread::spawn(move || {
                if let Err(error) = handle_local_client_with_peer_pool(local, peer_pool) {
                    eprintln!(
                        "event=local_ingress_error reason_class={}",
                        redacted_log_reason(&error)
                    );
                }
            });
        }
    }
    Ok(())
}

pub fn run_side_b(options: Options) -> Result<(), String> {
    println!(
        "chimera_peer_egress=side_b_connecting server=<redacted> server_label={} pool={}",
        redacted_destination_label(
            options
                .server
                .split_once(':')
                .map(|(host, _)| host)
                .unwrap_or(""),
            options
                .server
                .rsplit_once(':')
                .and_then(|(_, port)| port.parse::<u16>().ok())
                .unwrap_or(0)
        ),
        options.pool
    );
    for _ in 0..options.pool {
        let worker = options.clone();
        thread::spawn(move || {
            loop {
                if let Err(error) = outbound_peer_worker(&worker) {
                    eprintln!(
                        "event=outbound_peer_worker_error reason_class={}",
                        redacted_log_reason(&error)
                    );
                    thread::sleep(Duration::from_millis(500));
                }
            }
        });
    }
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}

pub fn side_b_worker(options: &Options) -> Result<(), String> {
    outbound_peer_worker(options)
}

pub fn outbound_peer_worker(options: &Options) -> Result<(), String> {
    outbound_peer_worker_with_next_hop(options, None, None)
}

pub fn outbound_peer_worker_with_next_hop(
    options: &Options,
    next_hops: Option<SharedPeerPool>,
    next_hop_dispatcher: Option<SharedTransitNextHopDispatcher>,
) -> Result<(), String> {
    outbound_peer_worker_with_next_hop_and_lane_document(
        options,
        next_hops,
        next_hop_dispatcher,
        None,
    )
}

pub(crate) fn outbound_peer_worker_with_next_hop_and_lane_document(
    options: &Options,
    next_hops: Option<SharedPeerPool>,
    next_hop_dispatcher: Option<SharedTransitNextHopDispatcher>,
    lane_document: Option<&TransitLaneDocument>,
) -> Result<(), String> {
    outbound_peer_worker_with_next_hop_lane_document_and_aggregate_ingress(
        options,
        next_hops,
        next_hop_dispatcher,
        lane_document,
        None,
        None,
    )
}


fn connect_and_establish_outbound_peer(options: &Options) -> Result<SecurePeerStream, String> {
    eprintln!("event=carrier_reconnect_attempt reason_class=carrier_peer_connect");
    let mut peer = connect_tcp(&options.server, options.connect_timeout_ms)
        .map_err(|error| format!("connect outbound peer failed: {error}"))?;
    tune_tcp(&peer)?;
    eprintln!("event=carrier_reconnect_tcp_connected");
    peer.write_all(b"CHIMERA-PEER-EGRESS/1\n")
        .map_err(|error| format!("write handshake failed: {error}"))?;
    peer.write_all(options.token.as_bytes())
        .and_then(|_| peer.write_all(b"\n"))
        .map_err(|error| format!("write token failed: {error}"))?;
    let peer = establish_secure_peer_client(peer, &options.token, options.aead)?;
    eprintln!("event=carrier_reconnect_success");
    Ok(peer)
}

pub(crate) fn outbound_peer_worker_with_next_hop_lane_document_and_aggregate_ingress(
    options: &Options,
    next_hops: Option<SharedPeerPool>,
    next_hop_dispatcher: Option<SharedTransitNextHopDispatcher>,
    lane_document: Option<&TransitLaneDocument>,
    aggregate_ingress: Option<SharedAggregateTransitIngressRegistry>,
    route_announcement_registry: Option<SharedRouteAnnouncementRegistry>,
) -> Result<(), String> {
    let trusted_keys = announce_keyring(options)?;
    let mut peer = connect_and_establish_outbound_peer(options)?;
    let transit_limits = options.transit_relay_limits();
    transit_limits.validate()?;
    let previous_read_timeout = peer
        .stream
        .read_timeout()
        .map_err(|error| format!("read first peer timeout failed: {error}"))?;
    peer.stream
        .set_read_timeout(Some(transit_limits.idle_timeout()))
        .map_err(|error| format!("set first peer read timeout failed: {error}"))?;
    let mut first_message = read_peer_message(&mut peer, 512)?;
    loop {
        match &first_message {
            PeerMessage::Announce(announcements) => {
                if let Some(registry) = &route_announcement_registry {
                    merge_received_announcements(registry, announcements, trusted_keys.as_ref())?;
                }
                first_message = read_peer_message(&mut peer, 512)?;
                continue;
            }
            _ => break,
        }
    }
    let destination = match first_message {
        PeerMessage::Connect(destination) => {
            peer.stream
                .set_read_timeout(previous_read_timeout)
                .map_err(|error| format!("restore peer read timeout failed: {error}"))?;
            destination
        }
        PeerMessage::SealedTransit(frame) => {
            return forward_peer_sealed_transit_with_document_or_pool_and_limits(
                peer,
                PeerTransitPolicy::from_bool(options.allow_pool_transit),
                next_hops,
                next_hop_dispatcher,
                lane_document,
                frame,
                transit_limits,
            );
        }
        PeerMessage::BoundSealedTransit(frame) => {
            return crate::peer_egress::transit::forward_bound_peer_sealed_transit_to_next_hop_with_limits(
                peer,
                BoundPeerTransitPolicy::from_bool(options.allow_bound_transit),
                next_hop_dispatcher,
                frame,
                transit_limits,
            );
        }
        PeerMessage::AggregateSealedTransit(shard) => {
            handle_aggregate_peer_ingress_shard(
                shard,
                aggregate_ingress,
                PeerTransitPolicy::from_bool(options.allow_pool_transit),
                next_hops,
                next_hop_dispatcher,
                lane_document,
                transit_limits,
            )?;
            return Ok(());
        }
        PeerMessage::AckOk => return Err("unexpected peer ack before request".to_string()),
        PeerMessage::Announce(_) => {
            return Err("unexpected peer announce after announce loop".to_string())
        }
    };
    let target_addr = destination.connect_addr();
    let destination_id = destination.redacted_label();
    eprintln!(
        "event=outbound_peer_request_received request=<redacted> destination_id={destination_id}"
    );
    eprintln!(
        "event=outbound_peer_target_connecting target=<redacted> destination_id={destination_id}"
    );
    let target = connect_tcp(&target_addr, options.connect_timeout_ms)
        .map_err(|error| format!("connect outbound target failed: {error}"))?;
    tune_tcp(&target)?;
    eprintln!(
        "event=outbound_peer_target_connected target=<redacted> destination_id={destination_id}"
    );
    write_ack_ok(&mut peer)?;
    eprintln!(
        "event=outbound_peer_connect_ack_sent target=<redacted> destination_id={destination_id}"
    );
    pipe_secure_peer_with_plain(peer, target)
}

fn forward_peer_sealed_transit_with_document_or_pool_and_limits(
    peer: SecurePeerStream,
    policy: PeerTransitPolicy,
    next_hops: Option<SharedPeerPool>,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    lane_document: Option<&TransitLaneDocument>,
    first: crate::peer_egress::transit::TransitRelayFrame,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    if let Some(document) = lane_document
        && !document.is_empty()
    {
        return forward_peer_sealed_transit_with_lane_document_and_limits(
            peer, document, dispatcher, first, limits,
        );
    }
    crate::peer_egress::transit::forward_peer_sealed_transit_to_next_hop_with_limits(
        peer, policy, next_hops, first, limits,
    )
}

pub(crate) fn serve_peer_pool_worker(
    options: &Options,
    next_hops: Option<SharedPeerPool>,
    next_hop_dispatcher: Option<SharedTransitNextHopDispatcher>,
    lane_document: Option<&TransitLaneDocument>,
    aggregate_ingress: Option<SharedAggregateTransitIngressRegistry>,
    route_announcement_registry: Option<SharedRouteAnnouncementRegistry>,
    mut peer: SecurePeerStream,
) -> Result<(), String> {
    let trusted_keys = announce_keyring(options)?;
    let transit_limits = options.transit_relay_limits();
    transit_limits.validate()?;
    let previous_read_timeout = peer
        .stream
        .read_timeout()
        .map_err(|error| format!("read first peer timeout failed: {error}"))?;
    peer.stream
        .set_read_timeout(Some(transit_limits.idle_timeout()))
        .map_err(|error| format!("set first peer read timeout failed: {error}"))?;
    let mut first_message = read_peer_message(&mut peer, 512)?;
    loop {
        match &first_message {
            PeerMessage::Announce(announcements) => {
                if let Some(registry) = &route_announcement_registry {
                    merge_received_announcements(registry, announcements, trusted_keys.as_ref())?;
                }
                first_message = read_peer_message(&mut peer, 512)?;
                continue;
            }
            _ => break,
        }
    }
    let destination = match first_message {
        PeerMessage::Connect(destination) => {
            peer.stream
                .set_read_timeout(previous_read_timeout)
                .map_err(|error| format!("restore peer read timeout failed: {error}"))?;
            destination
        }
        PeerMessage::SealedTransit(frame) => {
            return forward_peer_sealed_transit_with_document_or_pool_and_limits(
                peer,
                PeerTransitPolicy::from_bool(options.allow_pool_transit),
                next_hops,
                next_hop_dispatcher,
                lane_document,
                frame,
                transit_limits,
            );
        }
        PeerMessage::BoundSealedTransit(frame) => {
            return crate::peer_egress::transit::forward_bound_peer_sealed_transit_to_next_hop_with_limits(
                peer,
                BoundPeerTransitPolicy::from_bool(options.allow_bound_transit),
                next_hop_dispatcher,
                frame,
                transit_limits,
            );
        }
        PeerMessage::AggregateSealedTransit(shard) => {
            handle_aggregate_peer_ingress_shard(
                shard,
                aggregate_ingress,
                PeerTransitPolicy::from_bool(options.allow_pool_transit),
                next_hops,
                next_hop_dispatcher,
                lane_document,
                transit_limits,
            )?;
            return Ok(());
        }
        PeerMessage::AckOk => return Err("unexpected peer ack before request".to_string()),
        PeerMessage::Announce(_) => {
            return Err("unexpected peer announce after announce loop".to_string())
        }
    };
    let target_addr = destination.connect_addr();
    let destination_id = destination.redacted_label();
    eprintln!(
        "event=peer_pool_request_received request=<redacted> destination_id={destination_id}"
    );
    eprintln!(
        "event=peer_pool_target_connecting target=<redacted> destination_id={destination_id}"
    );
    let target = connect_tcp(&target_addr, options.connect_timeout_ms)
        .map_err(|error| format!("connect peer pool target failed: {error}"))?;
    tune_tcp(&target)?;
    eprintln!("event=peer_pool_target_connected target=<redacted> destination_id={destination_id}");
    write_ack_ok(&mut peer)?;
    eprintln!("event=peer_pool_connect_ack_sent target=<redacted> destination_id={destination_id}");
    pipe_secure_peer_with_plain(peer, target)
}

fn announce_keyring(
    options: &Options,
) -> Result<Option<std::collections::BTreeMap<String, Vec<u8>>>, String> {
    let map = options.mesh_announcement_keyring_map()?;
    if map.is_empty() {
        Ok(None)
    } else {
        Ok(Some(map))
    }
}

#[cfg(test)]
#[path = "modes_tests/local_egress.rs"]
mod local_egress_tests;
