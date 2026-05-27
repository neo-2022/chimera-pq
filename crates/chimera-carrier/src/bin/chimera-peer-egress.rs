#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chimera_crypto::{
    SuiteId, TrafficSecret, TranscriptHash, X25519Secret, decrypt_aes256gcm_in_place,
    decrypt_chacha20poly1305_in_place, derive_hybrid_traffic_secrets, encrypt_aes256gcm_in_place,
    encrypt_chacha20poly1305_in_place, ml_kem_768_decapsulate, ml_kem_768_encapsulate,
    ml_kem_768_generate_keypair,
};
use sha2::{Digest, Sha256};
use socket2::SockRef;

const HANDSHAKE_MAGIC: &[u8] = b"CHIMERA-PEER-EGRESS/1\n";
const LOCAL_MAGIC: &[u8] = b"CHIMERA-LOCAL/1\n";
const MAX_TOKEN_LEN: usize = 256;
const SECURE_MAGIC: &[u8] = b"CHIMERA-PEER-SECURE/1\n";
const SECURE_NONCE_LEN: usize = 32;
const SECURE_CHACHA20POLY1305_SUITE_ID: u16 = 0xEE02;
const SECURE_AES256GCM_SUITE_ID: u16 = 0xEE03;
const SECURE_PLAINTEXT_CHUNK_LEN: usize = 1024 * 1024;
const SECURE_MAX_CIPHERTEXT_LEN: usize = SECURE_PLAINTEXT_CHUNK_LEN + 32;
const TCP_BUFFER_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    Vps,
    Laptop,
    Bench,
    Echo,
    Probe,
    DownloadEcho,
    DownloadProbe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    mode: Mode,
    local_listen: String,
    peer_listen: String,
    state_file: Option<String>,
    server: String,
    token: String,
    pool: usize,
    bench_bytes: usize,
    target: String,
    connect_timeout_ms: u64,
    min_throughput_mib_s: u64,
    connections: usize,
    aead: AeadSuite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AeadSuite {
    Chacha20Poly1305,
    Aes256Gcm,
}

impl AeadSuite {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "chacha20poly1305" | "chacha20-poly1305" => Ok(Self::Chacha20Poly1305),
            "aes256gcm" | "aes-256-gcm" => Ok(Self::Aes256Gcm),
            _ => Err("aead must be chacha20poly1305 or aes256gcm".to_string()),
        }
    }

    fn suite_id(self) -> u16 {
        match self {
            Self::Chacha20Poly1305 => SECURE_CHACHA20POLY1305_SUITE_ID,
            Self::Aes256Gcm => SECURE_AES256GCM_SUITE_ID,
        }
    }

    fn wire_name(self) -> &'static str {
        match self {
            Self::Chacha20Poly1305 => "chacha20poly1305",
            Self::Aes256Gcm => "aes256gcm",
        }
    }
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
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
        })
    }
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

fn parse_positive_u64(value: &str, name: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} must be positive"));
    }
    Ok(parsed)
}

#[derive(Debug, Default)]
struct PeerPool {
    peers: Mutex<VecDeque<SecurePeerStream>>,
    ready: Condvar,
}

impl PeerPool {
    fn push(&self, stream: SecurePeerStream) -> Result<(), String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        peers.push_back(stream);
        self.ready.notify_one();
        Ok(())
    }

    fn pop_wait(&self) -> Result<SecurePeerStream, String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        loop {
            if let Some(stream) = peers.pop_front() {
                return Ok(stream);
            }
            peers = self
                .ready
                .wait(peers)
                .map_err(|_| "peer pool wait poisoned".to_string())?;
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let options = match Options::parse(&args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(2);
        }
    };
    let result = match options.mode {
        Mode::Vps => run_vps(options),
        Mode::Laptop => run_laptop(options),
        Mode::Bench => run_bench(options),
        Mode::Echo => run_echo(options),
        Mode::Probe => run_probe(options),
        Mode::DownloadEcho => run_download_echo(options),
        Mode::DownloadProbe => run_download_probe(options),
    };
    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run_vps(options: Options) -> Result<(), String> {
    let peer_listener = TcpListener::bind(&options.peer_listen)
        .map_err(|error| format!("bind peer listener failed: {error}"))?;
    let local_listener = TcpListener::bind(&options.local_listen)
        .map_err(|error| format!("bind local listener failed: {error}"))?;
    let resolved_peer_listen = peer_listener
        .local_addr()
        .map_err(|error| format!("resolve peer listener addr failed: {error}"))?
        .to_string();
    let resolved_local_listen = local_listener
        .local_addr()
        .map_err(|error| format!("resolve local listener addr failed: {error}"))?
        .to_string();
    if let Some(state_file) = &options.state_file {
        if let Err(error) = write_resolved_state_file(
            state_file,
            &options.mode,
            &resolved_local_listen,
            &resolved_peer_listen,
        ) {
            eprintln!("event=peer_state_write_failed reason={error}");
        }
    }
    let pool = Arc::new(PeerPool::default());
    let token = options.token.clone();
    let aead = options.aead;
    let peer_pool = Arc::clone(&pool);
    thread::spawn(move || {
        for incoming in peer_listener.incoming() {
            let Ok(mut stream) = incoming else {
                continue;
            };
            if let Err(error) = tune_tcp(&stream) {
                eprintln!("event=peer_socket_tune_failed reason={error}");
            }
            match authenticate_peer(&mut stream, &token)
                .and_then(|_| establish_secure_peer_server(stream, &token, aead))
            {
                Ok(peer) => {
                    eprintln!("event=peer_authenticated");
                    let _ = peer_pool.push(peer);
                }
                Err(error) => {
                    eprintln!("event=peer_auth_failed reason={error}");
                }
            }
        }
    });
    println!(
        "chimera_peer_egress=vps_ready local={} peer={} resolved_local={} resolved_peer={}",
        options.local_listen,
        options.peer_listen,
        resolved_local_listen,
        resolved_peer_listen
    );
    for incoming in local_listener.incoming() {
        let Ok(local) = incoming else {
            continue;
        };
        eprintln!("event=local_ingress_accepted");
        let Ok(peer) = pool.pop_wait() else {
            continue;
        };
        eprintln!("event=local_ingress_paired_with_peer");
        thread::spawn(move || {
            let _ = handle_local_client(local, peer);
        });
    }
    Ok(())
}

