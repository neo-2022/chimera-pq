#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use chimera_core::{ChimeraError, ChimeraResult};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RawConfig {
    values: BTreeMap<String, RawValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawValue {
    value: String,
    line: usize,
}

impl RawConfig {
    pub fn parse(input: &str) -> ChimeraResult<Self> {
        let mut values: BTreeMap<String, RawValue> = BTreeMap::new();

        for (index, raw_line) in input.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                return Err(ChimeraError::InvalidConfig(format!(
                    "line {} is missing '='",
                    index + 1
                )));
            };

            let key = key.trim();
            let value = value.trim().trim_matches('"');

            if key.is_empty() {
                return Err(ChimeraError::InvalidConfig(format!(
                    "line {} has empty key",
                    index + 1
                )));
            }

            if let Some(previous) = values.get(key) {
                return Err(ChimeraError::InvalidConfig(format!(
                    "line {} duplicates key '{}' (first declared at line {})",
                    index + 1,
                    key,
                    previous.line
                )));
            }
            values.insert(
                key.to_string(),
                RawValue {
                    value: value.to_string(),
                    line: index + 1,
                },
            );
        }

        Ok(Self { values })
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|raw| raw.value.as_str())
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }

    pub fn key_line(&self, key: &str) -> Option<usize> {
        self.values.get(key).map(|raw| raw.line)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigCarrierProfile {
    InMemory,
    Tls,
    Quic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigCaptureMode {
    Auto,
    Tun,
    LocalProxy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RekeyLimits {
    pub max_age_seconds: u64,
    pub max_packets_per_key: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientConfig {
    pub carrier_profile: ConfigCarrierProfile,
    pub carrier_addr: String,
    pub carrier_server_name: String,
    pub capture_mode: ConfigCaptureMode,
    pub tun_supported: bool,
    pub split_tunnel_default: bool,
    pub auto_failover: bool,
    pub invisible_mode_required: bool,
    pub rekey: RekeyLimits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayConfig {
    pub carrier_profile: ConfigCarrierProfile,
    pub listen_addr: String,
    pub rekey: RekeyLimits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshRuntimeConfig {
    pub peer_table_max_entries: usize,
    pub peer_table_max_entries_per_region: usize,
    pub peer_table_stale_after_ticks: u64,
}

impl Default for MeshRuntimeConfig {
    fn default() -> Self {
        Self {
            peer_table_max_entries: 256,
            peer_table_max_entries_per_region: 64,
            peer_table_stale_after_ticks: 16,
        }
    }
}

pub fn parse_client_config_text(input: &str) -> ChimeraResult<ClientConfig> {
    let raw = RawConfig::parse(input)?;
    validate_keys(
        &raw,
        &[
            "carrier.profile",
            "carrier.addr",
            "carrier.server_name",
            "capture.mode",
            "capture.tun_supported",
            "routing.split_tunnel_default",
            "routing.auto_failover",
            "runtime.invisible_mode_required",
            "rekey.max_age_seconds",
            "rekey.max_packets_per_key",
        ],
        "client",
    )?;
    let carrier_profile = parse_carrier_profile(
        raw.get("carrier.profile").unwrap_or("in-memory"),
        "carrier.profile",
    )?;
    let carrier_addr = raw
        .get("carrier.addr")
        .unwrap_or("127.0.0.1:443")
        .to_string();
    let carrier_server_name = raw
        .get("carrier.server_name")
        .unwrap_or("gateway.example.org")
        .to_string();
    if carrier_addr.trim().is_empty() {
        return Err(ChimeraError::InvalidConfig(
            "carrier.addr is empty".to_string(),
        ));
    }
    if carrier_server_name.trim().is_empty() {
        return Err(ChimeraError::InvalidConfig(
            "carrier.server_name is empty".to_string(),
        ));
    }
    let capture_mode =
        parse_capture_mode(raw.get("capture.mode").unwrap_or("auto"), "capture.mode")?;
    let tun_supported = parse_bool(
        raw.get("capture.tun_supported").unwrap_or("true"),
        "capture.tun_supported",
    )?;
    let split_tunnel_default = parse_bool(
        raw.get("routing.split_tunnel_default").unwrap_or("true"),
        "routing.split_tunnel_default",
    )?;
    let auto_failover = parse_bool(
        raw.get("routing.auto_failover").unwrap_or("true"),
        "routing.auto_failover",
    )?;
    let invisible_mode_required = parse_bool(
        raw.get("runtime.invisible_mode_required").unwrap_or("true"),
        "runtime.invisible_mode_required",
    )?;
    let rekey = parse_rekey_limits(&raw)?;

    Ok(ClientConfig {
        carrier_profile,
        carrier_addr,
        carrier_server_name,
        capture_mode,
        tun_supported,
        split_tunnel_default,
        auto_failover,
        invisible_mode_required,
        rekey,
    })
}

pub fn parse_gateway_config_text(input: &str) -> ChimeraResult<GatewayConfig> {
    let raw = RawConfig::parse(input)?;
    validate_keys(
        &raw,
        &[
            "carrier.profile",
            "gateway.listen_addr",
            "rekey.max_age_seconds",
            "rekey.max_packets_per_key",
        ],
        "gateway",
    )?;
    let carrier_profile = parse_carrier_profile(
        raw.get("carrier.profile").unwrap_or("tls"),
        "carrier.profile",
    )?;
    let listen_addr = raw
        .get("gateway.listen_addr")
        .unwrap_or("0.0.0.0:443")
        .to_string();
    if listen_addr.trim().is_empty() {
        return Err(ChimeraError::InvalidConfig(
            "gateway.listen_addr is empty".to_string(),
        ));
    }
    let rekey = parse_rekey_limits(&raw)?;
    Ok(GatewayConfig {
        carrier_profile,
        listen_addr,
        rekey,
    })
}

pub fn parse_mesh_runtime_config_text(input: &str) -> ChimeraResult<MeshRuntimeConfig> {
    let raw = RawConfig::parse(input)?;
    validate_keys(
        &raw,
        &[
            "mesh.table.max_entries",
            "mesh.table.max_entries_per_region",
            "mesh.table.stale_after_ticks",
        ],
        "mesh-runtime",
    )?;

    let max_entries = parse_usize(
        raw.get("mesh.table.max_entries").unwrap_or("256"),
        "mesh.table.max_entries",
    )?;
    let max_entries_per_region = parse_usize(
        raw.get("mesh.table.max_entries_per_region").unwrap_or("64"),
        "mesh.table.max_entries_per_region",
    )?;
    let stale_after_ticks = parse_u64(
        raw.get("mesh.table.stale_after_ticks").unwrap_or("16"),
        "mesh.table.stale_after_ticks",
    )?;

    if max_entries == 0 {
        return Err(ChimeraError::InvalidConfig(
            "mesh.table.max_entries must be > 0".to_string(),
        ));
    }
    if max_entries_per_region == 0 {
        return Err(ChimeraError::InvalidConfig(
            "mesh.table.max_entries_per_region must be > 0".to_string(),
        ));
    }
    if max_entries_per_region > max_entries {
        return Err(ChimeraError::InvalidConfig(
            "mesh.table.max_entries_per_region must be <= mesh.table.max_entries".to_string(),
        ));
    }
    if stale_after_ticks == 0 {
        return Err(ChimeraError::InvalidConfig(
            "mesh.table.stale_after_ticks must be > 0".to_string(),
        ));
    }

    Ok(MeshRuntimeConfig {
        peer_table_max_entries: max_entries,
        peer_table_max_entries_per_region: max_entries_per_region,
        peer_table_stale_after_ticks: stale_after_ticks,
    })
}

fn validate_keys(raw: &RawConfig, allowed: &[&str], profile: &str) -> ChimeraResult<()> {
    for key in raw.keys() {
        if !allowed.contains(&key) {
            let line = raw.key_line(key).unwrap_or(0);
            let suggestion = suggest_key(key, allowed)
                .map(|candidate| format!("; did you mean '{candidate}'?"))
                .unwrap_or_default();
            return Err(ChimeraError::InvalidConfig(format!(
                "line {line}: unknown {profile} config key '{key}'{suggestion}"
            )));
        }
    }
    Ok(())
}

fn suggest_key<'a>(unknown: &str, allowed: &'a [&str]) -> Option<&'a str> {
    let mut best: Option<(&str, usize)> = None;
    for candidate in allowed {
        let distance = edit_distance(unknown, candidate);
        if distance <= 3 && best.is_none_or(|(_, current)| distance < current) {
            best = Some((candidate, distance));
        }
    }
    best.map(|(candidate, _)| candidate)
}

fn edit_distance(left: &str, right: &str) -> usize {
    let left_chars: Vec<char> = left.chars().collect();
    let right_chars: Vec<char> = right.chars().collect();
    if left_chars.is_empty() {
        return right_chars.len();
    }
    if right_chars.is_empty() {
        return left_chars.len();
    }

    let mut prev: Vec<usize> = (0..=right_chars.len()).collect();
    let mut curr: Vec<usize> = vec![0; right_chars.len() + 1];

    for (i, left_char) in left_chars.iter().enumerate() {
        curr[0] = i + 1;
        for (j, right_char) in right_chars.iter().enumerate() {
            let cost = usize::from(left_char != right_char);
            curr[j + 1] = (curr[j] + 1).min(prev[j + 1] + 1).min(prev[j] + cost);
        }
        prev.clone_from(&curr);
    }
    prev[right_chars.len()]
}

fn parse_rekey_limits(raw: &RawConfig) -> ChimeraResult<RekeyLimits> {
    let max_age_seconds = parse_u64(
        raw.get("rekey.max_age_seconds").unwrap_or("300"),
        "rekey.max_age_seconds",
    )?;
    let max_packets_per_key = parse_u64(
        raw.get("rekey.max_packets_per_key").unwrap_or("10000"),
        "rekey.max_packets_per_key",
    )?;
    if max_age_seconds == 0 {
        return Err(ChimeraError::InvalidConfig(
            "rekey.max_age_seconds must be > 0".to_string(),
        ));
    }
    if max_packets_per_key == 0 {
        return Err(ChimeraError::InvalidConfig(
            "rekey.max_packets_per_key must be > 0".to_string(),
        ));
    }
    Ok(RekeyLimits {
        max_age_seconds,
        max_packets_per_key,
    })
}

fn parse_u64(input: &str, key: &str) -> ChimeraResult<u64> {
    input
        .parse::<u64>()
        .map_err(|_| ChimeraError::InvalidConfig(format!("{key} must be an integer")))
}

fn parse_usize(input: &str, key: &str) -> ChimeraResult<usize> {
    input
        .parse::<usize>()
        .map_err(|_| ChimeraError::InvalidConfig(format!("{key} must be an integer")))
}

fn parse_bool(input: &str, key: &str) -> ChimeraResult<bool> {
    match input {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(ChimeraError::InvalidConfig(format!(
            "{key} must be true or false"
        ))),
    }
}

fn parse_carrier_profile(input: &str, key: &str) -> ChimeraResult<ConfigCarrierProfile> {
    match input {
        "in-memory" => Ok(ConfigCarrierProfile::InMemory),
        "tls" => Ok(ConfigCarrierProfile::Tls),
        "quic" => Ok(ConfigCarrierProfile::Quic),
        _ => Err(ChimeraError::InvalidConfig(format!(
            "{key} has unknown value '{input}'"
        ))),
    }
}

fn parse_capture_mode(input: &str, key: &str) -> ChimeraResult<ConfigCaptureMode> {
    match input {
        "auto" => Ok(ConfigCaptureMode::Auto),
        "tun" => Ok(ConfigCaptureMode::Tun),
        "local-proxy" => Ok(ConfigCaptureMode::LocalProxy),
        _ => Err(ChimeraError::InvalidConfig(format!(
            "{key} has unknown value '{input}'"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConfigCaptureMode, ConfigCarrierProfile, RawConfig, parse_client_config_text,
        parse_gateway_config_text, parse_mesh_runtime_config_text,
    };

    #[test]
    fn parses_simple_key_values() {
        let parsed = RawConfig::parse("gateway = \"127.0.0.1:8443\"\nmode = test\n");
        assert!(parsed.is_ok());
        let config = match parsed {
            Ok(config) => config,
            Err(error) => unreachable!("config should parse: {error}"),
        };

        assert_eq!(config.get("gateway"), Some("127.0.0.1:8443"));
        assert_eq!(config.get("mode"), Some("test"));
    }

    #[test]
    fn rejects_line_without_separator() {
        let parsed = RawConfig::parse("gateway 127.0.0.1");
        assert!(parsed.is_err());
    }

    #[test]
    fn rejects_duplicate_key() {
        let parsed = RawConfig::parse("carrier.profile = tls\ncarrier.profile = quic\n");
        assert!(parsed.is_err());
    }

    #[test]
    fn duplicate_key_error_contains_first_line() {
        let parsed = RawConfig::parse("carrier.profile = tls\ncarrier.profile = quic\n");
        match parsed {
            Ok(_) => unreachable!("config must fail"),
            Err(error) => {
                let message = error.to_string();
                assert!(message.contains("line 2"));
                assert!(message.contains("first declared at line 1"));
            }
        }
    }

    #[test]
    fn parses_client_config_with_defaults() {
        let text = "carrier.profile = tls\ncarrier.addr = 203.0.113.10:443\n";
        let config = match parse_client_config_text(text) {
            Ok(config) => config,
            Err(error) => unreachable!("client config should parse: {error}"),
        };
        assert_eq!(config.carrier_profile, ConfigCarrierProfile::Tls);
        assert_eq!(config.capture_mode, ConfigCaptureMode::Auto);
        assert_eq!(config.rekey.max_age_seconds, 300);
        assert_eq!(config.rekey.max_packets_per_key, 10_000);
    }

    #[test]
    fn rejects_client_config_with_duplicate_key() {
        let text = "carrier.addr = 203.0.113.10:443\ncarrier.addr = 198.51.100.7:443\n";
        let parsed = parse_client_config_text(text);
        assert!(parsed.is_err());
    }

    #[test]
    fn rejects_client_config_with_bad_bool() {
        let text = "capture.tun_supported = maybe";
        assert!(parse_client_config_text(text).is_err());
    }

    #[test]
    fn rejects_client_config_with_unknown_key() {
        let text = "carrier.addr = 203.0.113.10:443\ncapture.mdoe = auto\n";
        assert!(parse_client_config_text(text).is_err());
    }

    #[test]
    fn unknown_key_error_contains_line_number() {
        let text = "carrier.addr = 203.0.113.10:443\ncapture.mdoe = auto\n";
        let parsed = parse_client_config_text(text);
        match parsed {
            Ok(_) => unreachable!("config must fail"),
            Err(error) => {
                let message = error.to_string();
                assert!(message.contains("line 2"));
            }
        }
    }

    #[test]
    fn unknown_key_error_contains_suggestion() {
        let text = "capture.mdoe = auto\n";
        let parsed = parse_client_config_text(text);
        match parsed {
            Ok(_) => unreachable!("config must fail"),
            Err(error) => {
                let message = error.to_string();
                assert!(message.contains("did you mean 'capture.mode'?"));
            }
        }
    }

    #[test]
    fn unknown_key_error_without_close_match_has_no_suggestion() {
        let text = "zzzzzzzz = value\n";
        let parsed = parse_client_config_text(text);
        match parsed {
            Ok(_) => unreachable!("config must fail"),
            Err(error) => {
                let message = error.to_string();
                assert!(message.contains("unknown client config key 'zzzzzzzz'"));
                assert!(!message.contains("did you mean"));
            }
        }
    }

    #[test]
    fn gateway_unknown_key_error_contains_suggestion() {
        let text = "gateway.lsiten_addr = 127.0.0.1:443\n";
        let parsed = parse_gateway_config_text(text);
        match parsed {
            Ok(_) => unreachable!("config must fail"),
            Err(error) => {
                let message = error.to_string();
                assert!(message.contains("did you mean 'gateway.listen_addr'?"));
            }
        }
    }

    #[test]
    fn parses_gateway_config_with_defaults() {
        let text = "gateway.listen_addr = 0.0.0.0:8443";
        let config = match parse_gateway_config_text(text) {
            Ok(config) => config,
            Err(error) => unreachable!("gateway config should parse: {error}"),
        };
        assert_eq!(config.listen_addr, "0.0.0.0:8443");
        assert_eq!(config.carrier_profile, ConfigCarrierProfile::Tls);
    }

    #[test]
    fn rejects_gateway_config_with_unknown_key() {
        let text = "gateway.listen_addr = 0.0.0.0:8443\ngateway.lsiten_addr = 127.0.0.1:443\n";
        assert!(parse_gateway_config_text(text).is_err());
    }

    #[test]
    fn parses_mesh_runtime_config_with_defaults() {
        let config = parse_mesh_runtime_config_text("").unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(config.peer_table_max_entries, 256);
        assert_eq!(config.peer_table_max_entries_per_region, 64);
        assert_eq!(config.peer_table_stale_after_ticks, 16);
    }

    #[test]
    fn parses_mesh_runtime_config_explicit_values() {
        let text = "mesh.table.max_entries = 128\nmesh.table.max_entries_per_region = 8\nmesh.table.stale_after_ticks = 12\n";
        let config = parse_mesh_runtime_config_text(text).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(config.peer_table_max_entries, 128);
        assert_eq!(config.peer_table_max_entries_per_region, 8);
        assert_eq!(config.peer_table_stale_after_ticks, 12);
    }

    #[test]
    fn rejects_mesh_runtime_config_bad_ratio() {
        let text = "mesh.table.max_entries = 4\nmesh.table.max_entries_per_region = 8\n";
        assert!(parse_mesh_runtime_config_text(text).is_err());
    }
}
