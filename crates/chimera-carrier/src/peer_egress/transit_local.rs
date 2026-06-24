use std::io::Read;
use std::net::{Shutdown, TcpStream};

use chimera_mesh::{MeshMultipathFlowKey, MeshPathPlan};

use crate::peer_egress::lane_binding::{TransitLaneDocument, TransitLaneRegistration};
use crate::peer_egress::net::tune_tcp;
use crate::peer_egress::pool::SharedPeerPool;
use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::transit::{
    BoundPeerTransitPolicy, PeerTransitPolicy, TransitRelayFrame,
    pop_bound_transit_dispatch_next_hop, pop_unbound_pool_transit_next_hop,
    read_weave_bound_sealed_transit_frame, read_weave_sealed_transit_frame,
};
use crate::peer_egress::transit_binding::{BOUND_TRANSIT_HEADER_LEN, BoundTransitRelayFrame};
use crate::peer_egress::transit_dispatch::SharedTransitNextHopDispatcher;
use crate::peer_egress::transit_guard::{
    TransitRelayGuard, TransitRelayLimits, apply_transit_stream_limits,
};
use crate::peer_egress::transit_lane_selection::{
    pop_planned_transit_dispatch_next_hop, pop_registered_transit_next_hop,
};
use crate::peer_egress::wire::{write_bound_sealed_transit_message, write_sealed_transit_message};

pub fn relay_local_sealed_transit(
    local: TcpStream,
    peer: SecurePeerStream,
    first_byte: u8,
) -> Result<(), String> {
    relay_local_sealed_transit_with_limits(local, peer, first_byte, TransitRelayLimits::default())
}

pub fn relay_local_sealed_transit_with_limits(
    mut local: TcpStream,
    mut peer: SecurePeerStream,
    first_byte: u8,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    limits.validate()?;
    tune_tcp(&local)?;
    tune_tcp(&peer.stream)?;
    apply_transit_stream_limits(&local, limits)?;
    apply_transit_stream_limits(&peer.stream, limits)?;
    let mut next_first = Some(first_byte);
    let mut guard = TransitRelayGuard::new(limits);
    loop {
        let byte = match next_first.take() {
            Some(byte) => byte,
            None => unreachable!("missing transit frame prefix"),
        };
        let transit = read_weave_sealed_transit_frame(&mut local, byte)?;
        guard.record_frame(transit.sealed_bytes().len())?;
        eprintln!("event=weave_transit_frame_forwarded");
        write_sealed_transit_message(&mut peer, &transit)
            .map_err(|error| format!("write transit frame to peer failed: {error}"))?;
        let mut first = [0_u8; 1];
        match local.read(&mut first) {
            Ok(0) => {
                let _ = peer.stream.shutdown(Shutdown::Write);
                return Ok(());
            }
            Ok(1) => next_first = Some(first[0]),
            Ok(_) => unreachable!("single-byte read returned more than one byte"),
            Err(error) => return Err(format!("read transit frame prefix failed: {error}")),
        }
    }
}

pub fn relay_local_sealed_transit_to_next_hop(
    local: TcpStream,
    policy: PeerTransitPolicy,
    peer_pool: SharedPeerPool,
    first_byte: u8,
) -> Result<(), String> {
    relay_local_sealed_transit_to_next_hop_with_limits(
        local,
        policy,
        peer_pool,
        first_byte,
        TransitRelayLimits::default(),
    )
}

pub fn relay_local_sealed_transit_to_next_hop_with_limits(
    mut local: TcpStream,
    policy: PeerTransitPolicy,
    peer_pool: SharedPeerPool,
    first_byte: u8,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    limits.validate()?;
    tune_tcp(&local)?;
    apply_transit_stream_limits(&local, limits)?;
    let first = read_weave_sealed_transit_frame(&mut local, first_byte)?;
    let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(first.sealed_bytes())?;
    let peer = pop_unbound_pool_transit_next_hop(policy, Some(peer_pool), flow_key)?;
    relay_local_sealed_transit_after_first_with_limits(local, peer, first, limits)
}

