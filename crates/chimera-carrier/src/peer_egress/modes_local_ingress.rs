use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use crate::peer_egress::lane_binding::{
    TransitLaneDocument, TransitLaneRegistration, transit_path_binding_from_mesh_lane,
};
use crate::peer_egress::live_lane_selection::{
    select_carrier_binding_from_multipath_schedule, select_carrier_binding_from_registrations,
};
use crate::peer_egress::options::LOCAL_MAGIC;
use crate::peer_egress::pool::SharedPeerPool;
use crate::peer_egress::protocol::{
    Destination, SecurePeerStream, read_native_connect_destination, redacted_log_reason,
};
use crate::peer_egress::transit_binding::TransitPathBinding;
use crate::peer_egress::transit_dispatch::SharedTransitNextHopDispatcher;
use crate::peer_egress::wire::{PeerMessage, read_peer_message, write_connect_message};
use chimera_mesh::{MeshMultipathFlowKey, MeshMultipathLaneRole, MeshMultipathSchedule};

use super::tune_tcp;

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
        PeerMessage::AggregateSealedTransit(_) => {
            Err("peer returned unexpected aggregate transit frame".to_string())
        }
        PeerMessage::BoundSealedTransit(_) => {
            Err("peer returned unexpected bound transit frame".to_string())
        }
        PeerMessage::Announce(_) => Err("peer returned unexpected announce message".to_string()),
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

    let deadline = Instant::now()
        .checked_add(Duration::from_millis(peer_handshake_timeout_ms()))
        .ok_or_else(|| "peer handshake deadline overflow".to_string())?;
    let mut attempt: usize = 0;
    let mut prefer_flow_key = true;

    loop {
        attempt += 1;
        let now = Instant::now();
        if now >= deadline {
            return Err(format!(
                "peer handshake deadline reached after {} ms; last attempt={}",
                peer_handshake_timeout_ms(),
                attempt.saturating_sub(1)
            ));
        }
        let remaining = deadline.saturating_duration_since(now);
        eprintln!(
            "event=local_ingress_peer_select attempt={attempt} prefer_flow_key={prefer_flow_key} destination_id={destination_id}"
        );
        let maybe_peer = if prefer_flow_key {
            peer_pool.pop_wait_timeout_for_flow_key(flow_key, remaining)?
        } else {
            peer_pool.pop_wait_timeout(remaining)?
        };
        let Some(mut peer) = maybe_peer else {
            return Err(format!(
                "peer pool wait timed out after {} ms (attempt {attempt})",
                peer_handshake_timeout_ms()
            ));
        };
        match handshake_peer_for_destination(&mut peer, &destination) {
            Ok(()) => {
                eprintln!(
                    "event=local_ingress_paired_with_peer attempt={attempt} destination_id={destination_id}"
                );
                local
                    .write_all(b"OK\n")
                    .map_err(|error| format!("write native local ack failed: {error}"))?;
                return crate::peer_egress::net::pipe_plain_with_secure_peer(local, peer);
            }
            Err(error) => {
                peer.mark_dead();
                // The peer failed the handshake. Discard it by letting it drop
                // out of scope; the pool never sees this stream again, so the
                // same dead peer cannot be retried within this flow.
                eprintln!(
                    "event=local_ingress_peer_dead_discarded attempt={attempt} reason_class={} destination_id={destination_id}",
                    redacted_log_reason(&error)
                );
                // After a dead peer, stop pinning this request to the same
                // flow-key slot so the next iteration can pick any live peer.
                prefer_flow_key = false;
            }
        }
    }
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
    let binding = select_carrier_binding_from_registrations(registrations, flow_key)
        .map_err(|error| error.to_string())?;
    let peer = dispatcher.pop_for(binding)?;
    eprintln!("event=local_ingress_paired_with_peer");
    connect_local_client_via_peer(local, peer, destination)
}

