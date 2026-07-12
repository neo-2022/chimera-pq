use std::io::{Read, Write};
use std::net::TcpStream;

use crate::peer_egress::aggregate_ingress::SharedAggregateTransitIngressRegistry;
use crate::peer_egress::aggregate_peer_ingress::handle_aggregate_peer_ingress_shard;
use crate::peer_egress::lane_binding::TransitLaneDocument;
use crate::peer_egress::net::{connect_tcp, pipe_secure_peer_with_plain, tune_tcp};
use crate::peer_egress::options::LOCAL_MAGIC;
use crate::peer_egress::pool::SharedPeerPool;
use crate::peer_egress::protocol::{SecurePeerStream, read_native_connect_destination};
use crate::peer_egress::transit::{BoundPeerTransitPolicy, PeerTransitPolicy};
use crate::peer_egress::transit_dispatch::SharedTransitNextHopDispatcher;
use crate::peer_egress::transit_guard::TransitRelayLimits;
use crate::peer_egress::wire::{PeerMessage, read_peer_message, write_ack_ok};

pub fn handle_reverse_peer(
    peer: SecurePeerStream,
    policy: PeerTransitPolicy,
    bound_policy: BoundPeerTransitPolicy,
    pool: SharedPeerPool,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
) -> Result<(), String> {
    handle_reverse_peer_with_lane_document(peer, policy, bound_policy, pool, dispatcher, None)
}

pub fn handle_reverse_peer_with_lane_document(
    peer: SecurePeerStream,
    policy: PeerTransitPolicy,
    bound_policy: BoundPeerTransitPolicy,
    pool: SharedPeerPool,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    lane_document: Option<&TransitLaneDocument>,
) -> Result<(), String> {
    handle_reverse_peer_with_lane_document_and_aggregate_ingress(
        peer,
        policy,
        bound_policy,
        pool,
        dispatcher,
        lane_document,
        None,
    )
}

pub(super) struct ReversePeerTransitContext<'a> {
    pool: SharedPeerPool,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    lane_document: Option<&'a TransitLaneDocument>,
    aggregate_ingress: Option<SharedAggregateTransitIngressRegistry>,
    limits: TransitRelayLimits,
}

impl<'a> ReversePeerTransitContext<'a> {
    pub(super) fn new(
        pool: SharedPeerPool,
        dispatcher: Option<SharedTransitNextHopDispatcher>,
        lane_document: Option<&'a TransitLaneDocument>,
        aggregate_ingress: Option<SharedAggregateTransitIngressRegistry>,
        limits: TransitRelayLimits,
    ) -> Self {
        Self {
            pool,
            dispatcher,
            lane_document,
            aggregate_ingress,
            limits,
        }
    }
}

pub(crate) fn handle_reverse_peer_with_lane_document_and_aggregate_ingress(
    peer: SecurePeerStream,
    policy: PeerTransitPolicy,
    bound_policy: BoundPeerTransitPolicy,
    pool: SharedPeerPool,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    lane_document: Option<&TransitLaneDocument>,
    aggregate_ingress: Option<SharedAggregateTransitIngressRegistry>,
) -> Result<(), String> {
    handle_reverse_peer_with_context(
        peer,
        policy,
        bound_policy,
        ReversePeerTransitContext::new(
            pool,
            dispatcher,
            lane_document,
            aggregate_ingress,
            TransitRelayLimits::default(),
        ),
    )
}

pub(super) fn handle_reverse_peer_with_context(
    mut peer: SecurePeerStream,
    policy: PeerTransitPolicy,
    bound_policy: BoundPeerTransitPolicy,
    ctx: ReversePeerTransitContext<'_>,
) -> Result<(), String> {
    ctx.limits.validate()?;
    let previous_read_timeout = peer
        .stream
        .read_timeout()
        .map_err(|error| format!("read reverse peer timeout failed: {error}"))?;
    peer.stream
        .set_read_timeout(Some(ctx.limits.idle_timeout()))
        .map_err(|error| format!("set reverse peer read timeout failed: {error}"))?;
    let first_message = read_peer_message(&mut peer, 512)?;
    let destination = match first_message {
        PeerMessage::Connect(destination) => {
            peer.stream
                .set_read_timeout(previous_read_timeout)
                .map_err(|error| format!("restore reverse peer read timeout failed: {error}"))?;
            destination
        }
        PeerMessage::SealedTransit(frame) => {
            return super::forward_peer_sealed_transit_with_document_or_pool_and_limits(
                peer,
                policy,
                Some(ctx.pool),
                ctx.dispatcher,
                ctx.lane_document,
                frame,
                ctx.limits,
            );
        }
        PeerMessage::BoundSealedTransit(frame) => {
            return crate::peer_egress::transit::forward_bound_peer_sealed_transit_to_next_hop_with_limits(
                peer,
                bound_policy,
                ctx.dispatcher,
                frame,
                ctx.limits,
            );
        }
        PeerMessage::AggregateSealedTransit(shard) => {
            handle_aggregate_peer_ingress_shard(
                shard,
                ctx.aggregate_ingress,
                policy,
                Some(ctx.pool),
                ctx.dispatcher,
                ctx.lane_document,
                ctx.limits,
            )?;
            return Ok(());
        }
        PeerMessage::AckOk => return Err("unexpected peer ack before request".to_string()),
        PeerMessage::Announce(_) => return Err("unexpected peer announce message".to_string()),
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