fn run_laptop(options: Options) -> Result<(), String> {
    println!(
        "chimera_peer_egress=laptop_connecting server={} pool={}",
        options.server, options.pool
    );
    for _ in 0..options.pool {
        let worker = options.clone();
        thread::spawn(move || {
            loop {
                if let Err(error) = laptop_worker(&worker) {
                    eprintln!("worker_error={error}");
                    thread::sleep(Duration::from_secs(1));
                }
            }
        });
    }
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}

fn laptop_worker(options: &Options) -> Result<(), String> {
    let mut peer = connect_tcp(&options.server, options.connect_timeout_ms)
        .map_err(|error| format!("connect peer server failed: {error}"))?;
    tune_tcp(&peer)?;
    eprintln!("event=laptop_peer_connected");
    peer.write_all(HANDSHAKE_MAGIC)
        .map_err(|error| format!("write handshake failed: {error}"))?;
    peer.write_all(options.token.as_bytes())
        .and_then(|_| peer.write_all(b"\n"))
        .map_err(|error| format!("write token failed: {error}"))?;
    let mut peer = establish_secure_peer_client(peer, &options.token, options.aead)?;
    let request = peer.read_line(512)?;
    eprintln!("event=laptop_peer_request_received request={request}");
    let target_addr = parse_peer_connect_request(&request)?;
    eprintln!("event=laptop_target_connecting target={target_addr}");
    let target = connect_tcp(&target_addr, options.connect_timeout_ms)
        .map_err(|error| format!("connect target failed: {error}"))?;
    tune_tcp(&target)?;
    eprintln!("event=laptop_target_connected target={target_addr}");
    peer.write_line("OK")?;
    eprintln!("event=laptop_peer_connect_ack_sent target={target_addr}");
    pipe_secure_peer_with_plain(peer, target)
}

fn handle_local_client(mut local: TcpStream, mut peer: SecurePeerStream) -> Result<(), String> {
    tune_tcp(&local)?;
    let mut first = [0_u8; 1];
    local
        .read_exact(&mut first)
        .map_err(|error| format!("read local protocol byte failed: {error}"))?;
    let (destination, native_client) = if first[0] == LOCAL_MAGIC[0] {
        (read_native_connect_destination(&mut local, first[0])?, true)
    } else if first[0] == 5 {
        (
            read_socks5_connect_destination(&mut local, first[0])?,
            false,
        )
    } else {
        return Err("unsupported local ingress protocol".to_string());
    };
    eprintln!(
        "event=local_ingress_destination host={} port={} native_client={}",
        destination.host, destination.port, native_client
    );
    let request = format!("CONNECT {} {}", destination.host, destination.port);
    peer.write_line(&request)?;
    eprintln!("event=peer_connect_request_sent request={request}");
    let ack = peer.read_line(16)?;
    if ack != "OK" {
        return Err("peer connect failed".to_string());
    }
    eprintln!("event=peer_connect_ack_received");
    if native_client {
        local
            .write_all(b"OK\n")
            .map_err(|error| format!("write native local ack failed: {error}"))?;
    } else {
        write_socks5_success(&mut local)?;
    }
    pipe_plain_with_secure_peer(local, peer)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Destination {
    host: String,
    port: u16,
}

fn read_native_connect_destination(
    stream: &mut TcpStream,
    first_byte: u8,
) -> Result<Destination, String> {
    let mut rest = vec![0_u8; LOCAL_MAGIC.len() - 1];
    stream
        .read_exact(&mut rest)
        .map_err(|error| format!("read native local magic failed: {error}"))?;
    let mut magic = vec![first_byte];
    magic.extend(rest);
    if magic != LOCAL_MAGIC {
        return Err("bad native local magic".to_string());
    }
    let request = read_line_limited(stream, 512)?;
    let mut parts = request.split_whitespace();
    if parts.next() != Some("CONNECT") {
        return Err("native local request must be CONNECT".to_string());
    }
    let host = parts
        .next()
        .ok_or_else(|| "native local request missing host".to_string())?;
    let port = parts
        .next()
        .ok_or_else(|| "native local request missing port".to_string())?
        .parse::<u16>()
        .map_err(|_| "native local request has invalid port".to_string())?;
    if parts.next().is_some() || host.is_empty() || host.contains('\r') || host.contains('\n') {
        return Err("native local request is invalid".to_string());
    }
    Ok(Destination {
        host: host.to_string(),
        port,
    })
}

fn read_socks5_connect_destination(
    stream: &mut TcpStream,
    first_byte: u8,
) -> Result<Destination, String> {
    let mut greeting_tail = [0_u8; 1];
    stream
        .read_exact(&mut greeting_tail)
        .map_err(|error| format!("read socks greeting failed: {error}"))?;
    if first_byte != 5 {
        return Err("unsupported socks version".to_string());
    }
    let methods_len = greeting_tail[0] as usize;
    let mut methods = vec![0_u8; methods_len];
    stream
        .read_exact(&mut methods)
        .map_err(|error| format!("read socks methods failed: {error}"))?;
    stream
        .write_all(&[5, 0])
        .map_err(|error| format!("write socks method response failed: {error}"))?;

    let mut head = [0_u8; 4];
    stream
        .read_exact(&mut head)
        .map_err(|error| format!("read socks connect head failed: {error}"))?;
    if head[0] != 5 || head[1] != 1 {
        return Err("only socks5 CONNECT is supported".to_string());
    }
    let host = match head[3] {
        1 => {
            let mut octets = [0_u8; 4];
            stream
                .read_exact(&mut octets)
                .map_err(|error| format!("read ipv4 target failed: {error}"))?;
            Ipv4Addr::from(octets).to_string()
        }
        3 => {
            let mut len = [0_u8; 1];
            stream
                .read_exact(&mut len)
                .map_err(|error| format!("read domain length failed: {error}"))?;
            let mut raw = vec![0_u8; len[0] as usize];
            stream
                .read_exact(&mut raw)
                .map_err(|error| format!("read domain target failed: {error}"))?;
            String::from_utf8(raw).map_err(|_| "domain target is not utf-8".to_string())?
        }
        4 => {
            let mut octets = [0_u8; 16];
            stream
                .read_exact(&mut octets)
                .map_err(|error| format!("read ipv6 target failed: {error}"))?;
            Ipv6Addr::from(octets).to_string()
        }
        _ => return Err("unsupported socks address type".to_string()),
    };
    let mut port_raw = [0_u8; 2];
    stream
        .read_exact(&mut port_raw)
        .map_err(|error| format!("read socks target port failed: {error}"))?;
    let port = u16::from_be_bytes(port_raw);
    Ok(Destination { host, port })
}

fn write_socks5_success(stream: &mut TcpStream) -> Result<(), String> {
    stream
        .write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0])
        .map_err(|error| format!("write socks success failed: {error}"))
}

