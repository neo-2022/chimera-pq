use std::io::Read;
use std::net::{Shutdown, TcpStream};
use std::thread;

use chimera_mesh::{
    MeshMultipathFlowKey, MeshPathPlan, WeaveSealedTransitFrame, forward_weave_transit_frame,
    validate_weave_sealed_transit_frame,
};

use crate::peer_egress::bound_transit::forward_bound_peer_transit_pair;
use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::net::tune_tcp;
use crate::peer_egress::pool::{SharedPeerPool, UniquePeerPop};
use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::secure_halves::{
    SecurePeerReader, SecurePeerWriter, split_secure_peer_stream,
};
use crate::peer_egress::transit_binding::BoundTransitRelayFrame;
use crate::peer_egress::transit_dispatch::SharedTransitNextHopDispatcher;
use crate::peer_egress::transit_lane_selection::{
    pop_planned_transit_dispatch_next_hop, pop_registered_transit_next_hop,
};
use crate::peer_egress::wire::{
    PeerMessage, write_bound_sealed_transit_message, write_sealed_transit_message,
};

const TRANSIT_FRAME_HEADER_REST_LEN: usize = 13;
const TRANSIT_FRAME_HEADER_LEN: usize = 1 + TRANSIT_FRAME_HEADER_REST_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerTransitPolicy {
    DenyPoolNextHop,
    AllowPoolNextHop,
}

