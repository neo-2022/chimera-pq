use std::collections::BTreeSet;

use super::policy_parse::{
    parse_bool_field, parse_csv_unique, parse_csv_unique_normalized, parse_i32_field,
    parse_u8_field, parse_u16_csv_field, parse_u64_field, parse_usize_field,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshPathProfile {
    Fast,
    Balanced,
    Resilient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficClass {
    ControlDns,
    WebInteractive,
    CdnStatic,
    BufferedStreaming,
    BulkTransfer,
    CloudSyncBackup,
    ArtifactDownload,
    GamingFps,
    RealtimeInteractive,
    Messaging,
    AuthSensitive,
    P2pRestricted,
    BackgroundTelemetry,
    ControlHealth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultipathMode {
    Off,
    StandbyOnly,
    FlowShard,
    AggregateBuffered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinuityPolicy {
    SameEgressOnly,
    AllowFlowDrain,
    AllowHardRebindOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowSwitchMode {
    TransportOnly,
    FlowDrain,
    HardRebindOnly,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshTrafficHints {
    pub traffic_class: Option<TrafficClass>,
    pub multipath_mode: Option<MultipathMode>,
    pub continuity_policy: Option<ContinuityPolicy>,
    pub shadow_switch_mode: ShadowSwitchMode,
}

impl MeshTrafficHints {
    pub fn has_any_hint(&self) -> bool {
        self.traffic_class.is_some()
            || self.multipath_mode.is_some()
            || self.continuity_policy.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrafficClassProfile {
    pub latency_p95_ms: f32,
    pub jitter_p95_ms: f32,
    pub loss_pct: f32,
    pub pri_warm_threshold: f32,
    pub pri_switch_threshold: f32,
}

impl TrafficClass {
    pub fn from_dps_value(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "control_dns" | "dns" => Ok(Self::ControlDns),
            "web_interactive" | "web" => Ok(Self::WebInteractive),
            "cdn_static" | "cdn" => Ok(Self::CdnStatic),
            "buffered_streaming" | "streaming_video" => Ok(Self::BufferedStreaming),
            "bulk_transfer" | "bulk_download" => Ok(Self::BulkTransfer),
            "cloud_sync_backup" => Ok(Self::CloudSyncBackup),
            "artifact_download" => Ok(Self::ArtifactDownload),
            "gaming_fps" => Ok(Self::GamingFps),
            "realtime_interactive" => Ok(Self::RealtimeInteractive),
            "messaging" => Ok(Self::Messaging),
            "auth_sensitive" => Ok(Self::AuthSensitive),
            "p2p_restricted" => Ok(Self::P2pRestricted),
            "background_telemetry" => Ok(Self::BackgroundTelemetry),
            "control_health" => Ok(Self::ControlHealth),
            _ => Err("mesh traffic_class must be one of: control_dns, web_interactive, cdn_static, buffered_streaming, bulk_transfer, cloud_sync_backup, artifact_download, realtime_interactive, gaming_fps, messaging, auth_sensitive, p2p_restricted, background_telemetry, control_health".to_string()),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ControlDns => "control_dns",
            Self::WebInteractive => "web_interactive",
            Self::CdnStatic => "cdn_static",
            Self::BufferedStreaming => "buffered_streaming",
            Self::BulkTransfer => "bulk_transfer",
            Self::CloudSyncBackup => "cloud_sync_backup",
            Self::ArtifactDownload => "artifact_download",
            Self::GamingFps => "gaming_fps",
            Self::RealtimeInteractive => "realtime_interactive",
            Self::Messaging => "messaging",
            Self::AuthSensitive => "auth_sensitive",
            Self::P2pRestricted => "p2p_restricted",
            Self::BackgroundTelemetry => "background_telemetry",
            Self::ControlHealth => "control_health",
        }
    }

    pub fn starter_profile(self) -> TrafficClassProfile {
        match self {
            Self::ControlDns => TrafficClassProfile {
                latency_p95_ms: 80.0,
                jitter_p95_ms: 8.0,
                loss_pct: 0.5,
                pri_warm_threshold: 0.60,
                pri_switch_threshold: 0.80,
            },
            Self::WebInteractive => TrafficClassProfile {
                latency_p95_ms: 120.0,
                jitter_p95_ms: 12.0,
                loss_pct: 1.0,
                pri_warm_threshold: 0.60,
                pri_switch_threshold: 0.85,
            },
            Self::CdnStatic => TrafficClassProfile {
                latency_p95_ms: 180.0,
                jitter_p95_ms: 16.0,
                loss_pct: 1.5,
                pri_warm_threshold: 0.60,
                pri_switch_threshold: 0.85,
            },
            Self::BufferedStreaming => TrafficClassProfile {
                latency_p95_ms: 220.0,
                jitter_p95_ms: 18.0,
                loss_pct: 1.2,
                pri_warm_threshold: 0.60,
                pri_switch_threshold: 0.85,
            },
            Self::BulkTransfer => TrafficClassProfile {
                latency_p95_ms: 500.0,
                jitter_p95_ms: 30.0,
                loss_pct: 3.0,
                pri_warm_threshold: 0.60,
                pri_switch_threshold: 0.85,
            },
            Self::CloudSyncBackup => TrafficClassProfile {
                latency_p95_ms: 450.0,
                jitter_p95_ms: 30.0,
                loss_pct: 2.5,
                pri_warm_threshold: 0.60,
                pri_switch_threshold: 0.85,
            },
            Self::ArtifactDownload => TrafficClassProfile {
                latency_p95_ms: 420.0,
                jitter_p95_ms: 26.0,
                loss_pct: 2.0,
                pri_warm_threshold: 0.60,
                pri_switch_threshold: 0.85,
            },
            Self::GamingFps => TrafficClassProfile {
                latency_p95_ms: 20.0,
                jitter_p95_ms: 2.0,
                loss_pct: 0.1,
                pri_warm_threshold: 0.60,
                pri_switch_threshold: 0.85,
            },
            Self::RealtimeInteractive => TrafficClassProfile {
                latency_p95_ms: 50.0,
                jitter_p95_ms: 5.0,
                loss_pct: 0.3,
                pri_warm_threshold: 0.60,
                pri_switch_threshold: 0.85,
            },
            Self::Messaging => TrafficClassProfile {
                latency_p95_ms: 180.0,
                jitter_p95_ms: 20.0,
                loss_pct: 1.5,
                pri_warm_threshold: 0.60,
                pri_switch_threshold: 0.85,
            },
            Self::AuthSensitive => TrafficClassProfile {
                latency_p95_ms: 150.0,
                jitter_p95_ms: 10.0,
                loss_pct: 1.0,
                pri_warm_threshold: 0.65,
                pri_switch_threshold: 0.90,
            },
            Self::P2pRestricted => TrafficClassProfile {
                latency_p95_ms: 350.0,
                jitter_p95_ms: 25.0,
                loss_pct: 2.5,
                pri_warm_threshold: 0.60,
                pri_switch_threshold: 0.85,
            },
            Self::BackgroundTelemetry => TrafficClassProfile {
                latency_p95_ms: 600.0,
                jitter_p95_ms: 40.0,
                loss_pct: 4.0,
                pri_warm_threshold: 0.60,
                pri_switch_threshold: 0.85,
            },
            Self::ControlHealth => TrafficClassProfile {
                latency_p95_ms: 150.0,
                jitter_p95_ms: 10.0,
                loss_pct: 1.0,
                pri_warm_threshold: 0.60,
                pri_switch_threshold: 0.85,
            },
        }
    }
}

impl MultipathMode {
    pub fn from_dps_value(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "standby_only" => Ok(Self::StandbyOnly),
            "flow_shard" => Ok(Self::FlowShard),
            "aggregate_buffered" => Ok(Self::AggregateBuffered),
            _ => Err(
                "mesh multipath_mode must be one of: off, standby_only, flow_shard, aggregate_buffered"
                    .to_string(),
            ),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::StandbyOnly => "standby_only",
            Self::FlowShard => "flow_shard",
            Self::AggregateBuffered => "aggregate_buffered",
        }
    }
}

impl ContinuityPolicy {
    pub fn from_dps_value(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "same_egress_only" => Ok(Self::SameEgressOnly),
            "allow_flow_drain" => Ok(Self::AllowFlowDrain),
            "allow_hard_rebind_only" => Ok(Self::AllowHardRebindOnly),
            _ => Err(
                "mesh continuity_policy must be one of: same_egress_only, allow_flow_drain, allow_hard_rebind_only"
                    .to_string(),
            ),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SameEgressOnly => "same_egress_only",
            Self::AllowFlowDrain => "allow_flow_drain",
            Self::AllowHardRebindOnly => "allow_hard_rebind_only",
        }
    }
}

impl ShadowSwitchMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TransportOnly => "transport_only",
            Self::FlowDrain => "flow_drain",
            Self::HardRebindOnly => "hard_rebind_only",
            Self::Unknown => "unknown",
        }
    }
}

impl MeshPathProfile {
    fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "fast" => Ok(Self::Fast),
            "balanced" => Ok(Self::Balanced),
            "resilient" => Ok(Self::Resilient),
            _ => Err("mesh path_profile must be one of: fast, balanced, resilient".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshPathPolicy {
    pub allowed_regions: Vec<String>,
    pub blocked_node_ids: Vec<String>,
    pub require_min_reliability: u8,
    pub max_load_score: u8,
    pub max_peers: usize,
    pub prefer_region_diversity: bool,
    pub max_selected_per_region: usize,
    pub min_distinct_regions: usize,
    pub path_profile_override: Option<MeshPathProfile>,
    pub connect_fallback_ports: Vec<u16>,
}

impl MeshPathPolicy {
    pub fn default_auto() -> Self {
        Self {
            allowed_regions: Vec::new(),
            blocked_node_ids: Vec::new(),
            require_min_reliability: 0,
            max_load_score: 100,
            max_peers: 1,
            prefer_region_diversity: true,
            max_selected_per_region: 1,
            min_distinct_regions: 1,
            path_profile_override: None,
            connect_fallback_ports: vec![443, 8443],
        }
    }

    pub fn manual_override_fields(&self) -> Vec<&'static str> {
        let base = Self::default_auto();
        let mut overrides = Vec::new();
        if self.allowed_regions != base.allowed_regions {
            overrides.push("allowed_regions");
        }
        if self.blocked_node_ids != base.blocked_node_ids {
            overrides.push("blocked_node_ids");
        }
        if self.require_min_reliability != base.require_min_reliability {
            overrides.push("require_min_reliability");
        }
        if self.max_load_score != base.max_load_score {
            overrides.push("max_load_score");
        }
        if self.max_peers != base.max_peers {
            overrides.push("max_peers");
        }
        if self.prefer_region_diversity != base.prefer_region_diversity {
            overrides.push("prefer_region_diversity");
        }
        if self.max_selected_per_region != base.max_selected_per_region {
            overrides.push("max_selected_per_region");
        }
        if self.min_distinct_regions != base.min_distinct_regions {
            overrides.push("min_distinct_regions");
        }
        if self.path_profile_override != base.path_profile_override {
            overrides.push("path_profile_override");
        }
        if self.connect_fallback_ports != base.connect_fallback_ports {
            overrides.push("connect_fallback_ports");
        }
        overrides
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.require_min_reliability > 100 {
            return Err("mesh policy require_min_reliability must be <= 100".to_string());
        }
        if self.max_load_score > 100 {
            return Err("mesh policy max_load_score must be <= 100".to_string());
        }
        if self.max_peers == 0 {
            return Err("mesh policy max_peers must be > 0".to_string());
        }
        if self.max_selected_per_region == 0 {
            return Err("mesh policy max_selected_per_region must be > 0".to_string());
        }
        if self.max_selected_per_region > self.max_peers {
            return Err("mesh policy max_selected_per_region must be <= max_peers".to_string());
        }
        if self.min_distinct_regions == 0 {
            return Err("mesh policy min_distinct_regions must be > 0".to_string());
        }
        if self.min_distinct_regions > self.max_peers {
            return Err("mesh policy min_distinct_regions must be <= max_peers".to_string());
        }
        if self.connect_fallback_ports.is_empty() {
            return Err("mesh policy connect_fallback_ports must not be empty".to_string());
        }
        Ok(())
    }

    pub fn from_dps_payload(payload: &str) -> Result<Self, String> {
        if payload.trim().is_empty() {
            return Err("mesh policy payload is empty".to_string());
        }

        let mut allowed_regions: Vec<String> = Vec::new();
        let mut blocked_node_ids: Vec<String> = Vec::new();
        let mut require_min_reliability: Option<u8> = None;
        let mut max_load_score: Option<u8> = None;
        let mut max_peers: Option<usize> = None;
        let mut prefer_region_diversity: Option<bool> = None;
        let mut max_selected_per_region: Option<usize> = None;
        let mut min_distinct_regions: Option<usize> = None;
        let mut path_profile_override: Option<MeshPathProfile> = None;
        let mut connect_fallback_ports: Option<Vec<u16>> = None;
        let mut seen_mesh_keys = BTreeSet::new();

        for segment in payload.split(';') {
            let part = segment.trim();
            if part.is_empty() {
                continue;
            }
            let (key_raw, value_raw) = match part.split_once('=') {
                Some(v) => v,
                None => return Err("mesh policy payload field is malformed".to_string()),
            };
            let key = key_raw.trim();
            let key_norm = key.to_ascii_lowercase();
            let value = value_raw.trim();
            if value.is_empty() {
                return Err(format!("mesh policy payload field '{key}' is empty"));
            }
            if key_norm.starts_with("mesh_") && !seen_mesh_keys.insert(key_norm.clone()) {
                return Err(format!(
                    "mesh policy payload contains duplicate field '{key}'"
                ));
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
                }
                "mesh_max_load" => {
                    max_load_score = Some(parse_u8_field(value, key)?);
                }
                "mesh_max_peers" => {
                    max_peers = Some(parse_usize_field(value, key)?);
                }
                "mesh_prefer_region_diversity" => {
                    prefer_region_diversity = Some(parse_bool_field(value, key)?);
                }
                "mesh_max_selected_per_region" => {
                    max_selected_per_region = Some(parse_usize_field(value, key)?);
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
                    let _ = TrafficClass::from_dps_value(value)?;
                }
                "mesh_multipath_mode" => {
                    let _ = MultipathMode::from_dps_value(value)?;
                }
                "mesh_continuity_policy" => {
                    let _ = ContinuityPolicy::from_dps_value(value)?;
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

        let mut policy = Self::default_auto();
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
        policy.connect_fallback_ports =
            connect_fallback_ports.unwrap_or(policy.connect_fallback_ports);
        policy.validate()?;
        Ok(policy)
    }
}

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
        continuity_policy,
        shadow_switch_mode,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshPeerTablePolicy {
    pub max_entries: usize,
    pub max_entries_per_region: usize,
    pub stale_after_ticks: u64,
    pub target_distinct_regions: usize,
    pub replacement_min_score_delta: i32,
    pub degraded_replacement_min_score_delta: i32,
    pub max_replacements_per_window: u64,
    pub stability_window_ticks: u64,
    pub profile_hysteresis_ticks: u64,
    pub resilient_region_spread_bonus_weight: u8,
}

impl MeshPeerTablePolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_entries == 0 {
            return Err("mesh peer table max_entries must be > 0".to_string());
        }
        if self.max_entries_per_region == 0 {
            return Err("mesh peer table max_entries_per_region must be > 0".to_string());
        }
        if self.max_entries_per_region > self.max_entries {
            return Err("mesh peer table per-region limit must be <= max_entries".to_string());
        }
        if self.stale_after_ticks == 0 {
            return Err("mesh peer table stale_after_ticks must be > 0".to_string());
        }
        if self.target_distinct_regions == 0 {
            return Err("mesh peer table target_distinct_regions must be > 0".to_string());
        }
        if self.target_distinct_regions > self.max_entries {
            return Err(
                "mesh peer table target_distinct_regions must be <= max_entries".to_string(),
            );
        }
        if self.replacement_min_score_delta <= 0 {
            return Err("mesh peer table replacement_min_score_delta must be > 0".to_string());
        }
        if self.degraded_replacement_min_score_delta <= 0 {
            return Err(
                "mesh peer table degraded_replacement_min_score_delta must be > 0".to_string(),
            );
        }
        if self.degraded_replacement_min_score_delta > self.replacement_min_score_delta {
            return Err(
                "mesh peer table degraded_replacement_min_score_delta must be <= replacement_min_score_delta"
                    .to_string(),
            );
        }
        if self.max_replacements_per_window == 0 {
            return Err("mesh peer table max_replacements_per_window must be > 0".to_string());
        }
        if self.stability_window_ticks == 0 {
            return Err("mesh peer table stability_window_ticks must be > 0".to_string());
        }
        if self.profile_hysteresis_ticks == 0 {
            return Err("mesh peer table profile_hysteresis_ticks must be > 0".to_string());
        }
        if self.resilient_region_spread_bonus_weight == 0 {
            return Err(
                "mesh peer table resilient_region_spread_bonus_weight must be > 0".to_string(),
            );
        }
        Ok(())
    }

    pub fn from_dps_payload(payload: &str) -> Result<Self, String> {
        if payload.trim().is_empty() {
            return Err("mesh peer table payload is empty".to_string());
        }
        let mut policy = Self::default();
        let mut seen_mesh_table_keys = BTreeSet::new();

        for segment in payload.split(';') {
            let part = segment.trim();
            if part.is_empty() {
                continue;
            }
            let (key_raw, value_raw) = match part.split_once('=') {
                Some(v) => v,
                None => return Err("mesh peer table payload field is malformed".to_string()),
            };
            let key = key_raw.trim();
            let key_norm = key.to_ascii_lowercase();
            let value = value_raw.trim();
            if value.is_empty() {
                return Err(format!("mesh peer table payload field '{key}' is empty"));
            }
            if key_norm.starts_with("mesh_table_") && !seen_mesh_table_keys.insert(key_norm.clone())
            {
                return Err(format!(
                    "mesh peer table payload contains duplicate field '{key}'"
                ));
            }

            match key_norm.as_str() {
                "mesh_table_max_entries" => {
                    policy.max_entries = parse_usize_field(value, key)?;
                }
                "mesh_table_max_entries_per_region" => {
                    policy.max_entries_per_region = parse_usize_field(value, key)?;
                }
                "mesh_table_stale_after_ticks" => {
                    policy.stale_after_ticks = parse_u64_field(value, key)?;
                }
                "mesh_table_target_distinct_regions" => {
                    policy.target_distinct_regions = parse_usize_field(value, key)?;
                }
                "mesh_table_replacement_min_score_delta" => {
                    policy.replacement_min_score_delta = parse_i32_field(value, key)?;
                }
                "mesh_table_degraded_replacement_min_score_delta" => {
                    policy.degraded_replacement_min_score_delta = parse_i32_field(value, key)?;
                }
                "mesh_table_max_replacements_per_window" => {
                    policy.max_replacements_per_window = parse_u64_field(value, key)?;
                }
                "mesh_table_stability_window_ticks" => {
                    policy.stability_window_ticks = parse_u64_field(value, key)?;
                }
                "mesh_table_profile_hysteresis_ticks" => {
                    policy.profile_hysteresis_ticks = parse_u64_field(value, key)?;
                }
                "mesh_table_resilient_region_spread_bonus_weight" => {
                    policy.resilient_region_spread_bonus_weight = parse_u8_field(value, key)?;
                }
                _ => {
                    if key_norm.starts_with("mesh_table_") {
                        return Err(format!(
                            "mesh peer table payload contains unknown field '{key}'"
                        ));
                    }
                }
            }
        }

        policy.validate()?;
        Ok(policy)
    }
}

impl Default for MeshPeerTablePolicy {
    fn default() -> Self {
        Self {
            max_entries: 256,
            max_entries_per_region: 64,
            stale_after_ticks: 16,
            target_distinct_regions: 1,
            replacement_min_score_delta: 1,
            degraded_replacement_min_score_delta: 1,
            max_replacements_per_window: 8,
            stability_window_ticks: 8,
            profile_hysteresis_ticks: 4,
            resilient_region_spread_bonus_weight: 10,
        }
    }
}
