use std::env;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Vps,
    Laptop,
    Bench,
    Echo,
    Probe,
    DownloadEcho,
    DownloadProbe,
}

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

#[derive(Debug, Clone)]
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
}

fn env_value(name: &str) -> Option<String> {
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

fn parse_positive_u64(value: &str, name: &str) -> Result<u64, String> {
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
        let mut index = 0usize;
        while index < args.len() {
            let flag = args[index].as_str();
            let value = args
                .get(index + 1)
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag {
                "--mode" => {
                    mode = Some(match value.as_str() {
                        "vps" => Mode::Vps,
                        "laptop" => Mode::Laptop,
                        "bench" => Mode::Bench,
                        "echo" => Mode::Echo,
                        "probe" => Mode::Probe,
                        "download-echo" => Mode::DownloadEcho,
                        "download-probe" => Mode::DownloadProbe,
                        _ => {
                            return Err(
                                "mode must be vps, laptop, bench, echo, probe, download-echo, or download-probe".to_string()
                            );
                        }
                    });
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
                "--bench-bytes" => {
                    bench_bytes = parse_positive_usize(value, "bench-bytes")?;
                }
                _ => return Err(format!("unknown flag: {flag}")),
            }
            index += 2;
        }
        if token.is_empty() || token.len() > MAX_TOKEN_LEN || token.contains('\n') {
            return Err("token must be non-empty, <=256 bytes, and single-line".to_string());
        }
        let mode = mode.ok_or_else(|| "missing --mode".to_string())?;
        let (local_listen, peer_listen, server) = match mode {
            Mode::Vps => (
                required_value(
                    local_listen,
                    "vps mode requires --local-listen or CHIMERA_PEER_EGRESS_LOCAL_LISTEN",
                )?,
                required_value(
                    peer_listen,
                    "vps mode requires --peer-listen or CHIMERA_PEER_EGRESS_PEER_LISTEN",
                )?,
                server.unwrap_or_default(),
            ),
            Mode::Laptop => (
                local_listen.unwrap_or_default(),
                peer_listen.unwrap_or_default(),
                required_value(
                    server,
                    "laptop mode requires --server or CHIMERA_PEER_EGRESS_SERVER",
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
        };
        let target = if matches!(mode, Mode::Probe | Mode::DownloadProbe) {
            required_value(
                target,
                "probe mode requires --target or CHIMERA_PEER_EGRESS_TARGET",
            )?
        } else {
            target.unwrap_or_default()
        };
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
        })
    }
}

pub fn mode_name(mode: &Mode) -> &'static str {
    match mode {
        Mode::Vps => "vps",
        Mode::Laptop => "laptop",
        Mode::Bench => "bench",
        Mode::Echo => "echo",
        Mode::Probe => "probe",
        Mode::DownloadEcho => "download-echo",
        Mode::DownloadProbe => "download-probe",
    }
}

pub fn split_host_port(target: &str) -> Result<(String, u16), String> {
    let (host, port_raw) = target
        .rsplit_once(':')
        .ok_or_else(|| "target must be host:port".to_string())?;
    if host.trim().is_empty() {
        return Err("target host is empty".to_string());
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

pub fn write_resolved_state_file(
    state_file: &str,
    mode: &Mode,
    resolved_local_listen: &str,
    resolved_peer_listen: &str,
) -> Result<(), String> {
    let contents = format!(
        "mode={}\nresolved_local_listen={}\nresolved_peer_listen={}\n",
        mode_name(mode),
        resolved_local_listen,
        resolved_peer_listen
    );
    std::fs::write(state_file, contents)
        .map_err(|error| format!("write state file failed: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rejects_empty_token() {
        let args = vec![
            "--mode".to_string(),
            "vps".to_string(),
            "--local-listen".to_string(),
            "127.0.0.1:0".to_string(),
            "--peer-listen".to_string(),
            "127.0.0.1:0".to_string(),
            "--token".to_string(),
            String::new(),
        ];
        assert!(Options::parse(&args).is_err());
    }

    #[test]
    fn parse_vps_requires_explicit_listeners() {
        let args = vec![
            "--mode".to_string(),
            "vps".to_string(),
            "--token".to_string(),
            "abc".to_string(),
        ];
        assert!(Options::parse(&args).is_err());
    }

    #[test]
    fn parse_laptop_options() {
        let args = vec![
            "--mode".to_string(),
            "laptop".to_string(),
            "--server".to_string(),
            "mesh-node.example.invalid:443".to_string(),
            "--token".to_string(),
            "abc".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.mode, Mode::Laptop);
        assert_eq!(parsed.pool, 8);
    }

    #[test]
    fn parse_bench_options() {
        let args = vec![
            "--mode".to_string(),
            "bench".to_string(),
            "--token".to_string(),
            "abc".to_string(),
            "--bench-bytes".to_string(),
            "1024".to_string(),
            "--min-throughput-mib-s".to_string(),
            "100".to_string(),
            "--connections".to_string(),
            "4".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.mode, Mode::Bench);
        assert_eq!(parsed.bench_bytes, 1024);
        assert_eq!(parsed.min_throughput_mib_s, 100);
        assert_eq!(parsed.connections, 4);
    }

    #[test]
    fn parse_probe_requires_target() {
        let args = vec![
            "--mode".to_string(),
            "probe".to_string(),
            "--server".to_string(),
            "127.0.0.1:1".to_string(),
            "--token".to_string(),
            "abc".to_string(),
        ];
        assert!(Options::parse(&args).is_err());
    }

    #[test]
    fn parse_download_probe_options() {
        let args = vec![
            "--mode".to_string(),
            "download-probe".to_string(),
            "--server".to_string(),
            "127.0.0.1:1".to_string(),
            "--target".to_string(),
            "node.example.invalid:443".to_string(),
            "--token".to_string(),
            "abc".to_string(),
            "--connections".to_string(),
            "2".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.mode, Mode::DownloadProbe);
        assert_eq!(parsed.connections, 2);
    }

    #[test]
    fn parse_aead_options() {
        let args = vec![
            "--mode".to_string(),
            "bench".to_string(),
            "--token".to_string(),
            "abc".to_string(),
            "--aead".to_string(),
            "aes256gcm".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.aead, AeadSuite::Aes256Gcm);

        let mut bad = args;
        bad[5] = "weak".to_string();
        assert!(Options::parse(&bad).is_err());
    }

    #[test]
    fn parse_rejects_zero_connect_timeout() {
        let args = vec![
            "--mode".to_string(),
            "bench".to_string(),
            "--token".to_string(),
            "abc".to_string(),
            "--connect-timeout-ms".to_string(),
            "0".to_string(),
        ];
        assert!(Options::parse(&args).is_err());
    }

    #[test]
    fn parse_rejects_zero_connections() {
        let args = vec![
            "--mode".to_string(),
            "bench".to_string(),
            "--token".to_string(),
            "abc".to_string(),
            "--connections".to_string(),
            "0".to_string(),
        ];
        assert!(Options::parse(&args).is_err());
    }

    #[test]
    fn split_host_port_accepts_valid_target() {
        let parsed = split_host_port("node.example.invalid:443")
            .unwrap_or_else(|error| unreachable!("target should parse: {error}"));
        assert_eq!(parsed, ("node.example.invalid".to_string(), 443));
    }

    #[test]
    fn throughput_gate_rejects_slow_path() {
        assert!(enforce_min_throughput(99.9, 100).is_err());
        assert!(enforce_min_throughput(100.0, 100).is_ok());
    }
}
