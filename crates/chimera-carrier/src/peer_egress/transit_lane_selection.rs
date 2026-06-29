use chimera_mesh::{MeshMultipathFlowKey, MeshMultipathSchedule};

use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::live_lane_selection::{
    select_carrier_binding_from_multipath_schedule, select_carrier_binding_from_registrations,
};
use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::transit_dispatch::SharedTransitNextHopDispatcher;

pub(crate) fn pop_scheduled_transit_dispatch_next_hop(
    schedule: &MeshMultipathSchedule,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    flow_key: MeshMultipathFlowKey,
) -> Result<SecurePeerStream, String> {
    let binding = select_carrier_binding_from_multipath_schedule(schedule, flow_key)
        .map_err(|reason| format!("sealed transit lane selection failed: {reason}"))?;
    pop_dispatch_next_hop_for_binding(dispatcher, binding)
}

fn pop_dispatch_next_hop_for_binding(
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    binding: crate::peer_egress::transit_binding::TransitPathBinding,
) -> Result<SecurePeerStream, String> {
    let dispatcher = dispatcher
        .ok_or_else(|| "sealed transit path binding dispatcher unavailable".to_string())?;
    dispatcher.pop_for(binding)
}

pub(crate) fn pop_registered_transit_next_hop(
    registrations: &[TransitLaneRegistration],
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    flow_key: MeshMultipathFlowKey,
) -> Result<SecurePeerStream, String> {
    if registrations.is_empty() {
        return Err("sealed transit lane registrations unavailable".to_string());
    }
    let binding = select_carrier_binding_from_registrations(registrations, flow_key)
        .map_err(|error| format!("sealed transit lane selection failed: {error}"))?;
    let dispatcher = dispatcher
        .ok_or_else(|| "sealed transit path binding dispatcher unavailable".to_string())?;
    dispatcher.pop_for(binding)
}