fn parse_peer_connect_request(line: &str) -> Result<String, String> {
    let mut parts = line.split_whitespace();
    let Some(kind) = parts.next() else {
        return Err("empty peer request".to_string());
    };
    if kind != "CONNECT" {
        return Err("unsupported peer request".to_string());
    }
    let host = parts
        .next()
        .ok_or_else(|| "peer request missing host".to_string())?;
    let port = parts
        .next()
        .ok_or_else(|| "peer request missing port".to_string())?
        .parse::<u16>()
        .map_err(|_| "peer request has invalid port".to_string())?;
    if parts.next().is_some() {
        return Err("peer request has trailing fields".to_string());
    }
    if host.is_empty() || host.contains('\n') || host.contains('\r') {
        return Err("peer request has invalid host".to_string());
    }
    Ok(format!("{host}:{port}"))
}

fn authenticate_peer(stream: &mut TcpStream, token: &str) -> Result<(), String> {
    let mut magic = vec![0_u8; HANDSHAKE_MAGIC.len()];
    stream
        .read_exact(&mut magic)
        .map_err(|error| format!("read magic failed: {error}"))?;
    if magic != HANDSHAKE_MAGIC {
        return Err("bad peer magic".to_string());
    }
    let line = read_line_limited(stream, MAX_TOKEN_LEN + 1)?;
    if line != token {
        return Err("bad peer token".to_string());
    }
    Ok(())
}

fn read_line_limited(stream: &mut TcpStream, max_len: usize) -> Result<String, String> {
    let mut out = Vec::new();
    let mut buf = [0_u8; 1];
    while out.len() <= max_len {
        stream
            .read_exact(&mut buf)
            .map_err(|error| format!("read line failed: {error}"))?;
        if buf[0] == b'\n' {
            return String::from_utf8(out).map_err(|_| "line is not utf-8".to_string());
        }
        out.push(buf[0]);
    }
    Err("line too long".to_string())
}

#[derive(Debug)]
struct SecurePeerStream {
    stream: TcpStream,
    send_secret: TrafficSecret,
    recv_secret: TrafficSecret,
    send_packet: u64,
    recv_packet: u64,
    aead: AeadSuite,
}

impl SecurePeerStream {
    fn write_line(&mut self, line: &str) -> Result<(), String> {
        let mut bytes = line.as_bytes().to_vec();
        bytes.push(b'\n');
        self.write_secure_payload(&bytes)
    }

    fn read_line(&mut self, max_len: usize) -> Result<String, String> {
        let payload = self.read_secure_payload()?;
        if payload.len() > max_len {
            return Err("secure line too long".to_string());
        }
        if payload.last() != Some(&b'\n') {
            return Err("secure line missing newline".to_string());
        }
        String::from_utf8(payload[..payload.len() - 1].to_vec())
            .map_err(|_| "secure line is not utf-8".to_string())
    }

    fn write_secure_payload(&mut self, plaintext: &[u8]) -> Result<(), String> {
        let packet = self.send_packet;
        self.send_packet = self.send_packet.saturating_add(1);
        let mut ciphertext = Vec::with_capacity(plaintext.len() + 16);
        ciphertext.extend_from_slice(plaintext);
        encrypt_secure_payload_in_place(
            self.aead,
            &self.send_secret,
            packet,
            b"peer-egress",
            &mut ciphertext,
        )
        .map_err(|error| format!("secure encrypt failed: {error}"))?;
        let len = u32::try_from(ciphertext.len())
            .map_err(|_| "secure ciphertext length overflow".to_string())?;
        self.stream
            .write_all(&packet.to_be_bytes())
            .and_then(|_| self.stream.write_all(&len.to_be_bytes()))
            .and_then(|_| self.stream.write_all(&ciphertext))
            .map_err(|error| format!("write secure frame failed: {error}"))
    }

