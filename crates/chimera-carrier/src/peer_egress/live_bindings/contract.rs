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
    if document.mesh_path_plan()?.is_none() {
        return Err(
            "live sealed transit lane document requires a mesh plan snapshot when bound transit is enabled"
                .to_string(),
        );
    }
    Ok(())
}
