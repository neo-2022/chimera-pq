use std::net::Shutdown;

use chimera_mesh::MeshMultipathFlowKey;

use crate::peer_egress::aggregate_ingress::{
    AggregatePeerIngressOutcome, SharedAggregateTransitIngressRegistry,
    accept_peer_aggregate_ingress_shard,
};
use crate::peer_egress::aggregate_wire::AggregateTransitShardFrame;
use crate::peer_egress::lane_binding::TransitLaneDocument;
use crate::peer_egress::net::tune_tcp;
use crate::peer_egress::pool::SharedPeerPool;
use crate::peer_egress::transit::{
    PeerTransitPolicy, TransitRelayFrame, pop_unbound_pool_transit_next_hop,
};
use crate::peer_egress::transit_dispatch::SharedTransitNextHopDispatcher;
use crate::peer_egress::transit_guard::{
    TransitRelayGuard, TransitRelayLimits, apply_transit_stream_limits,
};
use crate::peer_egress::transit_lane_selection::pop_planned_transit_dispatch_next_hop;
use crate::peer_egress::wire::write_sealed_transit_message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AggregatePeerIngressResult {
    Pending,
    Forwarded,
}

pub(crate) fn handle_aggregate_peer_ingress_shard(
    shard: AggregateTransitShardFrame,
    aggregate_ingress: Option<SharedAggregateTransitIngressRegistry>,
    policy: PeerTransitPolicy,
    next_hops: Option<SharedPeerPool>,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    lane_document: Option<&TransitLaneDocument>,
    limits: TransitRelayLimits,
) -> Result<AggregatePeerIngressResult, String> {
    limits.validate()?;
    let registry = aggregate_ingress
        .ok_or_else(|| "aggregate peer ingress registry unavailable".to_string())?;
    match accept_peer_aggregate_ingress_shard(&registry, shard)? {
        AggregatePeerIngressOutcome::Pending => Ok(AggregatePeerIngressResult::Pending),
        AggregatePeerIngressOutcome::Complete(frame) => {
            forward_completed_aggregate_transit_frame(
                frame,
                policy,
                next_hops,
                dispatcher,
                lane_document,
                limits,
            )?;
            Ok(AggregatePeerIngressResult::Forwarded)
        }
    }
}

fn forward_completed_aggregate_transit_frame(
    frame: TransitRelayFrame,
    policy: PeerTransitPolicy,
    next_hops: Option<SharedPeerPool>,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    lane_document: Option<&TransitLaneDocument>,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    if let Some(document) = lane_document
        && !document.is_empty()
    {
        let plan = document.require_mesh_path_plan()?;
        let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(frame.sealed_bytes())?;
        let peer = pop_planned_transit_dispatch_next_hop(&plan, dispatcher, flow_key)?;
        return forward_peer_sealed_transit_frame_once(peer, frame, limits);
    }
    let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(frame.sealed_bytes())?;
    let peer = pop_unbound_pool_transit_next_hop(policy, next_hops, flow_key)?;
    forward_peer_sealed_transit_frame_once(peer, frame, limits)
}

fn forward_peer_sealed_transit_frame_once(
    mut peer: crate::peer_egress::protocol::SecurePeerStream,
    frame: TransitRelayFrame,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    limits.validate()?;
    tune_tcp(&peer.stream)?;
    apply_transit_stream_limits(&peer.stream, limits)?;
    let mut guard = TransitRelayGuard::new(limits);
    guard.record_frame(frame.sealed_bytes().len())?;
    eprintln!("event=weave_peer_aggregate_transit_frame_forwarded");
    write_sealed_transit_message(&mut peer, &frame)
        .map_err(|error| format!("write aggregate transit frame to next hop failed: {error}"))?;
    let _ = peer.stream.shutdown(Shutdown::Write);
    Ok(())
}