    fn read_secure_payload(&mut self) -> Result<Vec<u8>, String> {
        let mut header = [0_u8; 12];
        self.stream
            .read_exact(&mut header)
            .map_err(|error| format!("read secure frame header failed: {error}"))?;
        let packet = u64::from_be_bytes(
            header[0..8]
                .try_into()
                .map_err(|_| "invalid secure packet field".to_string())?,
        );
        if packet != self.recv_packet {
            return Err("secure packet number mismatch".to_string());
        }
        self.recv_packet = self.recv_packet.saturating_add(1);
        let len = u32::from_be_bytes(
            header[8..12]
                .try_into()
                .map_err(|_| "invalid secure length field".to_string())?,
        ) as usize;
        if len == 0 || len > SECURE_MAX_CIPHERTEXT_LEN {
            return Err("secure ciphertext length invalid".to_string());
        }
        let mut ciphertext = vec![0_u8; len];
        self.stream
            .read_exact(&mut ciphertext)
            .map_err(|error| format!("read secure frame payload failed: {error}"))?;
        decrypt_secure_payload_in_place(
            self.aead,
            &self.recv_secret,
            packet,
            b"peer-egress",
            &mut ciphertext,
        )
        .map_err(|error| format!("secure decrypt failed: {error}"))?;
        Ok(ciphertext)
    }
}

fn encrypt_secure_payload_in_place(
    aead: AeadSuite,
    secret: &TrafficSecret,
    packet: u64,
    associated_data: &[u8],
    buffer: &mut Vec<u8>,
) -> chimera_core::ChimeraResult<()> {
    match aead {
        AeadSuite::Chacha20Poly1305 => {
            encrypt_chacha20poly1305_in_place(secret, packet, associated_data, buffer)
        }
        AeadSuite::Aes256Gcm => encrypt_aes256gcm_in_place(secret, packet, associated_data, buffer),
    }
}

fn decrypt_secure_payload_in_place(
    aead: AeadSuite,
    secret: &TrafficSecret,
    packet: u64,
    associated_data: &[u8],
    buffer: &mut Vec<u8>,
) -> chimera_core::ChimeraResult<()> {
    match aead {
        AeadSuite::Chacha20Poly1305 => {
            decrypt_chacha20poly1305_in_place(secret, packet, associated_data, buffer)
        }
        AeadSuite::Aes256Gcm => decrypt_aes256gcm_in_place(secret, packet, associated_data, buffer),
    }
}

fn establish_secure_peer_client(
    mut stream: TcpStream,
    token: &str,
    aead: AeadSuite,
) -> Result<SecurePeerStream, String> {
    let client_nonce = random_nonce()?;
    let client_x25519 = X25519Secret::from_private_bytes(random_nonce()?);
    let client_x25519_public = client_x25519.public_key_bytes();
    stream
        .write_all(SECURE_MAGIC)
        .and_then(|_| stream.write_all(&aead.suite_id().to_be_bytes()))
        .and_then(|_| stream.write_all(&client_nonce))
        .and_then(|_| stream.write_all(&client_x25519_public))
        .map_err(|error| format!("write secure client hello failed: {error}"))?;
    let mut magic = vec![0_u8; SECURE_MAGIC.len()];
    stream
        .read_exact(&mut magic)
        .map_err(|error| format!("read secure server hello magic failed: {error}"))?;
    if magic != SECURE_MAGIC {
        return Err("bad secure server magic".to_string());
    }
    let mut server_suite_raw = [0_u8; 2];
    stream
        .read_exact(&mut server_suite_raw)
        .map_err(|error| format!("read secure server suite failed: {error}"))?;
    let server_suite = u16::from_be_bytes(server_suite_raw);
    if server_suite != aead.suite_id() {
        return Err("secure server AEAD suite mismatch".to_string());
    }
    let mut server_nonce = [0_u8; SECURE_NONCE_LEN];
    stream
        .read_exact(&mut server_nonce)
        .map_err(|error| format!("read secure server nonce failed: {error}"))?;
    let mut server_x25519_public = [0_u8; 32];
    stream
        .read_exact(&mut server_x25519_public)
        .map_err(|error| format!("read secure server x25519 key failed: {error}"))?;
    let server_ml_kem_public = read_len_prefixed_vec(&mut stream, 4096)?;
    let (ml_kem_ciphertext, pq_shared_secret) = ml_kem_768_encapsulate(&server_ml_kem_public)
        .map_err(|error| {
            format!("ML-KEM-768 encapsulate failed during secure client handshake: {error}")
        })?;
    write_len_prefixed_vec(&mut stream, &ml_kem_ciphertext)?;
    let x25519_shared_secret = client_x25519.diffie_hellman(server_x25519_public);
    let (client_to_server, server_to_client) = derive_secure_peer_secrets(
        aead,
        token,
        &client_nonce,
        &server_nonce,
        &client_x25519_public,
        &server_x25519_public,
        &server_ml_kem_public,
        &ml_kem_ciphertext,
        x25519_shared_secret.as_bytes(),
        &pq_shared_secret,
    )?;
    Ok(SecurePeerStream {
        stream,
        send_secret: client_to_server,
        recv_secret: server_to_client,
        send_packet: 0,
        recv_packet: 0,
        aead,
    })
}

