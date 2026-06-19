use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::peer_egress::handshake::{
    authenticate_peer, establish_secure_peer_client, establish_secure_peer_server,
};
use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::live_lane_selection::select_carrier_lane_from_registrations;
use crate::peer_egress::net::{
    bind_reuse_listener, connect_tcp, pipe_plain_with_secure_peer, pipe_secure_peer_with_plain,
    tune_tcp,
};
use crate::peer_egress::options::{LOCAL_MAGIC, Options, write_resolved_state_file};
use crate::peer_egress::pool::{PeerPool, SharedPeerPool, new_shared_pool};
use crate::peer_egress::protocol::{
    Destination, SecurePeerStream, read_native_connect_destination, redacted_destination_label,
    redacted_log_reason,
};
use crate::peer_egress::transit::{
    BoundPeerTransitPolicy, PeerTransitPolicy, forward_bound_peer_sealed_transit_to_next_hop,
    forward_peer_sealed_transit_to_next_hop,
};
use crate::peer_egress::transit_binding::TransitPathBinding;
use crate::peer_egress::transit_dispatch::{
    SharedTransitNextHopDispatcher, new_shared_transit_dispatcher,
};
use crate::peer_egress::wire::{
    PeerMessage, read_peer_message, write_ack_ok, write_connect_message,
};
use chimera_mesh::MeshMultipathFlowKey;

pub mod lab;

pub use lab::{
    run_bench, run_download_echo, run_download_probe, run_echo, run_probe, start_vps_runtime,
};

pub fn run_vps(options: Options) -> Result<(), String> {
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
        eprintln!("event=peer_state_write_failed reason={error}");
    }
    let token = options.token.clone();
    let aead = options.aead;
    let reverse_connect = options.reverse_connect;
    let peer_transit_policy = PeerTransitPolicy::from_bool(options.allow_pool_transit);
    let bound_transit_policy = BoundPeerTransitPolicy::from_bool(options.allow_bound_transit);
    let transit_dispatcher = new_shared_transit_dispatcher();
    if reverse_connect {
        let peer_pool = new_shared_pool();
        let r_pool = peer_pool.clone();
        let r_token = token.clone();
        let r_dispatcher = transit_dispatcher.clone();
        thread::spawn(move || {
            for incoming in peer_listener.incoming() {
                let Ok(mut stream) = incoming else {
                    continue;
                };
                if let Err(error) = tune_tcp(&stream) {
                    eprintln!("event=peer_socket_tune_failed reason={error}");
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
                        thread::spawn(move || {
                            if let Err(error) = handle_reverse_peer(
                                peer,
                                policy,
                                bound_policy,
                                pool,
                                Some(dispatcher),
                            ) {
                                eprintln!(
                                    "event=reverse_peer_error reason_class={}",
                                    redacted_log_reason(&error)
                                );
                            }
                        });
                    }
                    Err(error) => {
                        eprintln!("event=reverse_peer_auth_failed reason={error}");
                    }
                }
            }
        });
        println!(
            "chimera_peer_egress=vps_reverse_ready local={} peer={} resolved_local={} resolved_peer={}",
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
                    eprintln!("event=peer_socket_tune_failed reason={error}");
                }
                match authenticate_peer(&mut stream, &token)
                    .and_then(|_| establish_secure_peer_server(stream, &token, aead))
                {
                    Ok(peer) => {
                        eprintln!("event=peer_authenticated");
                        let _ = peer_pool.push(peer);
                    }
                    Err(error) => {
                        eprintln!("event=peer_auth_failed reason={error}");
                    }
                }
            }
        });
        println!(
            "chimera_peer_egress=vps_ready local={} peer={} resolved_local={} resolved_peer={}",
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

pub fn handle_reverse_peer(
    mut peer: SecurePeerStream,
    policy: PeerTransitPolicy,
    bound_policy: BoundPeerTransitPolicy,
    pool: SharedPeerPool,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
) -> Result<(), String> {
    let destination = match read_peer_message(&mut peer, 512)? {
        PeerMessage::Connect(destination) => destination,
        PeerMessage::SealedTransit(frame) => {
            return forward_peer_sealed_transit_to_next_hop(peer, policy, Some(pool), frame);
        }
        PeerMessage::BoundSealedTransit(frame) => {
            return forward_bound_peer_sealed_transit_to_next_hop(
                peer,
                bound_policy,
                dispatcher,
                frame,
            );
        }
        PeerMessage::AckOk => return Err("unexpected peer ack before request".to_string()),
    };
    let target_addr = destination.connect_addr();
    let destination_id = destination.redacted_label();
    eprintln!(
        "event=reverse_peer_request_received request=<redacted> destination_id={destination_id}"
    );
    eprintln!(
        "event=reverse_peer_target_connecting target=<redacted> destination_id={destination_id}"
    );
    let target = connect_tcp(&target_addr, 10_000)
        .map_err(|error| format!("reverse connect target failed: {error}"))?;
    tune_tcp(&target)?;
    eprintln!(
        "event=reverse_peer_target_connected target=<redacted> destination_id={destination_id}"
    );
    write_ack_ok(&mut peer)?;
    eprintln!(
        "event=reverse_peer_connect_ack_sent target=<redacted> destination_id={destination_id}"
    );
    pipe_secure_peer_with_plain(peer, target)
}

pub fn handle_reverse_local_client(mut local: TcpStream) -> Result<(), String> {
    tune_tcp(&local)?;
    let mut first = [0_u8; 1];
    local
        .read_exact(&mut first)
        .map_err(|error| format!("read reverse local protocol byte failed: {error}"))?;
    let destination = if first[0] == LOCAL_MAGIC[0] {
        read_native_connect_destination(&mut local, first[0])?
    } else {
        return Err(
            "unsupported reverse local ingress protocol; expected CHIMERA-LOCAL/1".to_string(),
        );
    };
    let destination_id = destination.redacted_label();
    eprintln!(
        "event=reverse_local_ingress_destination host=<redacted> port=<redacted> destination_id={destination_id}"
    );
    let target_addr = destination.connect_addr();
    eprintln!(
        "event=reverse_local_target_connecting target=<redacted> destination_id={destination_id}"
    );
    let target = connect_tcp(&target_addr, 10_000)
        .map_err(|error| format!("reverse local connect target failed: {error}"))?;
    tune_tcp(&target)?;
    eprintln!(
        "event=reverse_local_target_connected target=<redacted> destination_id={destination_id}"
    );
    local
        .write_all(b"OK\n")
        .map_err(|error| format!("write reverse local ack failed: {error}"))?;
    eprintln!("event=reverse_local_ack_sent");
    crate::peer_egress::net::relay_plain(local, target)
}

pub fn run_laptop(options: Options) -> Result<(), String> {
    println!(
        "chimera_peer_egress=laptop_connecting server=<redacted> server_label={} pool={}",
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
                    thread::sleep(Duration::from_secs(1));
                }
            }
        });
    }
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}

