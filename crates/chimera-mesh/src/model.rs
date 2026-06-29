use crate::{multipath_model::MeshMultipathSchedule, nodes_model::validate_update_bootstrap_url};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshJoinRequest {
    pub namespace: String,
    pub node_name: String,
    pub invite_token: Option<String>,
}

impl MeshJoinRequest {
    pub fn validate(&self) -> Result<(), String> {
        validate_request_name_field(&self.namespace, "mesh namespace")?;
        validate_request_name_field(&self.node_name, "mesh node_name")?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshJoinMode {
    InvitationOnly,
    PublicDiscovery,
}

impl MeshJoinMode {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::InvitationOnly => "InvitationOnly",
            Self::PublicDiscovery => "PublicDiscovery",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MeshDiscoveryRecord {
    pub node_id: String,
    pub endpoint: String,
    pub region: String,
    pub load_score: u8,
    pub reliability_score: u8,
}

impl MeshDiscoveryRecord {
    pub fn validate(&self) -> Result<(), String> {
        validate_label_field(&self.node_id, "mesh discovery node_id")?;
        let endpoint = self.endpoint.trim();
        if endpoint.is_empty() {
            return Err("mesh discovery endpoint is empty".to_string());
        }
        validate_endpoint_host_port(endpoint)?;
        validate_label_field(&self.region, "mesh discovery region")?;
        if self.load_score > 100 {
            return Err("mesh discovery load_score must be <= 100".to_string());
        }
        if self.reliability_score > 100 {
            return Err("mesh discovery reliability_score must be <= 100".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MeshPublishedEndpointUpdate {
    pub node_id: String,
    pub endpoint: String,
    pub update_bootstrap_url: Option<String>,
    pub endpoint_generation: u64,
}

impl MeshPublishedEndpointUpdate {
    pub fn validate(&self) -> Result<(), String> {
        validate_label_field(&self.node_id, "mesh published endpoint node_id")?;
        let endpoint = self.endpoint.trim();
        if endpoint.is_empty() {
            return Err("mesh published endpoint is empty".to_string());
        }
        validate_endpoint_host_port_for(endpoint, "mesh published endpoint")?;
        if let Some(update_bootstrap_url) = self.update_bootstrap_url.as_deref() {
            validate_update_bootstrap_url(update_bootstrap_url)?;
        }
        if self.endpoint_generation == 0 {
            return Err("mesh published endpoint_generation must be > 0".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MeshPeerState {
    pub node_id: String,
    pub endpoint: String,
    pub region: String,
    pub reliability_score: u8,
    pub load_score: u8,
    pub latency_ms: Option<u32>,
    pub throughput_mbps: Option<u32>,
    pub selection_score: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshPeerPerformance {
    pub node_id: String,
    pub latency_ms: Option<u32>,
    pub throughput_mbps: Option<u32>,
}

impl MeshPeerPerformance {
    pub fn validate(&self) -> Result<(), String> {
        validate_label_field(&self.node_id, "mesh peer performance node_id")?;
        if matches!(self.latency_ms, Some(0)) {
            return Err("mesh peer performance latency_ms must be > 0".to_string());
        }
        if matches!(self.throughput_mbps, Some(0)) {
            return Err("mesh peer performance throughput_mbps must be > 0".to_string());
        }
        if self.latency_ms.is_none() && self.throughput_mbps.is_none() {
            return Err("mesh peer performance has no signal".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MeshPathPlan {
    pub namespace: String,
    pub join_mode: MeshJoinMode,
    pub selected_peers: Vec<MeshPeerState>,
    pub multipath_schedule: MeshMultipathSchedule,
    pub explain: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshFailoverEvent {
    pub failed_node_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshPeerHealth {
    pub node_id: String,
    pub healthy: bool,
    pub cooldown_active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreemptiveRisk {
    pub instant_risk: f64,
    pub trend_risk: f64,
    pub pri: f64,
    pub trend_lat: f64,
    pub trend_jit: f64,
    pub trend_loss: f64,
}

impl PreemptiveRisk {
    pub fn validate(&self) -> Result<(), String> {
        validate_unit_interval_f64(self.instant_risk, "preemptive_risk instant_risk")?;
        validate_unit_interval_f64(self.trend_risk, "preemptive_risk trend_risk")?;
        validate_unit_interval_f64(self.pri, "preemptive_risk pri")?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchDecision {
    pub should_prepare: bool,
    pub should_switch: bool,
    pub target_peer: Option<String>,
    pub reason: String,
    pub confidence: f64,
}

impl SwitchDecision {
    pub fn validate(&self) -> Result<(), String> {
        if self.reason.trim().is_empty() {
            return Err("switch_decision reason is empty".to_string());
        }
        validate_unit_interval_f64(self.confidence, "switch_decision confidence")
    }
}

pub(crate) fn peer_priority(peer: &MeshPeerState) -> i32 {
    (peer.reliability_score as i32 * 2) - peer.load_score as i32
}

fn validate_unit_interval_f64(value: f64, label: &str) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("{label} is not finite"));
    }
    if !(0.0..=1.0).contains(&value) {
        return Err(format!("{label} must be in [0.0, 1.0]"));
    }
    Ok(())
}

fn validate_endpoint_host_port(endpoint: &str) -> Result<(), String> {
    validate_endpoint_host_port_for(endpoint, "mesh discovery endpoint")
}

fn validate_endpoint_host_port_for(endpoint: &str, label: &str) -> Result<(), String> {
    if endpoint.starts_with('[') {
        return validate_bracketed_ipv6_endpoint(endpoint, label);
    }
    let (host, port_raw) = endpoint
        .rsplit_once(':')
        .ok_or_else(|| format!("{label} must be in host:port format"))?;
    if host.contains(':') {
        return Err(format!("{label} IPv6 endpoint must use [addr]:port format"));
    }
    validate_host_and_port(host, port_raw, label)
}

fn validate_bracketed_ipv6_endpoint(endpoint: &str, label: &str) -> Result<(), String> {
    let close = endpoint
        .find(']')
        .ok_or_else(|| format!("{label} IPv6 endpoint is missing closing bracket"))?;
    let host = &endpoint[1..close];
    let tail = &endpoint[(close + 1)..];
    let port_raw = tail
        .strip_prefix(':')
        .ok_or_else(|| format!("{label} must be in [addr]:port format"))?;
    validate_host_and_port(host, port_raw, label)
}

fn validate_host_and_port(host: &str, port_raw: &str, label: &str) -> Result<(), String> {
    if host.trim().is_empty() {
        return Err(format!("{label} host is empty"));
    }
    if host != host.trim() {
        return Err(format!("{label} host contains surrounding spaces"));
    }
    if host.contains(',') {
        return Err(format!("{label} host contains comma"));
    }
    if host.chars().any(char::is_whitespace) {
        return Err(format!("{label} host contains whitespace"));
    }
    if port_raw != port_raw.trim() {
        return Err(format!("{label} port contains surrounding spaces"));
    }
    if port_raw.chars().any(char::is_whitespace) {
        return Err(format!("{label} port contains whitespace"));
    }
    let port = port_raw
        .parse::<u16>()
        .map_err(|_| format!("{label} port is invalid"))?;
    if port == 0 {
        return Err(format!("{label} port must be > 0"));
    }
    Ok(())
}

fn validate_label_field(value: &str, label: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is empty"));
    }
    if value != trimmed {
        return Err(format!("{label} contains surrounding spaces"));
    }
    if value.contains(',') {
        return Err(format!("{label} contains comma"));
    }
    if value.contains('\n') || value.contains('\r') || value.contains('\t') {
        return Err(format!("{label} contains control whitespace"));
    }
    Ok(())
}

fn validate_request_name_field(value: &str, label: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is empty"));
    }
    if trimmed.contains(',') {
        return Err(format!("{label} contains comma"));
    }
    if trimmed.contains('\n') || trimmed.contains('\r') || trimmed.contains('\t') {
        return Err(format!("{label} contains control whitespace"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::MeshDiscoveryRecord;

    #[test]
    fn mesh_discovery_endpoint_rejects_comma() {
        let record = MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "bad,host.example:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 90,
        };

        let result = record.validate();
        assert!(result.is_err());
        let error = result.err().unwrap_or_default();

        assert!(error.contains("comma"));
    }
}