fn establish_secure_peer_server(
    mut stream: TcpStream,
    token: &str,
    aead: AeadSuite,
) -> Result<SecurePeerStream, String> {
    let mut magic = vec![0_u8; SECURE_MAGIC.len()];
    stream
        .read_exact(&mut magic)
        .map_err(|error| format!("read secure client hello magic failed: {error}"))?;
    if magic != SECURE_MAGIC {
        return Err("bad secure client magic".to_string());
    }
    let mut client_suite_raw = [0_u8; 2];
    stream
        .read_exact(&mut client_suite_raw)
        .map_err(|error| format!("read secure client suite failed: {error}"))?;
    let client_suite = u16::from_be_bytes(client_suite_raw);
    if client_suite != aead.suite_id() {
        return Err("secure client AEAD suite mismatch".to_string());
    }
    let mut client_nonce = [0_u8; SECURE_NONCE_LEN];
    stream
        .read_exact(&mut client_nonce)
        .map_err(|error| format!("read secure client nonce failed: {error}"))?;
    let mut client_x25519_public = [0_u8; 32];
    stream
        .read_exact(&mut client_x25519_public)
        .map_err(|error| format!("read secure client x25519 key failed: {error}"))?;
    let server_nonce = random_nonce()?;
    let server_x25519 = X25519Secret::from_private_bytes(random_nonce()?);
    let server_x25519_public = server_x25519.public_key_bytes();
    let (server_ml_kem_decapsulation_key, server_ml_kem_public) = ml_kem_768_generate_keypair();
    stream
        .write_all(SECURE_MAGIC)
        .and_then(|_| stream.write_all(&aead.suite_id().to_be_bytes()))
        .and_then(|_| stream.write_all(&server_nonce))
        .and_then(|_| stream.write_all(&server_x25519_public))
        .map_err(|error| format!("write secure server hello failed: {error}"))?;
    write_len_prefixed_vec(&mut stream, &server_ml_kem_public)?;
    let ml_kem_ciphertext = read_len_prefixed_vec(&mut stream, 4096)?;
    let pq_shared_secret =
        ml_kem_768_decapsulate(&server_ml_kem_decapsulation_key, &ml_kem_ciphertext).map_err(
            |error| {
                format!("ML-KEM-768 decapsulate failed during secure server handshake: {error}")
            },
        )?;
    let x25519_shared_secret = server_x25519.diffie_hellman(client_x25519_public);
    let (client_to_server, server_to_client) = derive_secure_peer_secrets(
        aead,
        token,
        &client_nonce,
        &server_nonce,
        &client_x25519_public,
        &server_x25519_public,
        &server_ml_kem_public,
        &ml_kem_ciphertext,
        x25519_shared_secret.as_bytes(),
        &pq_shared_secret,
    )?;
    Ok(SecurePeerStream {
        stream,
        send_secret: server_to_client,
        recv_secret: client_to_server,
        send_packet: 0,
        recv_packet: 0,
        aead,
    })
}

fn derive_secure_peer_secrets(
    aead: AeadSuite,
    token: &str,
    client_nonce: &[u8; SECURE_NONCE_LEN],
    server_nonce: &[u8; SECURE_NONCE_LEN],
    client_x25519_public: &[u8; 32],
    server_x25519_public: &[u8; 32],
    ml_kem_public: &[u8],
    ml_kem_ciphertext: &[u8],
    x25519_shared_secret: &[u8; 32],
    pq_shared_secret: &[u8; 32],
) -> Result<(TrafficSecret, TrafficSecret), String> {
    let token_digest = Sha256::digest(token.as_bytes());
    let transcript = TranscriptHash::from_messages(&[
        b"peer-egress-secure-v3-x25519-mlkem768-aead",
        aead.wire_name().as_bytes(),
        token_digest.as_ref(),
        client_nonce,
        server_nonce,
        client_x25519_public,
        server_x25519_public,
        ml_kem_public,
        ml_kem_ciphertext,
    ]);
    let secrets = derive_hybrid_traffic_secrets(
        SuiteId(aead.suite_id()),
        &transcript,
        x25519_shared_secret,
        pq_shared_secret,
    )
    .map_err(|error| format!("derive secure peer secrets failed: {error}"))?;
    Ok((
        secrets.client_to_gateway.clone(),
        secrets.gateway_to_client.clone(),
    ))
}

fn random_nonce() -> Result<[u8; SECURE_NONCE_LEN], String> {
    let mut nonce = [0_u8; SECURE_NONCE_LEN];
    let mut file =
        File::open("/dev/urandom").map_err(|error| format!("open urandom failed: {error}"))?;
    file.read_exact(&mut nonce)
        .map_err(|error| format!("read urandom failed: {error}"))?;
    Ok(nonce)
}

fn write_len_prefixed_vec(stream: &mut TcpStream, value: &[u8]) -> Result<(), String> {
    let len =
        u16::try_from(value.len()).map_err(|_| "length-prefixed value too large".to_string())?;
    if len == 0 {
        return Err("length-prefixed value is empty".to_string());
    }
    stream
        .write_all(&len.to_be_bytes())
        .and_then(|_| stream.write_all(value))
        .map_err(|error| format!("write length-prefixed value failed: {error}"))
}

fn read_len_prefixed_vec(stream: &mut TcpStream, max_len: usize) -> Result<Vec<u8>, String> {
    let mut len_raw = [0_u8; 2];
    stream
        .read_exact(&mut len_raw)
        .map_err(|error| format!("read length-prefixed length failed: {error}"))?;
    let len = u16::from_be_bytes(len_raw) as usize;
    if len == 0 || len > max_len {
        return Err("length-prefixed value length invalid".to_string());
    }
    let mut value = vec![0_u8; len];
    stream
        .read_exact(&mut value)
        .map_err(|error| format!("read length-prefixed value failed: {error}"))?;
    Ok(value)
}

