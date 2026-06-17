use crate::policy::{
    ContinuityPolicy, MeshTrafficHints, MultipathDemand, MultipathMode, ShadowSwitchMode,
    TrafficClass,
};

pub fn traffic_class_from_dps_payload(payload: &str) -> Result<Option<TrafficClass>, String> {
    if payload.trim().is_empty() {
        return Ok(None);
    }
    let mut found: Option<TrafficClass> = None;
    for segment in payload.split(';') {
        let part = segment.trim();
        if part.is_empty() {
            continue;
        }
        let Some((key_raw, value_raw)) = part.split_once('=') else {
            return Err("mesh policy payload field is malformed".to_string());
        };
        let key = key_raw.trim().to_ascii_lowercase();
        if key != "mesh_traffic_class" {
            continue;
        }
        if found.is_some() {
            return Err(
                "mesh policy payload contains duplicate field 'mesh_traffic_class'".to_string(),
            );
        }
        let value = value_raw.trim();
        if value.is_empty() {
            return Err("mesh policy payload field 'mesh_traffic_class' is empty".to_string());
        }
        found = Some(TrafficClass::from_dps_value(value)?);
    }
    Ok(found)
}

pub fn multipath_mode_from_dps_payload(payload: &str) -> Result<Option<MultipathMode>, String> {
    if payload.trim().is_empty() {
        return Ok(None);
    }
    let mut found: Option<MultipathMode> = None;
    for segment in payload.split(';') {
        let part = segment.trim();
        if part.is_empty() {
            continue;
        }
        let Some((key_raw, value_raw)) = part.split_once('=') else {
            return Err("mesh policy payload field is malformed".to_string());
        };
        let key = key_raw.trim().to_ascii_lowercase();
        if key != "mesh_multipath_mode" {
            continue;
        }
        if found.is_some() {
            return Err(
                "mesh policy payload contains duplicate field 'mesh_multipath_mode'".to_string(),
            );
        }
        let value = value_raw.trim();
        if value.is_empty() {
            return Err("mesh policy payload field 'mesh_multipath_mode' is empty".to_string());
        }
        found = Some(MultipathMode::from_dps_value(value)?);
    }
    Ok(found)
}

pub fn multipath_demand_from_dps_payload(payload: &str) -> Result<Option<MultipathDemand>, String> {
    if payload.trim().is_empty() {
        return Ok(None);
    }
    let mut found: Option<MultipathDemand> = None;
    for segment in payload.split(';') {
        let part = segment.trim();
        if part.is_empty() {
            continue;
        }
        let Some((key_raw, value_raw)) = part.split_once('=') else {
            return Err("mesh policy payload field is malformed".to_string());
        };
        let key = key_raw.trim().to_ascii_lowercase();
        if key != "mesh_multipath_demand" {
            continue;
        }
        if found.is_some() {
            return Err(
                "mesh policy payload contains duplicate field 'mesh_multipath_demand'".to_string(),
            );
        }
        let value = value_raw.trim();
        if value.is_empty() {
            return Err("mesh policy payload field 'mesh_multipath_demand' is empty".to_string());
        }
        found = Some(MultipathDemand::from_dps_value(value)?);
    }
    Ok(found)
}

pub fn continuity_policy_from_dps_payload(
    payload: &str,
) -> Result<Option<ContinuityPolicy>, String> {
    if payload.trim().is_empty() {
        return Ok(None);
    }
    let mut found: Option<ContinuityPolicy> = None;
    for segment in payload.split(';') {
        let part = segment.trim();
        if part.is_empty() {
            continue;
        }
        let Some((key_raw, value_raw)) = part.split_once('=') else {
            return Err("mesh policy payload field is malformed".to_string());
        };
        let key = key_raw.trim().to_ascii_lowercase();
        if key != "mesh_continuity_policy" {
            continue;
        }
        if found.is_some() {
            return Err(
                "mesh policy payload contains duplicate field 'mesh_continuity_policy'".to_string(),
            );
        }
        let value = value_raw.trim();
        if value.is_empty() {
            return Err("mesh policy payload field 'mesh_continuity_policy' is empty".to_string());
        }
        found = Some(ContinuityPolicy::from_dps_value(value)?);
    }
    Ok(found)
}

pub fn traffic_hints_from_dps_payload(payload: &str) -> Result<MeshTrafficHints, String> {
    let traffic_class = traffic_class_from_dps_payload(payload)?;
    let multipath_mode = multipath_mode_from_dps_payload(payload)?;
    let multipath_demand = multipath_demand_from_dps_payload(payload)?;
    let continuity_policy = continuity_policy_from_dps_payload(payload)?;
    let shadow_switch_mode = match continuity_policy {
        Some(ContinuityPolicy::AllowFlowDrain) => ShadowSwitchMode::FlowDrain,
        Some(ContinuityPolicy::SameEgressOnly) => ShadowSwitchMode::TransportOnly,
        Some(ContinuityPolicy::AllowHardRebindOnly) => ShadowSwitchMode::HardRebindOnly,
        None => match multipath_mode {
            Some(MultipathMode::StandbyOnly) => ShadowSwitchMode::TransportOnly,
            Some(MultipathMode::FlowShard) => ShadowSwitchMode::FlowDrain,
            Some(MultipathMode::AggregateBuffered) => ShadowSwitchMode::TransportOnly,
            Some(MultipathMode::Off) | None => ShadowSwitchMode::Unknown,
        },
    };
    Ok(MeshTrafficHints {
        traffic_class,
        multipath_mode,
        multipath_demand,
        continuity_policy,
        shadow_switch_mode,
    })
}