impl PeerTransitPolicy {
    pub fn from_bool(allowed: bool) -> Self {
        if allowed {
            Self::AllowPoolNextHop
        } else {
            Self::DenyPoolNextHop
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundPeerTransitPolicy {
    DenyBoundNextHop,
    AllowBoundNextHop,
}

impl BoundPeerTransitPolicy {
    pub fn from_bool(allowed: bool) -> Self {
        if allowed {
            Self::AllowBoundNextHop
        } else {
            Self::DenyBoundNextHop
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitRelayFrame {
    frame: WeaveSealedTransitFrame,
}

impl TransitRelayFrame {
    pub fn kind(&self) -> chimera_session::FrameKind {
        self.frame.kind()
    }

    pub fn packet_number(&self) -> u64 {
        self.frame.packet_number()
    }

    pub fn payload_len(&self) -> usize {
        self.frame.payload_len()
    }

    pub fn sealed_bytes(&self) -> &[u8] {
        self.frame.sealed_bytes()
    }
}

pub fn validate_transit_relay_frame(input: &[u8]) -> Result<TransitRelayFrame, String> {
    validate_weave_sealed_transit_frame(input)
        .map(|frame| TransitRelayFrame { frame })
        .map_err(|error| format!("validate transit frame failed: {error}"))
}

pub fn forward_transit_relay_frame(input: &[u8]) -> Result<Vec<u8>, String> {
    forward_weave_transit_frame(input)
        .map_err(|error| format!("forward transit frame failed: {error}"))
}

pub fn relay_local_sealed_transit(
    mut local: TcpStream,
    mut peer: SecurePeerStream,
    first_byte: u8,
) -> Result<(), String> {
    tune_tcp(&local)?;
    tune_tcp(&peer.stream)?;
    let mut next_first = Some(first_byte);
    loop {
        let byte = match next_first.take() {
            Some(byte) => byte,
            None => unreachable!("missing transit frame prefix"),
        };
        let transit = read_weave_sealed_transit_frame(&mut local, byte)?;
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
    mut local: TcpStream,
    peer_pool: SharedPeerPool,
    first_byte: u8,
) -> Result<(), String> {
    tune_tcp(&local)?;
    let first = read_weave_sealed_transit_frame(&mut local, first_byte)?;
    let peer =
        pop_unbound_pool_transit_next_hop(PeerTransitPolicy::AllowPoolNextHop, Some(peer_pool))?;
    relay_local_sealed_transit_after_first(local, peer, first)
}

pub fn relay_local_sealed_transit_to_planned_next_hop(
    mut local: TcpStream,
    plan: &MeshPathPlan,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first_byte: u8,
) -> Result<(), String> {
    tune_tcp(&local)?;
    let first = read_weave_sealed_transit_frame(&mut local, first_byte)?;
    let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(first.sealed_bytes())?;
    let peer = pop_planned_transit_dispatch_next_hop(plan, dispatcher, flow_key)?;
    relay_local_sealed_transit_after_first(local, peer, first)
}

pub fn relay_local_sealed_transit_with_registrations(
    mut local: TcpStream,
    registrations: &[TransitLaneRegistration],
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first_byte: u8,
) -> Result<(), String> {
    tune_tcp(&local)?;
    let first = read_weave_sealed_transit_frame(&mut local, first_byte)?;
    let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(first.sealed_bytes())?;
    let peer = pop_registered_transit_next_hop(registrations, dispatcher, flow_key)?;
    relay_local_sealed_transit_after_first(local, peer, first)
}

fn relay_local_sealed_transit_after_first(
    mut local: TcpStream,
    mut peer: SecurePeerStream,
    first: TransitRelayFrame,
) -> Result<(), String> {
    tune_tcp(&peer.stream)?;
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
    mut local: TcpStream,
    peer: SecurePeerStream,
    first_byte: u8,
) -> Result<(), String> {
    tune_tcp(&local)?;
    tune_tcp(&peer.stream)?;
    let first = read_weave_bound_sealed_transit_frame(&mut local, first_byte)?;
    relay_local_bound_sealed_transit_after_first(local, peer, first)
}

pub fn relay_local_bound_sealed_transit_to_next_hop(
    mut local: TcpStream,
    policy: BoundPeerTransitPolicy,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first_byte: u8,
) -> Result<(), String> {
    if policy != BoundPeerTransitPolicy::AllowBoundNextHop {
        return Err("sealed transit next hop denied by policy".to_string());
    }
    tune_tcp(&local)?;
    let first = read_weave_bound_sealed_transit_frame(&mut local, first_byte)?;
    let peer = pop_bound_transit_dispatch_next_hop(dispatcher, first.binding())?;
    relay_local_bound_sealed_transit_after_first(local, peer, first)
}

fn relay_local_bound_sealed_transit_after_first(
    mut local: TcpStream,
    mut peer: SecurePeerStream,
    first: BoundTransitRelayFrame,
) -> Result<(), String> {
    tune_tcp(&peer.stream)?;
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
        if transit.binding() != binding {
            let _ = peer.stream.shutdown(Shutdown::Write);
            return Err("bound transit stream binding changed midstream".to_string());
        }
        eprintln!("event=weave_bound_transit_frame_forwarded");
        write_bound_sealed_transit_message(&mut peer, &transit)
            .map_err(|error| format!("write bound transit frame to peer failed: {error}"))?;
    }
}

fn pop_bound_transit_dispatch_next_hop(
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    binding: crate::peer_egress::transit_binding::TransitPathBinding,
) -> Result<SecurePeerStream, String> {
    let dispatcher = dispatcher
        .ok_or_else(|| "sealed transit path binding dispatcher unavailable".to_string())?;
    dispatcher.pop_for(binding)
}

pub fn forward_peer_sealed_transit_to_next_hop(
    source: SecurePeerStream,
    policy: PeerTransitPolicy,
    next_hops: Option<SharedPeerPool>,
    first: TransitRelayFrame,
) -> Result<(), String> {
    let next_peer = pop_unbound_pool_transit_next_hop(policy, next_hops)?;
    forward_peer_sealed_transit_pair(source, next_peer, first)
}

pub fn forward_peer_sealed_transit_to_planned_next_hop(
    source: SecurePeerStream,
    plan: &MeshPathPlan,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first: TransitRelayFrame,
) -> Result<(), String> {
    let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(first.sealed_bytes())?;
    let next_peer = pop_planned_transit_dispatch_next_hop(plan, dispatcher, flow_key)?;
    forward_peer_sealed_transit_pair(source, next_peer, first)
}

pub fn forward_peer_sealed_transit_with_registrations(
    source: SecurePeerStream,
    registrations: &[TransitLaneRegistration],
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first: TransitRelayFrame,
) -> Result<(), String> {
    let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(first.sealed_bytes())?;
    let next_peer = pop_registered_transit_next_hop(registrations, dispatcher, flow_key)?;
    forward_peer_sealed_transit_pair(source, next_peer, first)
}

fn pop_unbound_pool_transit_next_hop(
    policy: PeerTransitPolicy,
    next_hops: Option<SharedPeerPool>,
) -> Result<SecurePeerStream, String> {
    if policy != PeerTransitPolicy::AllowPoolNextHop {
        return Err("sealed transit next hop denied by policy".to_string());
    }
    let pool = next_hops.ok_or_else(|| "sealed transit next hop unavailable".to_string())?;
    match pool.try_pop_unique()? {
        UniquePeerPop::Ready(peer) => Ok(peer),
        UniquePeerPop::Unavailable => Err("sealed transit next hop unavailable".to_string()),
        UniquePeerPop::Ambiguous => {
            Err("sealed transit next hop ambiguous without path binding".to_string())
        }
    }
}

pub fn forward_bound_peer_sealed_transit_to_next_hop(
    source: SecurePeerStream,
    policy: BoundPeerTransitPolicy,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first: BoundTransitRelayFrame,
) -> Result<(), String> {
    if policy != BoundPeerTransitPolicy::AllowBoundNextHop {
        return Err("sealed transit next hop denied by policy".to_string());
    }
    let next_peer = pop_bound_transit_dispatch_next_hop(dispatcher, first.binding())?;
    forward_bound_peer_transit_pair(source, next_peer, first)
}

fn forward_peer_sealed_transit_pair(
    source: SecurePeerStream,
    next_peer: SecurePeerStream,
    first: TransitRelayFrame,
) -> Result<(), String> {
    let (source_reader, source_writer) = split_secure_peer_stream(source)?;
    let (next_reader, next_writer) = split_secure_peer_stream(next_peer)?;
    let forward = thread::spawn(move || {
        pipe_sealed_transit_direction(source_reader, next_writer, Some(first), "source_to_next")
    });
    let reverse = thread::spawn(move || {
        pipe_sealed_transit_direction(next_reader, source_writer, None, "next_to_source")
    });
    let forward_result = forward
        .join()
        .map_err(|_| "sealed transit forward worker panicked".to_string())?;
    let reverse_result = reverse
        .join()
        .map_err(|_| "sealed transit reverse worker panicked".to_string())?;
    forward_result?;
    reverse_result?;
    Ok(())
}

fn pipe_sealed_transit_direction(
    mut reader: SecurePeerReader,
    mut writer: SecurePeerWriter,
    first: Option<TransitRelayFrame>,
    direction: &'static str,
) -> Result<(), String> {
    let result = (|| {
        let mut pending = first;
        loop {
            let frame = match pending.take() {
                Some(frame) => frame,
                None => match read_peer_message_from_reader(
                    &mut reader,
                    crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
                )? {
                    PeerMessage::SealedTransit(frame) => frame,
                    PeerMessage::Connect(_) => {
                        return Err("sealed transit stream received connect message".to_string());
                    }
                    PeerMessage::AckOk => {
                        return Err("sealed transit stream received ack".to_string());
                    }
                    PeerMessage::BoundSealedTransit(_) => {
                        return Err(
                            "sealed transit stream received nested bound transit frame".to_string()
                        );
                    }
                },
            };
            eprintln!("event=weave_peer_transit_frame_forwarded direction={direction}");
            let is_fin = frame.kind() == chimera_session::FrameKind::Fin;
            writer
                .write_secure_payload(frame.sealed_bytes())
                .map_err(|error| format!("write peer transit frame to next hop failed: {error}"))?;
            if is_fin {
                let _ = writer.stream.shutdown(Shutdown::Write);
                return Ok(());
            }
        }
    })();
    if result.is_err() {
        reader.shutdown();
        writer.shutdown();
    }
    result
}

fn read_peer_message_from_reader(
    reader: &mut SecurePeerReader,
    max_line_len: usize,
) -> Result<PeerMessage, String> {
    let payload = reader.read_secure_payload()?;
    crate::peer_egress::wire::parse_peer_payload(payload, max_line_len)
}

pub fn read_weave_sealed_transit_frame<R: Read>(
    stream: &mut R,
    first_byte: u8,
) -> Result<TransitRelayFrame, String> {
    if first_byte != chimera_session::FRAME_VERSION {
        return Err("transit frame version invalid".to_string());
    }
    let mut header = [0_u8; TRANSIT_FRAME_HEADER_REST_LEN];
    stream
        .read_exact(&mut header)
        .map_err(|error| format!("read transit frame header failed: {error}"))?;
    let payload_len = u32::from_be_bytes(
        header[9..13]
            .try_into()
            .map_err(|_| "invalid transit frame length field".to_string())?,
    ) as usize;
    if payload_len > chimera_session::MAX_PAYLOAD_LEN {
        return Err("transit frame payload too large".to_string());
    }
    let mut frame = Vec::with_capacity(TRANSIT_FRAME_HEADER_LEN + payload_len);
    frame.push(first_byte);
    frame.extend_from_slice(&header);
    let mut payload = vec![0_u8; payload_len];
    if payload_len > 0 {
        stream
            .read_exact(&mut payload)
            .map_err(|error| format!("read transit frame payload failed: {error}"))?;
    }
    frame.extend_from_slice(&payload);
    validate_transit_relay_frame(&frame)
}

pub fn read_weave_bound_sealed_transit_frame<R: Read>(
    stream: &mut R,
    first_byte: u8,
) -> Result<BoundTransitRelayFrame, String> {
    if first_byte != crate::peer_egress::transit_binding::BOUND_TRANSIT_MAGIC {
        return Err("bound transit frame magic invalid".to_string());
    }
    let mut header = [0_u8; crate::peer_egress::transit_binding::BOUND_TRANSIT_HEADER_REST_LEN];
    stream
        .read_exact(&mut header)
        .map_err(|error| format!("read bound transit frame header failed: {error}"))?;
    let mut nested_first = [0_u8; 1];
    stream
        .read_exact(&mut nested_first)
        .map_err(|error| format!("read nested sealed transit frame version failed: {error}"))?;
    let sealed = read_weave_sealed_transit_frame(stream, nested_first[0])?;
    let mut frame = Vec::with_capacity(
        1 + crate::peer_egress::transit_binding::BOUND_TRANSIT_HEADER_REST_LEN
            + sealed.sealed_bytes().len(),
    );
    frame.push(first_byte);
    frame.extend_from_slice(&header);
    frame.extend_from_slice(sealed.sealed_bytes());
    crate::peer_egress::transit_binding::validate_bound_transit_relay_frame(&frame)
}

#[cfg(test)]
#[path = "transit_tests/mod.rs"]
mod transit_tests;