fn pipe_plain_with_secure_peer(
    mut plain: TcpStream,
    mut peer: SecurePeerStream,
) -> Result<(), String> {
    let mut plain_read = plain
        .try_clone()
        .map_err(|error| format!("clone plain failed: {error}"))?;
    let mut peer_write = SecurePeerStream {
        stream: peer
            .stream
            .try_clone()
            .map_err(|error| format!("clone peer failed: {error}"))?,
        send_secret: peer.send_secret.clone(),
        recv_secret: peer.recv_secret.clone(),
        send_packet: peer.send_packet,
        recv_packet: peer.recv_packet,
        aead: peer.aead,
    };
    let a = thread::spawn(move || {
        let mut buf = vec![0_u8; SECURE_PLAINTEXT_CHUNK_LEN];
        loop {
            match plain_read.read(&mut buf) {
                Ok(0) => {
                    let _ = peer_write.write_secure_payload(&[]);
                    break;
                }
                Ok(n) => {
                    if peer_write.write_secure_payload(&buf[..n]).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    let b = thread::spawn(move || {
        loop {
            match peer.read_secure_payload() {
                Ok(payload) if payload.is_empty() => break,
                Ok(payload) => {
                    if plain.write_all(&payload).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = plain.shutdown(Shutdown::Write);
    });
    let _ = a.join();
    let _ = b.join();
    Ok(())
}

fn pipe_secure_peer_with_plain(peer: SecurePeerStream, plain: TcpStream) -> Result<(), String> {
    pipe_plain_with_secure_peer(plain, peer)
}

fn tune_tcp(stream: &TcpStream) -> Result<(), String> {
    stream
        .set_nodelay(true)
        .map_err(|error| format!("set TCP_NODELAY failed: {error}"))?;
    tune_tcp_buffers(stream);
    Ok(())
}

fn tune_tcp_buffers(stream: &TcpStream) {
    let socket = SockRef::from(stream);
    let _ = socket.set_recv_buffer_size(TCP_BUFFER_BYTES);
    let _ = socket.set_send_buffer_size(TCP_BUFFER_BYTES);
}

fn run_bench(options: Options) -> Result<(), String> {
    let token = if options.token.is_empty() {
        "bench-token".to_string()
    } else {
        options.token
    };
    let target_listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("bind bench target failed: {error}"))?;
    let target_addr = target_listener
        .local_addr()
        .map_err(|error| format!("read bench target addr failed: {error}"))?;
    thread::spawn(move || {
        for incoming in target_listener.incoming() {
            let Ok(mut stream) = incoming else {
                continue;
            };
            thread::spawn(move || {
                let mut total = 0usize;
                let mut buf = vec![0_u8; SECURE_PLAINTEXT_CHUNK_LEN];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => total += n,
                        Err(_) => return,
                    }
                }
                let _ = stream.write_all(format!("OK {total}\n").as_bytes());
            });
        }
    });

    let peer_listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("bind bench peer failed: {error}"))?;
    let local_listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("bind bench local failed: {error}"))?;
    let peer_addr = peer_listener
        .local_addr()
        .map_err(|error| format!("read bench peer addr failed: {error}"))?;
    let local_addr = local_listener
        .local_addr()
        .map_err(|error| format!("read bench local addr failed: {error}"))?;
    start_vps_runtime(peer_listener, local_listener, token.clone(), options.aead);

    let worker_options = Options {
        mode: Mode::Laptop,
        local_listen: String::new(),
        peer_listen: String::new(),
        state_file: None,
        server: peer_addr.to_string(),
        token,
        pool: options.pool,
        bench_bytes: options.bench_bytes,
        target: String::new(),
        connect_timeout_ms: options.connect_timeout_ms,
        min_throughput_mib_s: options.min_throughput_mib_s,
        connections: 1,
        aead: options.aead,
    };
    for _ in 0..options.pool {
        let worker = worker_options.clone();
        thread::spawn(move || {
            loop {
                if laptop_worker(&worker).is_err() {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        });
    }
    thread::sleep(Duration::from_millis(50));

    let mut local = TcpStream::connect(local_addr)
        .map_err(|error| format!("connect bench local ingress failed: {error}"))?;
    tune_tcp(&local)?;
    local
        .write_all(LOCAL_MAGIC)
        .and_then(|_| {
            local.write_all(
                format!("CONNECT {} {}\n", target_addr.ip(), target_addr.port()).as_bytes(),
            )
        })
        .map_err(|error| format!("write bench local request failed: {error}"))?;
    let ack = read_line_limited(&mut local, 16)?;
    if ack != "OK" {
        return Err("bench local connect failed".to_string());
    }
    let started = Instant::now();
    write_repeating_payload(&mut local, options.bench_bytes)?;
    local
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("shutdown bench writer failed: {error}"))?;
    let reply = read_line_limited(&mut local, 64)?;
    let elapsed = started.elapsed();
    let expected = format!("OK {}", options.bench_bytes);
    if reply != expected {
        return Err(format!(
            "bench byte mismatch: got {reply}, expected {expected}"
        ));
    }
    let seconds = elapsed.as_secs_f64().max(0.000_001);
    let mib = options.bench_bytes as f64 / 1024.0 / 1024.0;
    let throughput_mib_s = mib / seconds;
    enforce_min_throughput(throughput_mib_s, options.min_throughput_mib_s)?;
    println!(
        "chimera_peer_egress_bench=pass bytes={} elapsed_ms={} throughput_mib_s={:.2}",
        options.bench_bytes,
        elapsed.as_millis(),
        throughput_mib_s
    );
    Ok(())
}

fn run_echo(options: Options) -> Result<(), String> {
    let listener = TcpListener::bind(&options.local_listen)
        .map_err(|error| format!("bind echo listener failed: {error}"))?;
    println!(
        "chimera_peer_egress=echo_ready listen={}",
        options.local_listen
    );
    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else {
            continue;
        };
        if let Err(error) = tune_tcp(&stream) {
            eprintln!("event=echo_socket_tune_failed reason={error}");
        }
        thread::spawn(move || {
            let mut total = 0usize;
            let mut buf = vec![0_u8; SECURE_PLAINTEXT_CHUNK_LEN];
            loop {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => total += n,
                    Err(_) => return,
                }
            }
            let _ = stream.write_all(format!("OK {total}\n").as_bytes());
        });
    }
    Ok(())
}

fn run_download_echo(options: Options) -> Result<(), String> {
    let listener = TcpListener::bind(&options.local_listen)
        .map_err(|error| format!("bind download echo listener failed: {error}"))?;
    println!(
        "chimera_peer_egress=download_echo_ready listen={}",
        options.local_listen
    );
    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else {
            continue;
        };
        if let Err(error) = tune_tcp(&stream) {
            eprintln!("event=download_echo_socket_tune_failed reason={error}");
        }
        thread::spawn(move || {
            let Ok(request) = read_line_limited(&mut stream, 64) else {
                return;
            };
            let mut parts = request.split_whitespace();
            if parts.next() != Some("SEND") {
                return;
            }
            let Some(bytes_raw) = parts.next() else {
                return;
            };
            if parts.next().is_some() {
                return;
            }
            let Ok(bytes) = bytes_raw.parse::<usize>() else {
                return;
            };
            let _ = write_repeating_payload(&mut stream, bytes);
        });
    }
    Ok(())
}

fn run_probe(options: Options) -> Result<(), String> {
    let (target_host, target_port) = split_host_port(&options.target)?;
    if options.connections > 1 {
        return run_probe_parallel(options, target_host, target_port);
    }
    run_probe_connection(
        &options.server,
        &target_host,
        target_port,
        options.bench_bytes,
        options.connect_timeout_ms,
        options.min_throughput_mib_s,
        "chimera_peer_egress_probe",
    )
}

fn run_download_probe(options: Options) -> Result<(), String> {
    let (target_host, target_port) = split_host_port(&options.target)?;
    if options.connections > 1 {
        return run_download_probe_parallel(options, target_host, target_port);
    }
    run_download_probe_connection(
        &options.server,
        &target_host,
        target_port,
        options.bench_bytes,
        options.connect_timeout_ms,
        options.min_throughput_mib_s,
        "chimera_peer_egress_download_probe",
    )
}

fn write_resolved_state_file(
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

fn mode_name(mode: &Mode) -> &'static str {
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

fn run_download_probe_parallel(
    options: Options,
    target_host: String,
    target_port: u16,
) -> Result<(), String> {
    let started = Instant::now();
    let base = options.bench_bytes / options.connections;
    let remainder = options.bench_bytes % options.connections;
    let mut workers = Vec::with_capacity(options.connections);
    for index in 0..options.connections {
        let server = options.server.clone();
        let target_host = target_host.clone();
        let bytes = base + usize::from(index < remainder);
        let connect_timeout_ms = options.connect_timeout_ms;
        workers.push(thread::spawn(move || {
            run_download_probe_connection(
                &server,
                &target_host,
                target_port,
                bytes,
                connect_timeout_ms,
                0,
                "chimera_peer_egress_download_lane",
            )
        }));
    }
    for worker in workers {
        worker
            .join()
            .map_err(|_| "download probe worker panicked".to_string())??;
    }
    let elapsed = started.elapsed();
    let seconds = elapsed.as_secs_f64().max(0.000_001);
    let mib = options.bench_bytes as f64 / 1024.0 / 1024.0;
    let throughput_mib_s = mib / seconds;
    enforce_min_throughput(throughput_mib_s, options.min_throughput_mib_s)?;
    println!(
        "chimera_peer_egress_download_probe=pass bytes={} connections={} elapsed_ms={} throughput_mib_s={:.2}",
        options.bench_bytes,
        options.connections,
        elapsed.as_millis(),
        throughput_mib_s
    );
    Ok(())
}

fn run_download_probe_connection(
    server: &str,
    target_host: &str,
    target_port: u16,
    bench_bytes: usize,
    connect_timeout_ms: u64,
    min_throughput_mib_s: u64,
    event_name: &str,
) -> Result<(), String> {
    let mut local = connect_tcp(server, connect_timeout_ms)
        .map_err(|error| format!("connect native local ingress failed: {error}"))?;
    tune_tcp(&local)?;
    local
        .write_all(LOCAL_MAGIC)
        .and_then(|_| local.write_all(format!("CONNECT {target_host} {target_port}\n").as_bytes()))
        .map_err(|error| format!("write native download request failed: {error}"))?;
    let ack = read_line_limited(&mut local, 16)?;
    if ack != "OK" {
        return Err("native download connect failed".to_string());
    }
    let started = Instant::now();
    local
        .write_all(format!("SEND {bench_bytes}\n").as_bytes())
        .map_err(|error| format!("write download echo request failed: {error}"))?;
    read_exact_bytes(&mut local, bench_bytes)?;
    let elapsed = started.elapsed();
    let seconds = elapsed.as_secs_f64().max(0.000_001);
    let mib = bench_bytes as f64 / 1024.0 / 1024.0;
    let throughput_mib_s = mib / seconds;
    enforce_min_throughput(throughput_mib_s, min_throughput_mib_s)?;
    println!(
        "{event_name}=pass bytes={} elapsed_ms={} throughput_mib_s={:.2}",
        bench_bytes,
        elapsed.as_millis(),
        throughput_mib_s
    );
    Ok(())
}

fn run_probe_parallel(
    options: Options,
    target_host: String,
    target_port: u16,
) -> Result<(), String> {
    let started = Instant::now();
    let base = options.bench_bytes / options.connections;
    let remainder = options.bench_bytes % options.connections;
    let mut workers = Vec::with_capacity(options.connections);
    for index in 0..options.connections {
        let server = options.server.clone();
        let target_host = target_host.clone();
        let bytes = base + usize::from(index < remainder);
        let connect_timeout_ms = options.connect_timeout_ms;
        workers.push(thread::spawn(move || {
            run_probe_connection(
                &server,
                &target_host,
                target_port,
                bytes,
                connect_timeout_ms,
                0,
                "chimera_peer_egress_probe_lane",
            )
        }));
    }
    for worker in workers {
        worker
            .join()
            .map_err(|_| "probe worker panicked".to_string())??;
    }
    let elapsed = started.elapsed();
    let seconds = elapsed.as_secs_f64().max(0.000_001);
    let mib = options.bench_bytes as f64 / 1024.0 / 1024.0;
    let throughput_mib_s = mib / seconds;
    enforce_min_throughput(throughput_mib_s, options.min_throughput_mib_s)?;
    println!(
        "chimera_peer_egress_probe=pass bytes={} connections={} elapsed_ms={} throughput_mib_s={:.2}",
        options.bench_bytes,
        options.connections,
        elapsed.as_millis(),
        throughput_mib_s
    );
    Ok(())
}

fn run_probe_connection(
    server: &str,
    target_host: &str,
    target_port: u16,
    bench_bytes: usize,
    connect_timeout_ms: u64,
    min_throughput_mib_s: u64,
    event_name: &str,
) -> Result<(), String> {
    let mut local = connect_tcp(server, connect_timeout_ms)
        .map_err(|error| format!("connect native local ingress failed: {error}"))?;
    tune_tcp(&local)?;
    local
        .write_all(LOCAL_MAGIC)
        .and_then(|_| local.write_all(format!("CONNECT {target_host} {target_port}\n").as_bytes()))
        .map_err(|error| format!("write native probe request failed: {error}"))?;
    let ack = read_line_limited(&mut local, 16)?;
    if ack != "OK" {
        return Err("native probe connect failed".to_string());
    }
    let started = Instant::now();
    write_repeating_payload(&mut local, bench_bytes)?;
    local
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("shutdown native probe writer failed: {error}"))?;
    let reply = read_line_limited(&mut local, 64)?;
    let elapsed = started.elapsed();
    let expected = format!("OK {bench_bytes}");
    if reply != expected {
        return Err(format!(
            "native probe byte mismatch: got {reply}, expected {expected}"
        ));
    }
    let seconds = elapsed.as_secs_f64().max(0.000_001);
    let mib = bench_bytes as f64 / 1024.0 / 1024.0;
    let throughput_mib_s = mib / seconds;
    enforce_min_throughput(throughput_mib_s, min_throughput_mib_s)?;
    println!(
        "{event_name}=pass bytes={} elapsed_ms={} throughput_mib_s={:.2}",
        bench_bytes,
        elapsed.as_millis(),
        throughput_mib_s
    );
    Ok(())
}

fn enforce_min_throughput(actual_mib_s: f64, min_mib_s: u64) -> Result<(), String> {
    if min_mib_s == 0 || actual_mib_s >= min_mib_s as f64 {
        return Ok(());
    }
    Err(format!(
        "throughput below gate: actual_mib_s={actual_mib_s:.2} min_mib_s={min_mib_s}"
    ))
}

fn split_host_port(target: &str) -> Result<(String, u16), String> {
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

fn connect_tcp(target: &str, timeout_ms: u64) -> Result<TcpStream, String> {
    let timeout = Duration::from_millis(timeout_ms);
    let addrs: Vec<SocketAddr> = target
        .to_socket_addrs()
        .map_err(|error| format!("resolve target failed: {error}"))?
        .collect();
    if addrs.is_empty() {
        return Err("target resolved to no socket addresses".to_string());
    }
    let mut last_error = String::new();
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = format!("{addr}: {error}"),
        }
    }
    Err(last_error)
}

