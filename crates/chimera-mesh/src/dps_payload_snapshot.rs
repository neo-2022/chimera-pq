use std::collections::BTreeSet;

use crate::multipath_model::MeshRouteBindingId;
use crate::policy::{
    ContinuityPolicy, MeshPathPolicy, MeshPathProfile, MeshTrafficHints, MultipathDemand,
    MultipathMode, ShadowSwitchMode, TrafficClass,
};
use crate::policy_parse::{
    parse_bool_field, parse_csv_unique, parse_csv_unique_normalized, parse_u8_field,
    parse_u16_csv_field, parse_u64_field, parse_usize_field,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MeshDpsPayloadSnapshot {
    mesh_field_count: usize,
    mesh_keys: BTreeSet<String>,
    mesh_policy_keys_fingerprint: String,
    traffic_hints: MeshTrafficHints,
    route_binding_id: Option<MeshRouteBindingId>,
    mesh_require_min_reliability_present: bool,
    mesh_max_load_score_present: bool,
    mesh_max_peers_present: bool,
    mesh_max_selected_per_region_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MeshDpsPayloadParse {
    pub(crate) policy: MeshPathPolicy,
    pub(crate) snapshot: MeshDpsPayloadSnapshot,
}

pub(crate) fn parse_mesh_dps_payload(payload: &str) -> Result<MeshDpsPayloadParse, String> {
    if payload.trim().is_empty() {
        return Err("mesh policy payload must include at least one mesh_* field".to_string());
    }

    let mut allowed_regions: Vec<String> = Vec::new();
    let mut blocked_node_ids: Vec<String> = Vec::new();
    let mut require_min_reliability: Option<u8> = None;
    let mut max_load_score: Option<u8> = None;
    let mut max_peers: Option<usize> = None;
    let mut prefer_region_diversity: Option<bool> = None;
    let mut max_selected_per_region: Option<usize> = None;
    let mut min_distinct_regions: Option<usize> = None;
    let mut path_profile_override: Option<crate::policy::MeshPathProfile> = None;
    let mut connect_fallback_ports: Option<Vec<u16>> = None;
    let mut traffic_class: Option<TrafficClass> = None;
    let mut multipath_mode: Option<MultipathMode> = None;
    let mut multipath_demand: Option<MultipathDemand> = None;
    let mut continuity_policy: Option<ContinuityPolicy> = None;
    let mut route_binding_id: Option<MeshRouteBindingId> = None;
    let mut mesh_require_min_reliability_present = false;
    let mut mesh_max_load_score_present = false;
    let mut mesh_max_peers_present = false;
    let mut mesh_max_selected_per_region_present = false;
    let mut mesh_field_count = 0usize;
    let mut mesh_keys = BTreeSet::new();

    for segment in payload.split(';') {
        let part = segment.trim();
        if part.is_empty() {
            continue;
        }
        let Some((key_raw, value_raw)) = part.split_once('=') else {
            return Err("mesh policy payload field is malformed".to_string());
        };
        let key = key_raw.trim();
        let key_norm = key.to_ascii_lowercase();
        let value = value_raw.trim();
        if value.is_empty() {
            return Err(format!("mesh policy payload field '{key}' is empty"));
        }
        if key_norm.starts_with("mesh_") {
            mesh_field_count = mesh_field_count.saturating_add(1);
            if !mesh_keys.insert(key_norm.clone()) {
                return Err(format!(
                    "mesh policy payload contains duplicate field '{key}'"
                ));
            }
        }

        match key_norm.as_str() {
            "mesh_allowed_regions" => {
                allowed_regions = parse_csv_unique_normalized(value)?;
            }
            "mesh_blocked_nodes" => {
                blocked_node_ids = parse_csv_unique(value)?;
            }
            "mesh_min_reliability" => {
                require_min_reliability = Some(parse_u8_field(value, key)?);
                mesh_require_min_reliability_present = true;
            }
            "mesh_max_load" => {
                max_load_score = Some(parse_u8_field(value, key)?);
                mesh_max_load_score_present = true;
            }
            "mesh_max_peers" => {
                max_peers = Some(parse_usize_field(value, key)?);
                mesh_max_peers_present = true;
            }
            "mesh_prefer_region_diversity" => {
                prefer_region_diversity = Some(parse_bool_field(value, key)?);
            }
            "mesh_max_selected_per_region" => {
                max_selected_per_region = Some(parse_usize_field(value, key)?);
                mesh_max_selected_per_region_present = true;
            }
            "mesh_min_distinct_regions" => {
                min_distinct_regions = Some(parse_usize_field(value, key)?);
            }
            "mesh_path_profile" => {
                path_profile_override = Some(MeshPathProfile::parse(value)?);
            }
            "mesh_connect_fallback_ports" => {
                connect_fallback_ports = Some(parse_u16_csv_field(value, key)?);
            }
            "mesh_traffic_class" => {
                traffic_class = Some(TrafficClass::from_dps_value(value)?);
            }
            "mesh_multipath_mode" => {
                multipath_mode = Some(MultipathMode::from_dps_value(value)?);
            }
            "mesh_multipath_demand" => {
                multipath_demand = Some(MultipathDemand::from_dps_value(value)?);
            }
            "mesh_continuity_policy" => {
                continuity_policy = Some(ContinuityPolicy::from_dps_value(value)?);
            }
            "mesh_route_binding_id" => {
                let parsed = parse_u64_field(value, key)?;
                if parsed == 0 {
                    return Err("mesh policy route_binding_id must be nonzero".to_string());
                }
                route_binding_id = MeshRouteBindingId::new(parsed).ok();
            }
            _ => {
                if key_norm.starts_with("mesh_") {
                    return Err(format!(
                        "mesh policy payload contains unknown field '{key}'"
                    ));
                }
            }
        }
    }

    let mut policy = MeshPathPolicy::default_auto();
    policy.allowed_regions = allowed_regions;
    policy.blocked_node_ids = blocked_node_ids;
    policy.require_min_reliability =
        require_min_reliability.unwrap_or(policy.require_min_reliability);
    policy.max_load_score = max_load_score.unwrap_or(policy.max_load_score);
    policy.max_peers = max_peers.unwrap_or(policy.max_peers);
    policy.prefer_region_diversity =
        prefer_region_diversity.unwrap_or(policy.prefer_region_diversity);
    policy.max_selected_per_region =
        max_selected_per_region.unwrap_or(policy.max_selected_per_region);
    policy.min_distinct_regions = min_distinct_regions.unwrap_or(policy.min_distinct_regions);
    policy.path_profile_override = path_profile_override;
    policy.multipath_mode = multipath_mode;
    policy.multipath_demand = multipath_demand;
    policy.connect_fallback_ports = connect_fallback_ports.unwrap_or(policy.connect_fallback_ports);
    policy.validate()?;

    let snapshot = MeshDpsPayloadSnapshot {
        mesh_field_count,
        mesh_policy_keys_fingerprint: if mesh_keys.is_empty() {
            "none".to_string()
        } else {
            mesh_keys.iter().cloned().collect::<Vec<_>>().join(",")
        },
        mesh_keys,
        traffic_hints: build_traffic_hints(
            traffic_class,
            multipath_mode,
            multipath_demand,
            continuity_policy,
        ),
        route_binding_id,
        mesh_require_min_reliability_present,
        mesh_max_load_score_present,
        mesh_max_peers_present,
        mesh_max_selected_per_region_present,
    };

    Ok(MeshDpsPayloadParse { policy, snapshot })
}

impl MeshDpsPayloadSnapshot {
    pub(crate) fn mesh_field_count(&self) -> usize {
        self.mesh_field_count
    }

    pub(crate) fn mesh_policy_keys_fingerprint(&self) -> &str {
        &self.mesh_policy_keys_fingerprint
    }

    pub(crate) fn traffic_hints(&self) -> MeshTrafficHints {
        self.traffic_hints
    }

    pub(crate) fn route_binding_id(&self) -> Option<MeshRouteBindingId> {
        self.route_binding_id
    }

    pub(crate) fn has_mesh_policy_key(&self, expected_key: &str) -> bool {
        match expected_key.trim() {
            "mesh_require_min_reliability" => self.mesh_require_min_reliability_present,
            "mesh_max_load_score" => self.mesh_max_load_score_present,
            "mesh_max_peers" => self.mesh_max_peers_present,
            "mesh_max_selected_per_region" => self.mesh_max_selected_per_region_present,
            other => self.mesh_keys.contains(other),
        }
    }
}

fn build_traffic_hints(
    traffic_class: Option<TrafficClass>,
    multipath_mode: Option<MultipathMode>,
    multipath_demand: Option<MultipathDemand>,
    continuity_policy: Option<ContinuityPolicy>,
) -> MeshTrafficHints {
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

    MeshTrafficHints {
        traffic_class,
        multipath_mode,
        multipath_demand,
        continuity_policy,
        shadow_switch_mode,
    }
}
