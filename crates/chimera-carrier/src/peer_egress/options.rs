use std::env;

use crate::peer_egress::options_mode::parse_mode;
pub use crate::peer_egress::options_mode::{Mode, mode_name};
use crate::peer_egress::options_proof::TransitProofOptions;
use crate::peer_egress::options_transit_guard::TransitRelayGuardOptionValues;
use crate::peer_egress::transit_guard::TransitRelayLimits;

pub const HANDSHAKE_MAGIC: &[u8] = b"CHIMERA-PEER-EGRESS/1\n";
pub const LOCAL_MAGIC: &[u8] = b"CHIMERA-LOCAL/1\n";
pub const MAX_TOKEN_LEN: usize = 256;
pub const SECURE_MAGIC: &[u8] = b"CHIMERA-PEER-SECURE/1\n";
pub const SECURE_NONCE_LEN: usize = 32;
pub const SECURE_CHACHA20POLY1305_SUITE_ID: u16 = 0xEE02;
pub const SECURE_AES256GCM_SUITE_ID: u16 = 0xEE03;
pub const SECURE_PLAINTEXT_CHUNK_LEN: usize = 1024 * 1024;
pub const SECURE_MAX_CIPHERTEXT_LEN: usize = SECURE_PLAINTEXT_CHUNK_LEN + 32;
pub const TCP_BUFFER_BYTES: usize = 4 * 1024 * 1024;
pub const NODE_DEFAULT_LOCAL_LISTEN: &str = "127.0.0.1:0";
pub const NODE_DEFAULT_PEER_LISTEN: &str = "0.0.0.0:0";

mod state_file;
pub use state_file::write_resolved_state_file;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AeadSuite {
    Chacha20Poly1305,
    Aes256Gcm,
}

impl AeadSuite {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "chacha20poly1305" | "chacha20-poly1305" => Ok(Self::Chacha20Poly1305),
            "aes256gcm" | "aes-256-gcm" => Ok(Self::Aes256Gcm),
            _ => Err("aead must be chacha20poly1305 or aes256gcm".to_string()),
        }
    }

    pub fn suite_id(self) -> u16 {
        match self {
            Self::Chacha20Poly1305 => SECURE_CHACHA20POLY1305_SUITE_ID,
            Self::Aes256Gcm => SECURE_AES256GCM_SUITE_ID,
        }
    }

    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Chacha20Poly1305 => "chacha20poly1305",
            Self::Aes256Gcm => "aes256gcm",
        }
    }
}

#[derive(Clone)]
pub struct Options {
    pub mode: Mode,
    pub local_listen: String,
    pub peer_listen: String,
    pub state_file: Option<String>,
    pub server: String,
    pub token: String,
    pub pool: usize,
    pub bench_bytes: usize,
    pub target: String,
    pub connect_timeout_ms: u64,
    pub min_throughput_mib_s: u64,
    pub connections: usize,
    pub aead: AeadSuite,
    pub reverse_connect: bool,
    pub allow_pool_transit: bool,
    pub allow_bound_transit: bool,
    pub transit_lane_bindings_file: Option<String>,
    pub transit_max_frames_per_direction: u64,
    pub transit_max_bytes_per_direction: u64,
    pub transit_idle_timeout_ms: u64,
    pub transit_payload_bytes: usize,
    pub transit_packet_number: u64,
    pub transit_route_id: Option<u64>,
    pub transit_lane_index: Option<usize>,
    pub discovery_url: Option<String>,
    pub discovery_pubkey: Option<String>,
    pub discovery_keyring: Option<String>,
    pub mesh_namespace: String,
    pub mesh_self_node_id: String,
    pub mesh_policy_payload: String,
    pub lane_document_path: Option<String>,
    pub discovery_poll_interval_ms: u64,
    pub discovery_timeout_ms: u64,
}