pub fn relay_local_sealed_transit_to_planned_next_hop(
    local: TcpStream,
    plan: &MeshPathPlan,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first_byte: u8,
) -> Result<(), String> {
    relay_local_sealed_transit_to_planned_next_hop_with_limits(
        local,
        plan,
        dispatcher,
        first_byte,
        TransitRelayLimits::default(),
    )
}

pub fn relay_local_sealed_transit_to_planned_next_hop_with_limits(
    mut local: TcpStream,
    plan: &MeshPathPlan,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first_byte: u8,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    limits.validate()?;
    tune_tcp(&local)?;
    apply_transit_stream_limits(&local, limits)?;
    let first = read_weave_sealed_transit_frame(&mut local, first_byte)?;
    let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(first.sealed_bytes())?;
    let peer = pop_planned_transit_dispatch_next_hop(plan, dispatcher, flow_key)?;
    relay_local_sealed_transit_after_first_with_limits(local, peer, first, limits)
}

pub fn relay_local_sealed_transit_with_registrations(
    local: TcpStream,
    registrations: &[TransitLaneRegistration],
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first_byte: u8,
) -> Result<(), String> {
    relay_local_sealed_transit_with_registrations_with_limits(
        local,
        registrations,
        dispatcher,
        first_byte,
        TransitRelayLimits::default(),
    )
}

pub fn relay_local_sealed_transit_with_registrations_with_limits(
    mut local: TcpStream,
    registrations: &[TransitLaneRegistration],
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first_byte: u8,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    limits.validate()?;
    tune_tcp(&local)?;
    apply_transit_stream_limits(&local, limits)?;
    let first = read_weave_sealed_transit_frame(&mut local, first_byte)?;
    let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(first.sealed_bytes())?;
    let peer = pop_registered_transit_next_hop(registrations, dispatcher, flow_key)?;
    relay_local_sealed_transit_after_first_with_limits(local, peer, first, limits)
}

pub fn relay_local_sealed_transit_with_lane_document_and_first_byte(
    local: TcpStream,
    document: &TransitLaneDocument,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first_byte: u8,
) -> Result<(), String> {
    relay_local_sealed_transit_with_lane_document_and_first_byte_with_limits(
        local,
        document,
        dispatcher,
        first_byte,
        TransitRelayLimits::default(),
    )
}

pub fn relay_local_sealed_transit_with_lane_document_and_first_byte_with_limits(
    local: TcpStream,
    document: &TransitLaneDocument,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first_byte: u8,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    let plan = document.require_mesh_path_plan()?;
    relay_local_sealed_transit_to_planned_next_hop_with_limits(
        local, &plan, dispatcher, first_byte, limits,
    )
}

fn relay_local_sealed_transit_after_first_with_limits(
    mut local: TcpStream,
    mut peer: SecurePeerStream,
    first: TransitRelayFrame,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    limits.validate()?;
    tune_tcp(&peer.stream)?;
    apply_transit_stream_limits(&local, limits)?;
    apply_transit_stream_limits(&peer.stream, limits)?;
    let mut guard = TransitRelayGuard::new(limits);
    let mut pending = Some(first);
    loop {
        let transit = match pending.take() {
            Some(frame) => frame,
            None => {
                let mut first = [0_u8; 1];
                match local.read(&mut first) {
                    Ok(0) => {
                        let _ = peer.stream.shutdown(Shutdown::Write);
                        return Ok(());
                    }
                    Ok(1) => read_weave_sealed_transit_frame(&mut local, first[0])?,
                    Ok(_) => unreachable!("single-byte read returned more than one byte"),
                    Err(error) => {
                        return Err(format!("read transit frame prefix failed: {error}"));
                    }
                }
            }
        };
        guard.record_frame(transit.sealed_bytes().len())?;
        eprintln!("event=weave_transit_frame_forwarded");
        write_sealed_transit_message(&mut peer, &transit)
            .map_err(|error| format!("write transit frame to peer failed: {error}"))?;
        let mut first = [0_u8; 1];
        match local.read(&mut first) {
            Ok(0) => {
                let _ = peer.stream.shutdown(Shutdown::Write);
                return Ok(());
            }
            Ok(1) => pending = Some(read_weave_sealed_transit_frame(&mut local, first[0])?),
            Ok(_) => unreachable!("single-byte read returned more than one byte"),
            Err(error) => return Err(format!("read transit frame prefix failed: {error}")),
        }
    }
}