pub fn laptop_worker(options: &Options) -> Result<(), String> {
    outbound_peer_worker(options)
}

pub fn outbound_peer_worker(options: &Options) -> Result<(), String> {
    outbound_peer_worker_with_next_hop(options, None, None)
}

pub fn outbound_transit_lane_registration_worker(
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

pub fn outbound_peer_worker_with_next_hop(
    options: &Options,
    next_hops: Option<SharedPeerPool>,
    next_hop_dispatcher: Option<SharedTransitNextHopDispatcher>,
) -> Result<(), String> {
    let mut peer = connect_tcp(&options.server, options.connect_timeout_ms)
        .map_err(|error| format!("connect outbound peer failed: {error}"))?;
    tune_tcp(&peer)?;
    eprintln!("event=outbound_peer_connected");
    peer.write_all(b"CHIMERA-PEER-EGRESS/1\n")
        .map_err(|error| format!("write handshake failed: {error}"))?;
    peer.write_all(options.token.as_bytes())
        .and_then(|_| peer.write_all(b"\n"))
        .map_err(|error| format!("write token failed: {error}"))?;
    let mut peer = establish_secure_peer_client(peer, &options.token, options.aead)?;
    let destination = match read_peer_message(&mut peer, 512)? {
        PeerMessage::Connect(destination) => destination,
        PeerMessage::SealedTransit(frame) => {
            return forward_peer_sealed_transit_to_next_hop(
                peer,
                PeerTransitPolicy::from_bool(options.allow_pool_transit),
                next_hops,
                frame,
            );
        }
        PeerMessage::BoundSealedTransit(frame) => {
            return forward_bound_peer_sealed_transit_to_next_hop(
                peer,
                BoundPeerTransitPolicy::from_bool(options.allow_bound_transit),
                next_hop_dispatcher,
                frame,
            );
        }
        PeerMessage::AckOk => return Err("unexpected peer ack before request".to_string()),
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

pub fn handle_local_client(mut local: TcpStream, peer: SecurePeerStream) -> Result<(), String> {
    tune_tcp(&local)?;
    let mut first = [0_u8; 1];
    local
        .read_exact(&mut first)
        .map_err(|error| format!("read local protocol byte failed: {error}"))?;
    handle_local_client_with_first_byte(local, peer, first[0])
}

pub fn handle_local_client_with_first_byte(
    mut local: TcpStream,
    peer: SecurePeerStream,
    first_byte: u8,
) -> Result<(), String> {
    let destination = read_local_connect_destination(&mut local, first_byte)?;
    connect_local_client_via_peer(local, peer, destination)
}

fn require_peer_ack(peer: &mut SecurePeerStream) -> Result<(), String> {
    match read_peer_message(peer, 16)? {
        PeerMessage::AckOk => Ok(()),
        PeerMessage::Connect(_) => Err("peer returned unexpected connect request".to_string()),
        PeerMessage::SealedTransit(_) => Err("peer returned unexpected transit frame".to_string()),
        PeerMessage::BoundSealedTransit(_) => {
            Err("peer returned unexpected bound transit frame".to_string())
        }
    }
}

pub fn handle_local_client_with_peer_pool(
    mut local: TcpStream,
    peer_pool: SharedPeerPool,
) -> Result<(), String> {
    tune_tcp(&local)?;
    let mut first = [0_u8; 1];
    local
        .read_exact(&mut first)
        .map_err(|error| format!("read local protocol byte failed: {error}"))?;
    handle_local_client_with_peer_pool_and_first_byte(local, peer_pool, first[0])
}

pub fn handle_local_client_with_peer_pool_and_first_byte(
    mut local: TcpStream,
    peer_pool: SharedPeerPool,
    first_byte: u8,
) -> Result<(), String> {
    let destination = read_local_connect_destination(&mut local, first_byte)?;
    let destination_id = destination.redacted_label();
    eprintln!(
        "event=local_ingress_destination host=<redacted> port=<redacted> destination_id={destination_id}"
    );
    let flow_key =
        MeshMultipathFlowKey::from_opaque_flow_bytes(destination.connect_addr().as_bytes())?;
    let peer = peer_pool.pop_wait_for_flow_key(flow_key)?;
    eprintln!("event=local_ingress_paired_with_peer");
    connect_local_client_via_peer(local, peer, destination)
}

pub fn handle_local_client_with_registrations_and_first_byte(
    mut local: TcpStream,
    registrations: &[TransitLaneRegistration],
    dispatcher: SharedTransitNextHopDispatcher,
    first_byte: u8,
) -> Result<(), String> {
    let destination = read_local_connect_destination(&mut local, first_byte)?;
    let destination_id = destination.redacted_label();
    eprintln!(
        "event=local_ingress_destination host=<redacted> port=<redacted> destination_id={destination_id}"
    );
    let flow_key =
        MeshMultipathFlowKey::from_opaque_flow_bytes(destination.connect_addr().as_bytes())?;
    let selection = select_carrier_lane_from_registrations(registrations, flow_key)?;
    let binding = selection
        .selected_binding
        .ok_or_else(|| "local ingress selected binding missing".to_string())?;
    let peer = dispatcher.pop_for(binding)?;
    eprintln!("event=local_ingress_paired_with_peer");
    connect_local_client_via_peer(local, peer, destination)
}

fn read_local_connect_destination(
    local: &mut TcpStream,
    first_byte: u8,
) -> Result<Destination, String> {
    if first_byte == LOCAL_MAGIC[0] {
        read_native_connect_destination(local, first_byte)
    } else {
        Err("unsupported local ingress protocol; expected CHIMERA-LOCAL/1".to_string())
    }
}

pub(crate) fn connect_local_client_via_peer(
    mut local: TcpStream,
    mut peer: SecurePeerStream,
    destination: Destination,
) -> Result<(), String> {
    let destination_id = destination.redacted_label();
    write_connect_message(&mut peer, &destination)?;
    eprintln!("event=peer_connect_request_sent request=<redacted> destination_id={destination_id}");
    require_peer_ack(&mut peer)?;
    eprintln!("event=peer_connect_ack_received destination_id={destination_id}");
    local
        .write_all(b"OK\n")
        .map_err(|error| format!("write native local ack failed: {error}"))?;
    pipe_plain_with_secure_peer(local, peer)
}

#[cfg(test)]
#[path = "modes_tests/local_egress.rs"]
mod local_egress_tests;
