use super::MeshMultipathRebuildDirtyScope;

pub(in crate::runtime) fn validate_rebuild_reason(reason: &str) -> Result<(), String> {
    if reason.is_empty() {
        return Err("multipath rebuild reason is empty".to_string());
    }
    if reason.trim() != reason {
        return Err("multipath rebuild reason must not contain surrounding whitespace".to_string());
    }
    if reason.len() > 64 {
        return Err("multipath rebuild reason is too long".to_string());
    }
    if !reason
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err("multipath rebuild reason must be a lowercase enum label".to_string());
    }
    if !allowed_rebuild_reason(reason) {
        return Err("multipath rebuild reason is not allowlisted".to_string());
    }
    Ok(())
}

pub(super) fn validate_dirty_scope(
    dirty_scope: MeshMultipathRebuildDirtyScope,
    affected_peer_count: usize,
) -> Result<(), String> {
    match dirty_scope {
        MeshMultipathRebuildDirtyScope::Unknown => {
            if affected_peer_count == 0 {
                Ok(())
            } else {
                Err(
                    "multipath rebuild dirty scope unknown must use affected_peer_count=0"
                        .to_string(),
                )
            }
        }
        MeshMultipathRebuildDirtyScope::PeerSet => {
            if affected_peer_count == 0 {
                Err(
                    "multipath rebuild peer-set dirty scope requires affected_peer_count > 0"
                        .to_string(),
                )
            } else {
                Ok(())
            }
        }
    }
}

fn allowed_rebuild_reason(reason: &str) -> bool {
    matches!(
        reason,
        "active_lanes_below_plan"
            | "active_binding_capacity_missing"
            | "active_binding_capacity_over_budget"
            | "active_binding_missing"
            | "capacity_pressure"
            | "capacity_overflow"
            | "demand_rebuild_recommended"
            | "duplicate_active_lane"
            | "published_endpoint_changed"
            | "peer_health_changed"
            | "peer_performance_changed"
            | "peer_table_changed"
            | "local_reserve_invalid"
            | "route_binding_mismatch"
            | "transit_payload_policy_not_opaque"
            | "urgent_failover"
            | "weighted_selection_no_match"
    )
}