pub fn relay_local_bound_sealed_transit(
    local: TcpStream,
    peer: SecurePeerStream,
    first_byte: u8,
) -> Result<(), String> {
    relay_local_bound_sealed_transit_with_limits(
        local,
        peer,
        first_byte,
        TransitRelayLimits::default(),
    )
}

pub fn relay_local_bound_sealed_transit_with_limits(
    mut local: TcpStream,
    peer: SecurePeerStream,
    first_byte: u8,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    limits.validate()?;
    tune_tcp(&local)?;
    tune_tcp(&peer.stream)?;
    apply_transit_stream_limits(&local, limits)?;
    apply_transit_stream_limits(&peer.stream, limits)?;
    let first = read_weave_bound_sealed_transit_frame(&mut local, first_byte)?;
    relay_local_bound_sealed_transit_after_first_with_limits(local, peer, first, limits)
}

pub fn relay_local_bound_sealed_transit_to_next_hop(
    local: TcpStream,
    policy: BoundPeerTransitPolicy,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first_byte: u8,
) -> Result<(), String> {
    relay_local_bound_sealed_transit_to_next_hop_with_limits(
        local,
        policy,
        dispatcher,
        first_byte,
        TransitRelayLimits::default(),
    )
}

pub fn relay_local_bound_sealed_transit_to_next_hop_with_limits(
    mut local: TcpStream,
    policy: BoundPeerTransitPolicy,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first_byte: u8,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    limits.validate()?;
    if policy != BoundPeerTransitPolicy::AllowBoundNextHop {
        return Err("sealed transit next hop denied by policy".to_string());
    }
    tune_tcp(&local)?;
    apply_transit_stream_limits(&local, limits)?;
    let first = read_weave_bound_sealed_transit_frame(&mut local, first_byte)?;
    let peer = pop_bound_transit_dispatch_next_hop(dispatcher, first.binding())?;
    relay_local_bound_sealed_transit_after_first_with_limits(local, peer, first, limits)
}

fn relay_local_bound_sealed_transit_after_first_with_limits(
    mut local: TcpStream,
    mut peer: SecurePeerStream,
    first: BoundTransitRelayFrame,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    limits.validate()?;
    tune_tcp(&peer.stream)?;
    apply_transit_stream_limits(&local, limits)?;
    apply_transit_stream_limits(&peer.stream, limits)?;
    let mut guard = TransitRelayGuard::new(limits);
    let binding = first.binding();
    let mut pending = Some(first);
    loop {
        let transit = match pending.take() {
            Some(frame) => frame,
            None => {
                let mut first = [0_u8; 1];
                match local.read(&mut first) {
                    Ok(0) => {
                        let _ = peer.stream.shutdown(Shutdown::Write);
                        return Ok(());
                    }
                    Ok(1) => read_weave_bound_sealed_transit_frame(&mut local, first[0])?,
                    Ok(_) => unreachable!("single-byte read returned more than one byte"),
                    Err(error) => {
                        return Err(format!("read bound transit frame prefix failed: {error}"));
                    }
                }
            }
        };
        guard.record_frame(
            transit
                .frame()
                .sealed_bytes()
                .len()
                .saturating_add(BOUND_TRANSIT_HEADER_LEN),
        )?;
        if transit.binding() != binding {
            let _ = peer.stream.shutdown(Shutdown::Write);
            return Err("bound transit stream binding changed midstream".to_string());
        }
        eprintln!("event=weave_bound_transit_frame_forwarded");
        write_bound_sealed_transit_message(&mut peer, &transit)
            .map_err(|error| format!("write bound transit frame to peer failed: {error}"))?;
    }
}