fn start_vps_runtime(
    peer_listener: TcpListener,
    local_listener: TcpListener,
    token: String,
    aead: AeadSuite,
) {
    let pool = Arc::new(PeerPool::default());
    let peer_pool = Arc::clone(&pool);
    thread::spawn(move || {
        for incoming in peer_listener.incoming() {
            let Ok(mut stream) = incoming else {
                continue;
            };
            match tune_tcp(&stream)
                .and_then(|_| authenticate_peer(&mut stream, &token))
                .and_then(|_| establish_secure_peer_server(stream, &token, aead))
            {
                Ok(peer) => {
                    eprintln!("event=bench_peer_authenticated");
                    let _ = peer_pool.push(peer);
                }
                Err(error) => {
                    eprintln!("event=bench_peer_auth_failed reason={error}");
                }
            }
        }
    });
    thread::spawn(move || {
        for incoming in local_listener.incoming() {
            let Ok(local) = incoming else {
                continue;
            };
            eprintln!("event=bench_local_ingress_accepted");
            let Ok(peer) = pool.pop_wait() else {
                continue;
            };
            eprintln!("event=bench_local_ingress_paired_with_peer");
            thread::spawn(move || {
                let _ = handle_local_client(local, peer);
            });
        }
    });
}

fn write_repeating_payload(stream: &mut TcpStream, bytes: usize) -> Result<(), String> {
    let chunk = vec![0xA5_u8; SECURE_PLAINTEXT_CHUNK_LEN];
    let mut remaining = bytes;
    while remaining > 0 {
        let n = remaining.min(chunk.len());
        stream
            .write_all(&chunk[..n])
            .map_err(|error| format!("write bench payload failed: {error}"))?;
        remaining -= n;
    }
    Ok(())
}

