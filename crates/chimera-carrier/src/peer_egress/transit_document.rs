use crate::peer_egress::lane_binding::TransitLaneDocument;
use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::transit::{
    TransitRelayFrame, forward_peer_sealed_transit_to_planned_next_hop,
};
use crate::peer_egress::transit_dispatch::SharedTransitNextHopDispatcher;

pub(crate) fn forward_peer_sealed_transit_with_lane_document(
    source: SecurePeerStream,
    document: &TransitLaneDocument,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    first: TransitRelayFrame,
) -> Result<(), String> {
    let plan = document.require_mesh_path_plan()?;
    forward_peer_sealed_transit_to_planned_next_hop(source, &plan, dispatcher, first)
}
