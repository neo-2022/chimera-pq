use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MeshNodeId(pub String);

impl MeshNodeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_non_empty_label(&self.0, "mesh node id")
    }
}

impl fmt::Display for MeshNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MeshNodeStatus {
    Healthy,
    Checking,
    Degraded,
    Down,
}

impl MeshNodeStatus {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "healthy" => Ok(Self::Healthy),
            "checking" => Ok(Self::Checking),
            "degraded" => Ok(Self::Degraded),
            "down" => Ok(Self::Down),
            _ => Err("mesh node status must be healthy, checking, degraded, or down".to_string()),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Checking => "checking",
            Self::Degraded => "degraded",
            Self::Down => "down",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Healthy => "ok",
            Self::Checking => "?",
            Self::Degraded => "!",
            Self::Down => "x",
        }
    }

    pub fn sort_rank(self) -> u8 {
        match self {
            Self::Healthy => 0,
            Self::Checking => 1,
            Self::Degraded => 2,
            Self::Down => 3,
        }
    }

    pub fn status_multiplier(self) -> f64 {
        match self {
            Self::Healthy => 1.0,
            Self::Checking => 0.75,
            Self::Degraded => 0.5,
            Self::Down => 0.0,
        }
    }

    pub fn is_down(self) -> bool {
        matches!(self, Self::Down)
    }

    pub fn is_available_now(self) -> bool {
        matches!(self, Self::Healthy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeshNodeCountrySource {
    OperatorOverride,
    GeoIp,
    NodeClaim,
    Unknown,
}

impl MeshNodeCountrySource {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "operator_override" => Ok(Self::OperatorOverride),
            "geoip" => Ok(Self::GeoIp),
            "node_claim" => Ok(Self::NodeClaim),
            "unknown" => Ok(Self::Unknown),
            _ => Err(
                "mesh node country source must be operator_override, geoip, node_claim, or unknown"
                    .to_string(),
            ),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::OperatorOverride => "operator_override",
            Self::GeoIp => "geoip",
            Self::NodeClaim => "node_claim",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeshNodeCountryConfidence {
    High,
    Medium,
    Low,
}

impl MeshNodeCountryConfidence {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            _ => Err("mesh node country confidence must be high, medium, or low".to_string()),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshNodeCountry {
    pub country_code: String,
    pub country_name: String,
    pub country_source: MeshNodeCountrySource,
    pub country_confidence: MeshNodeCountryConfidence,
    pub country_updated_at: String,
    pub country_ttl_sec: u64,
    pub country_conflict: bool,
    pub country_conflict_reason: Option<String>,
}

impl MeshNodeCountry {
    pub const UNKNOWN_CODE: &'static str = "ZZ";
    pub const UNKNOWN_NAME: &'static str = "Неизвестная страна";

    pub fn unknown(updated_at: impl Into<String>, ttl_sec: u64) -> Self {
        Self {
            country_code: Self::UNKNOWN_CODE.to_string(),
            country_name: Self::UNKNOWN_NAME.to_string(),
            country_source: MeshNodeCountrySource::Unknown,
            country_confidence: MeshNodeCountryConfidence::Low,
            country_updated_at: updated_at.into(),
            country_ttl_sec: ttl_sec,
            country_conflict: false,
            country_conflict_reason: Some("страна не определена".to_string()),
        }
    }

    pub fn is_unknown(&self) -> bool {
        self.country_code == Self::UNKNOWN_CODE
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_country_code(&self.country_code)?;
        validate_non_empty_label(&self.country_name, "mesh node country name")?;
        validate_non_empty_label(&self.country_updated_at, "mesh node country updated_at")?;
        if self.country_ttl_sec == 0 {
            return Err("mesh node country ttl must be > 0".to_string());
        }
        if self.is_unknown() && self.country_name != Self::UNKNOWN_NAME {
            return Err("mesh node unknown country must use unknown country name".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshNode {
    pub node_id: MeshNodeId,
    pub endpoint: String,
    pub invite_token: Option<String>,
    pub country: MeshNodeCountry,
    pub status: MeshNodeStatus,
    pub latency_ms: Option<f64>,
    pub jitter_ms: Option<f64>,
    pub loss_pct: Option<f64>,
    pub success_rate_5m: Option<f64>,
    pub success_rate_1h: Option<f64>,
    pub consecutive_failures: u32,
    pub observation_count: u32,
    pub score: f64,
    pub explain_reason: String,
}

impl MeshNode {
    pub fn validate(&self) -> Result<(), String> {
        self.node_id.validate()?;
        validate_endpoint_config_value(&self.endpoint)?;
        if let Some(invite_token) = self.invite_token.as_deref() {
            validate_non_empty_label(invite_token, "mesh node invite_token")?;
        }
        self.country.validate()?;
        validate_optional_non_negative("latency_ms", self.latency_ms)?;
        validate_optional_non_negative("jitter_ms", self.jitter_ms)?;
        validate_optional_pct("loss_pct", self.loss_pct)?;
        validate_optional_pct("success_rate_5m", self.success_rate_5m)?;
        validate_optional_pct("success_rate_1h", self.success_rate_1h)?;
        if !self.score.is_finite() || !(0.0..=100.0).contains(&self.score) {
            return Err("mesh node score must be finite and in 0..100".to_string());
        }
        Ok(())
    }

    pub fn is_selectable(&self) -> bool {
        !self.status.is_down()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeshNodeReasonCode {
    NoNodes,
    AllNodesDown,
    BestSelected,
    CurrentStillGood,
    CandidateBetterByMargin,
    CandidateNotBetterEnough,
    CurrentDownEmergencySwitch,
    PinnedNodeActive,
    PinnedNodeDownEmergencySwitch,
    HoldDownActive,
    MaxSwitchRateExceeded,
    InsufficientObservations,
    GeoIpConflict,
    CountryUnknown,
    GeoIpErrorFallbackCache,
    GeoIpErrorFallbackNodeClaim,
    GeoIpErrorFallbackUnknown,
}

impl MeshNodeReasonCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoNodes => "no_nodes",
            Self::AllNodesDown => "all_nodes_down",
            Self::BestSelected => "best_selected",
            Self::CurrentStillGood => "current_still_good",
            Self::CandidateBetterByMargin => "candidate_better_by_margin",
            Self::CandidateNotBetterEnough => "candidate_not_better_enough",
            Self::CurrentDownEmergencySwitch => "current_down_emergency_switch",
            Self::PinnedNodeActive => "pinned_node_active",
            Self::PinnedNodeDownEmergencySwitch => "pinned_node_down_emergency_switch",
            Self::HoldDownActive => "hold_down_active",
            Self::MaxSwitchRateExceeded => "max_switch_rate_exceeded",
            Self::InsufficientObservations => "insufficient_observations",
            Self::GeoIpConflict => "geoip_conflict",
            Self::CountryUnknown => "country_unknown",
            Self::GeoIpErrorFallbackCache => "geoip_error_fallback_cache",
            Self::GeoIpErrorFallbackNodeClaim => "geoip_error_fallback_node_claim",
            Self::GeoIpErrorFallbackUnknown => "geoip_error_fallback_unknown",
        }
    }
}

impl fmt::Display for MeshNodeReasonCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshNodeScoreBreakdown {
    pub node_id: MeshNodeId,
    pub latency_q: f64,
    pub jitter_q: f64,
    pub loss_q: f64,
    pub success_5m_q: f64,
    pub success_1h_q: f64,
    pub observation_q: f64,
    pub failure_penalty: f64,
    pub status_multiplier: f64,
    pub base_score: f64,
    pub final_score: f64,
    pub reason: MeshNodeReasonCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshNodeSwitchAction {
    Noop,
    Connect,
    Switch,
    ManualConnect,
    Pin,
    Unpin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshNodeSwitchDecision {
    pub action: MeshNodeSwitchAction,
    pub reason: MeshNodeReasonCode,
    pub current_node: Option<MeshNodeId>,
    pub candidate_node: Option<MeshNodeId>,
    pub current_score: Option<f64>,
    pub candidate_score: Option<f64>,
    pub allowed: bool,
    pub explain: String,
}

impl MeshNodeSwitchDecision {
    pub fn noop(
        reason: MeshNodeReasonCode,
        current_node: Option<MeshNodeId>,
        explain: impl Into<String>,
    ) -> Self {
        Self {
            action: MeshNodeSwitchAction::Noop,
            reason,
            current_node,
            candidate_node: None,
            current_score: None,
            candidate_score: None,
            allowed: false,
            explain: explain.into(),
        }
    }
}

pub fn validate_country_code(code: &str) -> Result<(), String> {
    if code == MeshNodeCountry::UNKNOWN_CODE {
        return Ok(());
    }
    if code.len() == 2 && code.chars().all(|ch| ch.is_ascii_uppercase()) {
        Ok(())
    } else {
        Err("mesh node country_code must be A-Z{2} or ZZ".to_string())
    }
}

pub(crate) fn cmp_f64_desc(a: f64, b: f64) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => b.partial_cmp(&a).unwrap_or(Ordering::Equal),
    }
}

pub(crate) fn cmp_optional_f64_asc(a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(left), Some(right)) => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn validate_endpoint_config_value(endpoint: &str) -> Result<(), String> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return Err("mesh node endpoint is empty".to_string());
    }
    if endpoint != trimmed {
        return Err("mesh node endpoint contains surrounding spaces".to_string());
    }
    if endpoint.chars().any(char::is_whitespace) {
        return Err("mesh node endpoint contains whitespace".to_string());
    }
    Ok(())
}

fn validate_non_empty_label(value: &str, label: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is empty"));
    }
    if value != trimmed {
        return Err(format!("{label} contains surrounding spaces"));
    }
    if value.contains('\n') || value.contains('\r') || value.contains('\t') {
        return Err(format!("{label} contains control whitespace"));
    }
    Ok(())
}

fn validate_optional_non_negative(name: &str, value: Option<f64>) -> Result<(), String> {
    if let Some(value) = value
        && (!value.is_finite() || value < 0.0)
    {
        return Err(format!("mesh node {name} must be finite and >= 0"));
    }
    Ok(())
}

fn validate_optional_pct(name: &str, value: Option<f64>) -> Result<(), String> {
    if let Some(value) = value
        && (!value.is_finite() || !(0.0..=100.0).contains(&value))
    {
        return Err(format!("mesh node {name} must be finite and in 0..100"));
    }
    Ok(())
}