pub(super) fn env_value(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn required_value(value: Option<String>, error: &str) -> Result<String, String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| error.to_string())
}

fn parse_pool(value: &str) -> Result<usize, String> {
    let pool = parse_positive_usize(value, "pool")?;
    if pool == 0 || pool > 128 {
        return Err("pool must be in 1..=128".to_string());
    }
    Ok(pool)
}

fn parse_positive_usize(value: &str, name: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} must be positive"));
    }
    Ok(parsed)
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err("expected true/false/1/0/yes/no".to_string()),
    }
}

pub(super) fn parse_positive_u64(value: &str, name: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} must be positive"));
    }
    Ok(parsed)
}

impl Options {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut mode: Option<Mode> = None;
        let mut local_listen = env_value("CHIMERA_PEER_EGRESS_LOCAL_LISTEN");
        let mut peer_listen = env_value("CHIMERA_PEER_EGRESS_PEER_LISTEN");
        let mut state_file = env_value("CHIMERA_PEER_EGRESS_STATE_FILE");
        let mut server = env_value("CHIMERA_PEER_EGRESS_SERVER");
        let mut token = env_value("CHIMERA_PEER_EGRESS_TOKEN").unwrap_or_default();
        let mut pool = env_value("CHIMERA_PEER_EGRESS_POOL")
            .map(|value| parse_pool(&value))
            .transpose()?
            .unwrap_or(8);
        let mut bench_bytes = env_value("CHIMERA_PEER_EGRESS_BENCH_BYTES")
            .map(|value| parse_positive_usize(&value, "bench-bytes"))
            .transpose()?
            .unwrap_or(64 * 1024 * 1024);
        let mut target = env_value("CHIMERA_PEER_EGRESS_TARGET");
        let mut connect_timeout_ms = env_value("CHIMERA_PEER_EGRESS_CONNECT_TIMEOUT_MS")
            .map(|value| parse_positive_u64(&value, "connect-timeout-ms"))
            .transpose()?
            .unwrap_or(3_000);
        let mut min_throughput_mib_s = env_value("CHIMERA_PEER_EGRESS_MIN_THROUGHPUT_MIB_S")
            .map(|value| parse_positive_u64(&value, "min-throughput-mib-s"))
            .transpose()?
            .unwrap_or(0);
        let mut connections = env_value("CHIMERA_PEER_EGRESS_CONNECTIONS")
            .map(|value| parse_pool(&value))
            .transpose()?
            .unwrap_or(1);
        let mut aead = env_value("CHIMERA_PEER_EGRESS_AEAD")
            .map(|value| AeadSuite::parse(&value))
            .transpose()?
            .unwrap_or(AeadSuite::Chacha20Poly1305);
        let mut reverse_connect = env_value("CHIMERA_PEER_EGRESS_REVERSE_CONNECT")
            .map(|value| parse_bool(&value))
            .transpose()?
            .unwrap_or(false);
        let mut allow_pool_transit = env_value("CHIMERA_PEER_EGRESS_ALLOW_POOL_TRANSIT")
            .map(|value| parse_bool(&value))
            .transpose()?
            .unwrap_or(false);
        let mut allow_bound_transit = env_value("CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT")
            .map(|value| parse_bool(&value))
            .transpose()?
            .unwrap_or(false);
        let mut transit_lane_bindings_file =
            env_value("CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE");
        let mut transit_relay_guard = TransitRelayGuardOptionValues::from_env()?;
        let mut transit_proof_args: Vec<(String, String)> = Vec::new();
        let mut discovery_url = env_value("CHIMERA_MESH_NODES_DISCOVERY_URL");
        let mut discovery_pubkey = env_value("CHIMERA_MESH_NODES_DISCOVERY_PUBKEY");
        let mut discovery_keyring = env_value("CHIMERA_MESH_NODES_DISCOVERY_KEYRING");
        let mut mesh_namespace = env_value("CHIMERA_MESH_NAMESPACE");
        let mut mesh_self_node_id = env_value("CHIMERA_MESH_SELF_NODE_ID");
        let mut mesh_policy_payload = env_value("CHIMERA_MESH_POLICY_PAYLOAD");
        let mut lane_document_path = env_value("CHIMERA_PEER_EGRESS_LANE_DOCUMENT_PATH");
        let mut discovery_poll_interval_ms = env_value("CHIMERA_MESH_DISCOVERY_POLL_INTERVAL_MS")
            .map(|value| parse_positive_u64(&value, "discovery-poll-interval-ms"))
            .transpose()?
            .unwrap_or(30_000);
        let mut discovery_timeout_ms = env_value("CHIMERA_MESH_DISCOVERY_TIMEOUT_MS")
            .map(|value| parse_positive_u64(&value, "discovery-timeout-ms"))
            .transpose()?
            .unwrap_or(5_000);
        let mut index = 0usize;
        while index < args.len() {
            let flag = args[index].as_str();
            let value = args
                .get(index + 1)
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag {
                "--mode" => {
                    mode = Some(parse_mode(value)?);
                }
                "--local-listen" => local_listen = Some(value.clone()),
                "--peer-listen" => peer_listen = Some(value.clone()),
                "--state-file" => state_file = Some(value.clone()),
                "--server" => server = Some(value.clone()),
                "--token" => token = value.clone(),
                "--pool" => pool = parse_pool(value)?,
                "--target" => target = Some(value.clone()),
                "--connect-timeout-ms" => {
                    connect_timeout_ms = parse_positive_u64(value, "connect-timeout-ms")?;
                }
                "--min-throughput-mib-s" => {
                    min_throughput_mib_s = parse_positive_u64(value, "min-throughput-mib-s")?;
                }
                "--connections" => {
                    connections = parse_pool(value)?;
                }
                "--aead" => {
                    aead = AeadSuite::parse(value)?;
                }
                "--reverse-connect" => {
                    reverse_connect = parse_bool(value)?;
                }
                "--allow-pool-transit" => {
                    allow_pool_transit = parse_bool(value)?;
                }
                "--allow-bound-transit" => {
                    allow_bound_transit = parse_bool(value)?;
                }
                "--transit-lane-bindings-file" => {
                    transit_lane_bindings_file = Some(value.clone());
                }
                "--discovery-url" => discovery_url = Some(value.clone()),
                "--discovery-pubkey" => discovery_pubkey = Some(value.clone()),
                "--discovery-keyring" => discovery_keyring = Some(value.clone()),
                "--mesh-namespace" => mesh_namespace = Some(value.clone()),
                "--mesh-self-node-id" => mesh_self_node_id = Some(value.clone()),
                "--mesh-policy-payload" => mesh_policy_payload = Some(value.clone()),
                "--lane-document-path" => lane_document_path = Some(value.clone()),
                "--discovery-poll-interval-ms" => {
                    discovery_poll_interval_ms =
                        parse_positive_u64(value, "discovery-poll-interval-ms")?;
                }
                "--discovery-timeout-ms" => {
                    discovery_timeout_ms = parse_positive_u64(value, "discovery-timeout-ms")?;
                }
                flag if transit_relay_guard.apply_flag(flag, value)? => {}
                flag if TransitProofOptions::is_flag(flag) => {
                    transit_proof_args.push((flag.to_string(), value.clone()));
                }
                "--bench-bytes" => {
                    bench_bytes = parse_positive_usize(value, "bench-bytes")?;
                }
                _ => return Err(format!("unknown flag: {flag}")),
            }
            index += 2;
        }
        let mode = mode.ok_or_else(|| "missing --mode".to_string())?;
        let mesh_namespace = mesh_namespace.unwrap_or_else(|| "chimera-mesh".to_string());
        let mesh_self_node_id = mesh_self_node_id
            .or_else(|| env_value("HOSTNAME"))
            .unwrap_or_else(|| "chimera-node".to_string());
        let mesh_policy_payload = mesh_policy_payload.unwrap_or_default();
        let discovery_configured = discovery_url.is_some();
        if discovery_configured {
            if discovery_pubkey.is_none() && discovery_keyring.is_none() {
                return Err(
                    "discovery requires --discovery-pubkey or --discovery-keyring".to_string(),
                );
            }
            if lane_document_path.is_none() {
                return Err("discovery requires --lane-document-path".to_string());
            }
            if mesh_policy_payload.is_empty() {
                return Err("discovery requires --mesh-policy-payload".to_string());
            }
            if !mesh_policy_payload.contains("mesh_route_binding_id") {
                return Err("mesh policy payload must contain mesh_route_binding_id for carrier lane bindings".to_string());
            }
        }
        let transit_proof = if matches!(mode, Mode::SealedTransitInject | Mode::BoundTransitInject)
        {
            let mut transit_proof = TransitProofOptions::from_env()?;
            for (flag, value) in &transit_proof_args {
                transit_proof.apply_flag(flag, value)?;
            }
            transit_proof
        } else if transit_proof_args.is_empty() {
            TransitProofOptions::default()
        } else {
            return Err("transit proof flags are only valid in transit inject modes".to_string());
        };
        let token_required = !matches!(mode, Mode::SealedTransitInject | Mode::BoundTransitInject);
        if token_required && token.is_empty() {
            return Err("token must be non-empty, <=256 bytes, and single-line".to_string());
        }
        if !token.is_empty() && (token.len() > MAX_TOKEN_LEN || token.contains('\n')) {
            return Err("token must be non-empty, <=256 bytes, and single-line".to_string());
        }
        let transit_relay_limits = transit_relay_guard.limits()?;
        let (local_listen, peer_listen, server) = match mode {
            Mode::Node => (
                local_listen.unwrap_or_else(|| NODE_DEFAULT_LOCAL_LISTEN.to_string()),
                peer_listen.unwrap_or_else(|| NODE_DEFAULT_PEER_LISTEN.to_string()),
                server.unwrap_or_default(),
            ),
            Mode::SideA => (
                required_value(
                    local_listen,
                    "side-a mode requires --local-listen or CHIMERA_PEER_EGRESS_LOCAL_LISTEN",
                )?,
                required_value(
                    peer_listen,
                    "side-a mode requires --peer-listen or CHIMERA_PEER_EGRESS_PEER_LISTEN",
                )?,
                server.unwrap_or_default(),
            ),
            Mode::SideB => (
                local_listen.unwrap_or_default(),
                peer_listen.unwrap_or_default(),
                required_value(
                    server,
                    "side-b mode requires --server or CHIMERA_PEER_EGRESS_SERVER",
                )?,
            ),
            Mode::Bench => (
                local_listen.unwrap_or_default(),
                peer_listen.unwrap_or_default(),
                server.unwrap_or_default(),
            ),
            Mode::Echo | Mode::DownloadEcho => (
                required_value(
                    local_listen,
                    "echo mode requires --local-listen or CHIMERA_PEER_EGRESS_LOCAL_LISTEN",
                )?,
                peer_listen.unwrap_or_default(),
                server.unwrap_or_default(),
            ),
            Mode::Probe | Mode::DownloadProbe => (
                local_listen.unwrap_or_default(),
                peer_listen.unwrap_or_default(),
                required_value(
                    server,
                    "probe mode requires --server or CHIMERA_PEER_EGRESS_SERVER",
                )?,
            ),
            Mode::SealedTransitInject | Mode::BoundTransitInject => (
                local_listen.unwrap_or_default(),
                peer_listen.unwrap_or_default(),
                required_value(
                    server,
                    "transit inject mode requires --server or CHIMERA_PEER_EGRESS_SERVER",
                )?,
            ),
        };
        let target = if matches!(mode, Mode::Probe | Mode::DownloadProbe) {
            required_value(
                target,
                "probe mode requires --target or CHIMERA_PEER_EGRESS_TARGET",
            )?
        } else {
            target.unwrap_or_default()
        };
        transit_proof.validate_for_mode(&mode)?;
        Ok(Self {
            mode,
            local_listen,
            peer_listen,
            state_file,
            server,
            token,
            pool,
            bench_bytes,
            target,
            connect_timeout_ms,
            min_throughput_mib_s,
            connections,
            aead,
            reverse_connect,
            allow_pool_transit,
            allow_bound_transit,
            transit_lane_bindings_file,
            transit_max_frames_per_direction: transit_relay_limits.max_frames_per_direction,
            transit_max_bytes_per_direction: transit_relay_limits.max_bytes_per_direction,
            transit_idle_timeout_ms: transit_relay_limits.idle_timeout_ms,
            transit_payload_bytes: transit_proof.payload_bytes,
            transit_packet_number: transit_proof.packet_number,
            transit_route_id: transit_proof.route_id,
            transit_lane_index: transit_proof.lane_index,
            discovery_url,
            discovery_pubkey,
            discovery_keyring,
            mesh_namespace,
            mesh_self_node_id,
            mesh_policy_payload,
            lane_document_path,
            discovery_poll_interval_ms,
            discovery_timeout_ms,
        })
    }

    pub fn transit_relay_limits(&self) -> TransitRelayLimits {
        TransitRelayLimits {
            max_frames_per_direction: self.transit_max_frames_per_direction,
            max_bytes_per_direction: self.transit_max_bytes_per_direction,
            idle_timeout_ms: self.transit_idle_timeout_ms,
        }
    }

    pub fn discovery_configured(&self) -> bool {
        self.discovery_url.is_some()
    }

    pub fn discovery_keyring_map(
        &self,
    ) -> Result<std::collections::BTreeMap<String, String>, String> {
        let mut keyring = std::collections::BTreeMap::new();
        if let Some(raw) = &self.discovery_keyring {
            for entry in raw.split(',').map(str::trim).filter(|v| !v.is_empty()) {
                let (key_id, pubkey) = entry
                    .split_once(':')
                    .ok_or_else(|| "discovery keyring entry must be key_id:base64".to_string())?;
                if key_id.trim().is_empty() || pubkey.trim().is_empty() {
                    return Err(
                        "discovery keyring entry must have non-empty key_id and pubkey".to_string(),
                    );
                }
                keyring.insert(key_id.trim().to_string(), pubkey.trim().to_string());
            }
        }
        if let Some(pubkey) = &self.discovery_pubkey {
            keyring.insert("default".to_string(), pubkey.clone());
        }
        if keyring.is_empty() {
            return Err("discovery keyring is required".to_string());
        }
        Ok(keyring)
    }
}

pub fn split_host_port(target: &str) -> Result<(String, u16), String> {
    let (host, port_raw) = target
        .rsplit_once(':')
        .ok_or_else(|| "target must be host:port".to_string())?;
    if host.trim().is_empty() {
        return Err("target host is empty".to_string());
    }
    if host.contains(',') {
        return Err("target host contains comma".to_string());
    }
    let port = port_raw
        .parse::<u16>()
        .map_err(|_| "target port is invalid".to_string())?;
    Ok((host.to_string(), port))
}

pub fn enforce_min_throughput(actual_mib_s: f64, min_mib_s: u64) -> Result<(), String> {
    if min_mib_s == 0 || actual_mib_s >= min_mib_s as f64 {
        return Ok(());
    }
    Err(format!(
        "throughput below gate: actual_mib_s={actual_mib_s:.2} min_mib_s={min_mib_s}"
    ))
}

#[cfg(test)]
#[path = "options_tests/mod.rs"]
mod options_tests;