pub fn handle_local_client_with_lane_document_and_first_byte(
    mut local: TcpStream,
    document: &TransitLaneDocument,
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
    let plan = document.require_mesh_path_plan_ref()?;
    let initial_binding =
        select_carrier_binding_from_multipath_schedule(&plan.multipath_schedule, flow_key)
            .map_err(|reason| format!("local ingress lane selection failed: {reason}"))?;
    let fallback_bindings =
        active_fallback_bindings(&plan.multipath_schedule, flow_key, initial_binding)
            .map_err(|error| format!("local ingress fallback binding enumeration failed: {error}"))?;

    let mut candidate_bindings: Vec<TransitPathBinding> =
        Vec::with_capacity(1 + fallback_bindings.len());
    candidate_bindings.push(initial_binding);
    candidate_bindings.extend(fallback_bindings);
    let mut binding_index = 0usize;

    let deadline = Instant::now()
        .checked_add(Duration::from_millis(peer_handshake_timeout_ms()))
        .ok_or_else(|| "peer handshake deadline overflow".to_string())?;
    let mut attempt: usize = 0;

    loop {
        attempt += 1;
        let now = Instant::now();
        if now >= deadline {
            return Err(format!(
                "lane document peer handshake deadline reached after {} ms; last attempt={}",
                peer_handshake_timeout_ms(),
                attempt.saturating_sub(1)
            ));
        }
        if binding_index >= candidate_bindings.len() {
            binding_index = 0;
        }
        let binding = candidate_bindings[binding_index];
        binding_index += 1;
        match dispatcher.pop_for(binding) {
            Ok(mut peer) => {
                match handshake_peer_for_destination(&mut peer, &destination) {
                    Ok(()) => {
                        eprintln!(
                            "event=local_ingress_paired_with_peer attempt={attempt} destination_id={destination_id}"
                        );
                        local
                            .write_all(b"OK\n")
                            .map_err(|error| format!("write native local ack failed: {error}"))?;
                        return crate::peer_egress::net::pipe_plain_with_secure_peer(local, peer);
                    }
                    Err(error) => {
                        peer.mark_dead();
                        eprintln!(
                            "event=local_ingress_peer_dead_discarded attempt={attempt} reason_class={} destination_id={destination_id}",
                            redacted_log_reason(&error)
                        );
                    }
                }
            }
            Err(error) => {
                eprintln!(
                    "event=local_ingress_lane_peer_unavailable attempt={attempt} reason_class={} destination_id={destination_id}",
                    redacted_log_reason(&error)
                );
            }
        }
        if Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

fn active_fallback_bindings(
    schedule: &MeshMultipathSchedule,
    _flow_key: MeshMultipathFlowKey,
    exclude: TransitPathBinding,
) -> Result<Vec<TransitPathBinding>, String> {
    let mut bindings: Vec<TransitPathBinding> = schedule
        .carrier_lane_bindings
        .iter()
        .filter(|lane| lane.role == MeshMultipathLaneRole::Active)
        .map(transit_path_binding_from_mesh_lane)
        .collect::<Result<Vec<_>, _>>()?;
    bindings.sort_by_key(|binding| binding.lane_id().get());
    bindings.retain(|&binding| binding != exclude);
    Ok(bindings)
}

pub fn read_local_connect_destination(
    local: &mut TcpStream,
    first_byte: u8,
) -> Result<Destination, String> {
    if first_byte == LOCAL_MAGIC[0] {
        read_native_connect_destination(local, first_byte)
    } else {
        Err("unsupported local ingress protocol; expected CHIMERA-LOCAL/1".to_string())
    }
}

fn peer_handshake_timeout_ms() -> u64 {
    const DEFAULT: u64 = 6_000;
    std::env::var("CHIMERA_PEER_EGRESS_HANDSHAKE_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT)
}

fn handshake_peer_for_destination(
    peer: &mut SecurePeerStream,
    destination: &Destination,
) -> Result<(), String> {
    write_connect_message(peer, destination)?;
    require_peer_ack(peer)
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
    crate::peer_egress::net::pipe_plain_with_secure_peer(local, peer)
}

#[cfg(test)]
#[path = "modes_local_ingress_tests.rs"]
mod tests;
