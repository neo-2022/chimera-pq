use crate::peer_egress::lane_binding::TransitLaneDocument;
use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::transit::{
    TransitRelayFrame, forward_peer_sealed_transit_to_planned_next_hop,
    forward_peer_sealed_transit_with_registrations,
};
use crate::peer_egress::transit_dispatch::SharedTransitNextHopDispatcher;

pub(crate) fn forward_peer_sealed_transit_with_lane_document(
    source: SecurePeerStream,
    document: &TransitLaneDocument,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first: TransitRelayFrame,
) -> Result<(), String> {
    if let Some(plan) = document.mesh_path_plan()? {
        return forward_peer_sealed_transit_to_planned_next_hop(source, &plan, dispatcher, first);
    }
    forward_peer_sealed_transit_with_registrations(
        source,
        document.registrations(),
        dispatcher,
        first,
    )
}