fn read_exact_bytes(stream: &mut TcpStream, bytes: usize) -> Result<(), String> {
    let mut buf = vec![0_u8; SECURE_PLAINTEXT_CHUNK_LEN];
    let mut remaining = bytes;
    while remaining > 0 {
        let n = remaining.min(buf.len());
        stream
            .read_exact(&mut buf[..n])
            .map_err(|error| format!("read download payload failed: {error}"))?;
        remaining -= n;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Mode, Options};

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
    fn parse_peer_connect_request_accepts_host_port() {
        let target = super::parse_peer_connect_request("CONNECT example.org 443")
            .unwrap_or_else(|error| unreachable!("request must parse: {error}"));
        assert_eq!(target, "example.org:443");
    }

    #[test]
    fn native_local_request_rejects_bad_shape() {
        let request = super::parse_peer_connect_request("GET example.org 443");
        assert!(request.is_err());
    }

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
        assert_eq!(parsed.aead, super::AeadSuite::Aes256Gcm);

        let mut bad = args;
        bad[5] = "weak".to_string();
        assert!(Options::parse(&bad).is_err());
    }

    #[test]
    fn split_host_port_accepts_valid_target() {
        let parsed = super::split_host_port("node.example.invalid:443")
            .unwrap_or_else(|error| unreachable!("target should parse: {error}"));
        assert_eq!(parsed, ("node.example.invalid".to_string(), 443));
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
    fn throughput_gate_rejects_slow_path() {
        assert!(super::enforce_min_throughput(99.9, 100).is_err());
        assert!(super::enforce_min_throughput(100.0, 100).is_ok());
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
}
