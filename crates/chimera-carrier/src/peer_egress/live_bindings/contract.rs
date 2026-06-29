use crate::peer_egress::lane_binding::TransitLaneDocument;
use crate::peer_egress::options::Options;

pub(super) fn validate_live_transit_lane_document_contract(
    options: &Options,
    document: &TransitLaneDocument,
) -> Result<(), String> {
    if document.is_empty() {
        return Ok(());
    }
    if !options.allow_bound_transit {
        return Err(
            "live sealed transit lane document requires allow_bound_transit=true".to_string(),
        );
    }
    if document.registrations().is_empty() {
        return Err("live sealed transit lane document requires registrations".to_string());
    }
    let Some(plan) = document.mesh_path_plan_ref() else {
        return Err(
            "live sealed transit lane document requires a mesh plan snapshot when bound transit is enabled"
                .to_string(),
        );
    };
    if plan.multipath_schedule.execution_status != "carrier_lane_binding_contract_ready" {
        return Err(
            "live sealed transit lane document requires carrier binding contract ready".to_string(),
        );
    }
    if plan.multipath_schedule.transit_payload_policy != "sealed_opaque_only" {
        return Err(
            "live sealed transit lane document requires opaque transit payload policy".to_string(),
        );
    }
    if plan.multipath_schedule.local_traffic_reserve_pct == 0 {
        return Err("live sealed transit lane document requires local reserve".to_string());
    }
    if plan.multipath_schedule.active_lane_count == 0
        || plan.multipath_schedule.carrier_lane_bindings.is_empty()
    {
        return Err(
            "live sealed transit lane document requires active carrier bindings".to_string(),
        );
    }
    Ok(())
}
