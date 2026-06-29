use std::io::{Read, Write};
use std::net::TcpStream;

use crate::peer_egress::lane_binding::{TransitLaneDocument, TransitLaneRegistration};
use crate::peer_egress::live_lane_selection::{
    select_carrier_binding_from_multipath_schedule, select_carrier_binding_from_registrations,
};
use crate::peer_egress::options::LOCAL_MAGIC;
use crate::peer_egress::pool::SharedPeerPool;
use crate::peer_egress::protocol::{
    Destination, SecurePeerStream, read_native_connect_destination,
};
use crate::peer_egress::transit_dispatch::SharedTransitNextHopDispatcher;
use crate::peer_egress::wire::{PeerMessage, read_peer_message, write_connect_message};
use chimera_mesh::MeshMultipathFlowKey;

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
    match select_carrier_binding_from_multipath_schedule(&plan.multipath_schedule, flow_key) {
        Ok(binding) => {
            let peer = dispatcher.pop_for(binding)?;
            eprintln!("event=local_ingress_paired_with_peer");
            connect_local_client_via_peer(local, peer, destination)
        }
        Err(reason) => Err(format!("local ingress lane selection failed: {reason}")),
    }
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
