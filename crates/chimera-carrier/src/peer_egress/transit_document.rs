use crate::peer_egress::lane_binding::TransitLaneDocument;
use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::transit::{
    TransitRelayFrame, forward_peer_sealed_transit_to_scheduled_next_hop_with_limits,
};
use crate::peer_egress::transit_dispatch::SharedTransitNextHopDispatcher;
use crate::peer_egress::transit_guard::TransitRelayLimits;

#[cfg(test)]
pub(crate) fn forward_peer_sealed_transit_with_lane_document(
    source: SecurePeerStream,
    document: &TransitLaneDocument,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first: TransitRelayFrame,
) -> Result<(), String> {
    forward_peer_sealed_transit_with_lane_document_and_limits(
        source,
        document,
        dispatcher,
        first,
        TransitRelayLimits::default(),
    )
}

pub(crate) fn forward_peer_sealed_transit_with_lane_document_and_limits(
    source: SecurePeerStream,
    document: &TransitLaneDocument,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first: TransitRelayFrame,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    let plan = document.require_mesh_path_plan_ref()?;
    forward_peer_sealed_transit_to_scheduled_next_hop_with_limits(
        source,
        &plan.multipath_schedule,
        dispatcher,
        first,
        limits,
    )
}
