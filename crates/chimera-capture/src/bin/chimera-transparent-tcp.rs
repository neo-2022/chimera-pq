#![forbid(unsafe_code)]

use std::env;
use std::io::{ErrorKind, Read, Write};
use std::net::{
    Ipv4Addr, Shutdown, SocketAddr, SocketAddrV4, TcpListener, TcpStream, ToSocketAddrs,
};

use nix::sys::socket::{getsockopt, sockopt};
use socket2::SockRef;
use std::thread;
use std::time::{Duration, Instant};

use chimera_capture::detect_forbidden_manual_proxy_protocol;

const LOCAL_MAGIC: &[u8] = b"CHIMERA-LOCAL/1\n";
const MAX_INITIAL_BYTES: usize = 128 * 1024;
const COPY_BUFFER_BYTES: usize = 1024 * 1024;
const TCP_BUFFER_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
enum DirectMode {
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    listen: String,
    transit_local: String,
    transit_fallback: Option<String>,
    direct_mode: DirectMode,
    direct_timeout_ms: u64,
    initial_read_timeout_ms: u64,
    connect_retry_count: usize,
    connect_retry_delay_ms: u64,
    #[cfg(test)]
    static_destination: Option<String>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut listen = env_value("CHIMERA_TRANSPARENT_TCP_LISTEN");
        let mut transit_local = env_value("CHIMERA_TRANSPARENT_TCP_TRANSIT_LOCAL")
            .or_else(|| env_value("CHIMERA_TRANSPARENT_TCP_GATEWAY_LOCAL"));
        let mut transit_fallback = env_value("CHIMERA_TRANSPARENT_TCP_TRANSIT_FALLBACK")
            .or_else(|| env_value("CHIMERA_TRANSPARENT_TCP_GATEWAY_FALLBACK"));
        let mut direct_mode = env_value("CHIMERA_TRANSPARENT_TCP_DIRECT_MODE")
            .map(|value| parse_direct_mode(&value))
            .transpose()?
            .unwrap_or(DirectMode::Disabled);
        let mut direct_timeout_ms = env_value("CHIMERA_TRANSPARENT_TCP_DIRECT_TIMEOUT_MS")
            .map(|value| parse_positive_u64(&value, "direct-timeout-ms"))
            .transpose()?
            .unwrap_or(1200);
        let mut initial_read_timeout_ms =
            env_value("CHIMERA_TRANSPARENT_TCP_INITIAL_READ_TIMEOUT_MS")
                .map(|value| parse_positive_u64(&value, "initial-read-timeout-ms"))
                .transpose()?
                .unwrap_or(500);
        let mut connect_retry_count = env_value("CHIMERA_TRANSPARENT_TCP_CONNECT_RETRY_COUNT")
            .map(|value| parse_non_negative_usize(&value, "connect-retry-count"))
            .transpose()?
            .unwrap_or(2);
        let mut connect_retry_delay_ms = env_value("CHIMERA_TRANSPARENT_TCP_CONNECT_RETRY_DELAY_MS")
            .map(|value| parse_positive_u64(&value, "connect-retry-delay-ms"))
            .transpose()?
            .unwrap_or(150);

        let mut index = 0usize;
        while index < args.len() {
            let flag = args[index].as_str();
            let value = args
                .get(index + 1)
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag {
                "--listen" => listen = Some(value.clone()),
                "--transit-local" | "--gateway-local" => transit_local = Some(value.clone()),
                "--transit-fallback" | "--gateway-fallback" => {
                    transit_fallback = Some(value.clone());
                }
                "--direct-mode" => direct_mode = parse_direct_mode(value)?,
                "--direct-timeout-ms" => {
                    direct_timeout_ms = parse_positive_u64(value, "direct-timeout-ms")?;
                }
                "--initial-read-timeout-ms" => {
                    initial_read_timeout_ms =
                        parse_positive_u64(value, "initial-read-timeout-ms")?;
                }
                "--connect-retry-count" => {
                    connect_retry_count = parse_non_negative_usize(value, "connect-retry-count")?;
                }
                "--connect-retry-delay-ms" => {
                    connect_retry_delay_ms =
                        parse_positive_u64(value, "connect-retry-delay-ms")?;
                }
                _ => return Err(format!("unknown flag: {flag}")),
            }
            index += 2;
        }

        Ok(Self {
            listen: required_value(listen, "missing --listen or CHIMERA_TRANSPARENT_TCP_LISTEN")?,
            transit_local: required_value(
                transit_local,
                "missing --transit-local/--gateway-local or CHIMERA_TRANSPARENT_TCP_TRANSIT_LOCAL",
            )?,
            transit_fallback,
            direct_mode,
            direct_timeout_ms,
            initial_read_timeout_ms,
            connect_retry_count,
            connect_retry_delay_ms,
            #[cfg(test)]
            static_destination: None,
        })
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
    if let Err(error) = run(options) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run(options: Options) -> Result<(), String> {
    let listener = TcpListener::bind(&options.listen)
        .map_err(|error| format!("bind transparent listener failed: {error}"))?;
    println!("chimera_transparent_tcp=ready listen={}", options.listen);
    for incoming in listener.incoming() {
        let Ok(client) = incoming else {
            continue;
        };
        let worker = options.clone();
        thread::spawn(move || {
            if let Err(error) = handle_client(client, &worker) {
                eprintln!("event=transparent_flow_error reason={error}");
            }
        });
    }
    Ok(())
}

fn handle_client(mut client: TcpStream, options: &Options) -> Result<(), String> {
    tune_tcp(&client)?;
    let initial = read_initial_bytes(&mut client, options.initial_read_timeout_ms)?;
    let destination = resolve_destination(&client, &initial, options)?;
    eprintln!("event=transparent_flow_accepted destination_state=resolved");
    match options.direct_mode {
        DirectMode::Disabled => {
            let transit = connect_transit_with_retry(
                &options.transit_local,
                options.transit_fallback.as_deref(),
                &destination,
                &initial,
                options.direct_timeout_ms,
                options.connect_retry_count,
                options.connect_retry_delay_ms,
            )?;
            eprintln!(
                "event=transparent_route_selected route=transit reason=direct_mode_disabled destination_state=resolved"
            );
            relay_plain(client, transit)
        }
    }
}

fn connect_transit_with_retry(
    transit_local: &str,
    transit_fallback: Option<&str>,
    destination: &SocketAddr,
    initial: &[u8],
    timeout_ms: u64,
    retry_count: usize,
    retry_delay_ms: u64,
) -> Result<TcpStream, String> {
    let mut last_error = String::new();
    for attempt in 0..=retry_count {
        match connect_transit_with_fallback(
            transit_local,
            transit_fallback,
            destination,
            initial,
            timeout_ms,
        ) {
            Ok(stream) => return Ok(stream),
            Err(error) => {
                last_error = error;
                eprintln!(
                    "event=transparent_transit_retry attempt={attempt} max={retry_count} retry_delay_ms={retry_delay_ms} reason=\"{last_error}\""
                );
                if attempt < retry_count {
                    thread::sleep(Duration::from_millis(retry_delay_ms));
                }
            }
        }
    }
    Err(format!(
        "transparent_transit_retry_exhausted attempts={} last_reason=\"{}\"",
        retry_count + 1,
        last_error
    ))
}

fn connect_transit_with_fallback(
    transit_local: &str,
    transit_fallback: Option<&str>,
    destination: &SocketAddr,
    initial: &[u8],
    timeout_ms: u64,
) -> Result<TcpStream, String> {
    match connect_transit(transit_local, destination, initial, timeout_ms) {
        Ok(stream) => Ok(stream),
        Err(err) => match transit_fallback {
            Some(fallback) => {
                eprintln!(
                    "event=transit_fallback_trying fallback_state=configured reason=\"{err}\""
                );
                connect_transit(fallback, destination, initial, timeout_ms)
            }
            None => Err(err),
        },
    }
}

fn connect_transit(
    transit_local: &str,
    destination: &SocketAddr,
    initial: &[u8],
    timeout_ms: u64,
) -> Result<TcpStream, String> {
    let mut transit = connect_tcp(transit_local, timeout_ms)
        .map_err(|error| format!("transit connect failed: {error}"))?;
    tune_tcp(&transit)?;
    let host = destination.ip().to_string();
    transit
        .write_all(LOCAL_MAGIC)
        .and_then(|_| {
            transit.write_all(format!("CONNECT {host} {}\n", destination.port()).as_bytes())
        })
        .map_err(|error| format!("transit connect request write failed: {error}"))?;
    let ack = read_line_limited(&mut transit, 16)?;
    if ack != "OK" {
        return Err("transit connect request rejected".to_string());
    }
    if !initial.is_empty() {
        transit
            .write_all(initial)
            .map_err(|error| format!("transit initial write failed: {error}"))?;
    }
    Ok(transit)
}

fn relay_plain(left: TcpStream, right: TcpStream) -> Result<(), String> {
    let mut left_read = left
        .try_clone()
        .map_err(|error| format!("clone left stream failed: {error}"))?;
    let mut right_write = right
        .try_clone()
        .map_err(|error| format!("clone right stream failed: {error}"))?;
    let mut right_read = right;
    let mut left_write = left;

    let a = thread::spawn(move || copy_until_eof(&mut left_read, &mut right_write));
    let b = thread::spawn(move || copy_until_eof(&mut right_read, &mut left_write));
    let _ = a.join().map_err(|_| "left relay panicked".to_string())?;
    let _ = b.join().map_err(|_| "right relay panicked".to_string())?;
    Ok(())
}

fn copy_until_eof(reader: &mut TcpStream, writer: &mut TcpStream) -> Result<(), String> {
    let mut buf = vec![0_u8; COPY_BUFFER_BYTES];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => {
                let _ = writer.shutdown(Shutdown::Write);
                return Ok(());
            }
            Ok(n) => writer
                .write_all(&buf[..n])
                .map_err(|error| format!("relay write failed: {error}"))?,
            Err(error) => return Err(format!("relay read failed: {error}")),
        }
    }
}

fn read_initial_bytes(client: &mut TcpStream, timeout_ms: u64) -> Result<Vec<u8>, String> {
    client
        .set_read_timeout(Some(Duration::from_millis(timeout_ms)))
        .map_err(|error| format!("set client read timeout failed: {error}"))?;
    let mut buf = vec![0_u8; MAX_INITIAL_BYTES];
    let result = match client.read(&mut buf) {
        Ok(0) => Ok(Vec::new()),
        Ok(n) => {
            buf.truncate(n);
            Ok(buf)
        }
        Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
            Ok(Vec::new())
        }
        Err(error) => Err(format!("read client initial bytes failed: {error}")),
    };
    client
        .set_read_timeout(None)
        .map_err(|error| format!("clear client read timeout failed: {error}"))?;
    result
}

fn original_destination(stream: &TcpStream) -> Result<SocketAddr, String> {
    let addr = getsockopt(stream, sockopt::OriginalDst)
        .map_err(|error| format!("SO_ORIGINAL_DST failed: {error}"))?;
    let ip = Ipv4Addr::from(addr.sin_addr.s_addr.to_ne_bytes());
    let port = u16::from_be(addr.sin_port);
    Ok(SocketAddr::V4(SocketAddrV4::new(ip, port)))
}

fn resolve_destination(
    client: &TcpStream,
    initial: &[u8],
    _options: &Options,
) -> Result<SocketAddr, String> {
    #[cfg(test)]
    if let Some(dest) = _options.static_destination.as_deref() {
        return parse_socket_addr(dest);
    }
    if let Some(protocol) = detect_forbidden_manual_proxy_protocol(initial) {
        return Err(format!(
            "manual_proxy_ingress_forbidden protocol={} reason=transparent_mesh_datapath_requires_os_original_destination",
            protocol.as_str()
        ));
    }
    original_destination(client)
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
    let started = Instant::now();
    let mut last_error = String::new();
    for addr in addrs {
        let elapsed = started.elapsed();
        let remaining = timeout
            .checked_sub(elapsed)
            .unwrap_or(Duration::from_millis(1));
        match TcpStream::connect_timeout(&addr, remaining) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = error.to_string(),
        }
    }
    Err(last_error)
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

#[cfg(test)]
fn parse_socket_addr(value: &str) -> Result<SocketAddr, String> {
    value
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid socket address: {error}"))
}

fn parse_direct_mode(value: &str) -> Result<DirectMode, String> {
    match value {
        "disabled" => Ok(DirectMode::Disabled),
        "auto" => Err(
            "direct-mode auto is forbidden; direct routes require policy-bound WEAVE routing"
                .to_string(),
        ),
        _ => Err("direct-mode must be disabled".to_string()),
    }
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

fn parse_positive_u64(value: &str, name: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} must be positive"));
    }
    Ok(parsed)
}

fn parse_non_negative_usize(value: &str, name: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a non-negative integer"))
}

#[cfg(test)]
mod tests {
    use super::{
        DirectMode, Options, handle_client, parse_socket_addr, read_line_limited,
        resolve_destination,
    };
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    fn base_options(transit_local: &str) -> Options {
        Options {
            listen: "127.0.0.1:0".to_string(),
            transit_local: transit_local.to_string(),
            transit_fallback: None,
            direct_mode: DirectMode::Disabled,
            direct_timeout_ms: 500,
            initial_read_timeout_ms: 500,
            connect_retry_count: 0,
            connect_retry_delay_ms: 50,
            static_destination: None,
        }
    }

    #[test]
    fn options_parse_runtime_values() {
        let args = vec![
            "--listen".to_string(),
            "127.0.0.1:0".to_string(),
            "--transit-local".to_string(),
            "127.0.0.1:1".to_string(),
            "--direct-mode".to_string(),
            "disabled".to_string(),
            "--direct-timeout-ms".to_string(),
            "100".to_string(),
            "--initial-read-timeout-ms".to_string(),
            "50".to_string(),
            "--connect-retry-count".to_string(),
            "1".to_string(),
            "--connect-retry-delay-ms".to_string(),
            "75".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.listen, "127.0.0.1:0");
        assert_eq!(parsed.transit_local, "127.0.0.1:1");
        assert_eq!(parsed.direct_mode, DirectMode::Disabled);
        assert_eq!(parsed.direct_timeout_ms, 100);
        assert_eq!(parsed.initial_read_timeout_ms, 50);
        assert_eq!(parsed.connect_retry_count, 1);
        assert_eq!(parsed.connect_retry_delay_ms, 75);
        assert_eq!(parsed.static_destination, None);
        assert_eq!(parsed.transit_fallback, None);
    }

    #[test]
    fn options_parse_transit_fallback() {
        let args = vec![
            "--listen".to_string(),
            "127.0.0.1:0".to_string(),
            "--transit-local".to_string(),
            "127.0.0.1:1".to_string(),
            "--transit-fallback".to_string(),
            "127.0.0.1:3".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.transit_fallback, Some("127.0.0.1:3".to_string()));
    }

    #[test]
    fn options_require_listener() {
        let args = vec!["--transit-local".to_string(), "127.0.0.1:1".to_string()];
        assert!(Options::parse(&args).is_err());
    }

    #[test]
    fn parse_socket_addr_rejects_bad_value() {
        assert!(parse_socket_addr("bad").is_err());
    }

    #[test]
    fn options_reject_auto_direct_mode() {
        let args = vec![
            "--listen".to_string(),
            "127.0.0.1:0".to_string(),
            "--transit-local".to_string(),
            "127.0.0.1:1".to_string(),
            "--direct-mode".to_string(),
            "auto".to_string(),
        ];
        assert!(
            Options::parse(&args)
                .is_err_and(|error| error.contains("direct-mode auto is forbidden"))
        );
    }

    fn start_echo_transit(transit: TcpListener, prefix: &'static [u8]) {
        thread::spawn(move || {
            let Ok((mut stream, _)) = transit.accept() else {
                return;
            };
            let mut magic = [0_u8; super::LOCAL_MAGIC.len()];
            if stream.read_exact(&mut magic).is_err() || magic != super::LOCAL_MAGIC {
                return;
            }
            if read_line_limited(&mut stream, 128).is_err() {
                return;
            }
            if stream.write_all(b"OK\n").is_err() {
                return;
            }
            let mut buf = [0_u8; 16];
            let Ok(n) = stream.read(&mut buf) else {
                return;
            };
            let _ = stream.write_all(prefix);
            let _ = stream.write_all(&buf[..n]);
        });
    }

    #[test]
    fn transparent_tcp_direct_disabled_uses_transit_without_direct_probe() {
        let direct_target = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("direct target listener should bind: {error}");
        });
        let direct_target_addr = direct_target.local_addr().unwrap_or_else(|error| {
            unreachable!("direct target addr should be available: {error}");
        });

        let transit = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("transit listener should bind: {error}");
        });
        let transit_addr = transit.local_addr().unwrap_or_else(|error| {
            unreachable!("transit addr should be available: {error}");
        });
        start_echo_transit(transit, b"forced-transit:");

        let transparent = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("transparent listener should bind: {error}");
        });
        let transparent_addr = transparent.local_addr().unwrap_or_else(|error| {
            unreachable!("transparent addr should be available: {error}");
        });
        let mut options = base_options(&transit_addr.to_string());
        options.listen = transparent_addr.to_string();
        options.static_destination = Some(direct_target_addr.to_string());
        thread::spawn(move || {
            let Ok((client, _)) = transparent.accept() else {
                return;
            };
            let _ = handle_client(client, &options);
        });

        let mut client = TcpStream::connect(transparent_addr).unwrap_or_else(|error| {
            unreachable!("client should connect to transparent listener: {error}");
        });
        client.write_all(b"hello").unwrap_or_else(|error| {
            unreachable!("client write should work: {error}");
        });
        let mut reply = [0_u8; 20];
        client.read_exact(&mut reply).unwrap_or_else(|error| {
            unreachable!("client read should work: {error}");
        });
        assert_eq!(&reply, b"forced-transit:hello");
        drop(direct_target);
    }

    #[test]
    fn transparent_tcp_uses_transit_fallback_when_transit_local_is_down() {
        let transit = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("fallback transit listener should bind: {error}");
        });
        let transit_addr = transit.local_addr().unwrap_or_else(|error| {
            unreachable!("fallback transit addr should be available: {error}");
        });
        start_echo_transit(transit, b"fallback-works:");

        let transparent = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("transparent listener should bind: {error}");
        });
        let transparent_addr = transparent.local_addr().unwrap_or_else(|error| {
            unreachable!("transparent addr should be available: {error}");
        });
        let closed_transit = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("closed transit listener should bind: {error}");
        });
        let closed_transit_addr = closed_transit.local_addr().unwrap_or_else(|error| {
            unreachable!("closed transit addr should be available: {error}");
        });
        drop(closed_transit);

        let mut options = base_options(&closed_transit_addr.to_string());
        options.listen = transparent_addr.to_string();
        options.transit_fallback = Some(transit_addr.to_string());
        options.static_destination = Some("127.0.0.1:80".to_string());
        thread::spawn(move || {
            let Ok((client, _)) = transparent.accept() else {
                return;
            };
            let _ = handle_client(client, &options);
        });

        let mut client = TcpStream::connect(transparent_addr).unwrap_or_else(|error| {
            unreachable!("client should connect to transparent listener: {error}");
        });
        client.write_all(b"hello").unwrap_or_else(|error| {
            unreachable!("client write should work: {error}");
        });
        let mut reply = [0_u8; 20];
        client.read_exact(&mut reply).unwrap_or_else(|error| {
            unreachable!("client read should work: {error}");
        });
        assert_eq!(&reply[..], b"fallback-works:hello");
    }

    #[test]
    fn transparent_tcp_retries_transit_until_success() {
        // First transit listener accepts and immediately closes (dead peer).
        // Second transit listener succeeds.
        let dead_transit = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("dead transit listener should bind: {error}");
        });
        let dead_transit_addr = dead_transit.local_addr().unwrap_or_else(|error| {
            unreachable!("dead transit addr should be available: {error}");
        });
        thread::spawn(move || {
            let Ok((stream, _)) = dead_transit.accept() else { return };
            let _ = stream.shutdown(std::net::Shutdown::Both);
        });

        let alive_transit = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("alive transit listener should bind: {error}");
        });
        let alive_transit_addr = alive_transit.local_addr().unwrap_or_else(|error| {
            unreachable!("alive transit addr should be available: {error}");
        });
        start_echo_transit(alive_transit, b"retry-ok:");

        let transparent = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("transparent listener should bind: {error}");
        });
        let transparent_addr = transparent.local_addr().unwrap_or_else(|error| {
            unreachable!("transparent addr should be available: {error}");
        });

        let mut options = base_options(&dead_transit_addr.to_string());
        options.listen = transparent_addr.to_string();
        options.transit_fallback = Some(alive_transit_addr.to_string());
        options.connect_retry_count = 2;
        options.connect_retry_delay_ms = 50;
        options.static_destination = Some("127.0.0.1:80".to_string());
        thread::spawn(move || {
            let Ok((client, _)) = transparent.accept() else {
                return;
            };
            let _ = handle_client(client, &options);
        });

        let mut client = TcpStream::connect(transparent_addr).unwrap_or_else(|error| {
            unreachable!("client should connect to transparent listener: {error}");
        });
        client.write_all(b"hello").unwrap_or_else(|error| {
            unreachable!("client write should work: {error}");
        });
        let mut reply = [0_u8; 32];
        let n = client.read(&mut reply).unwrap_or_else(|error| {
            unreachable!("client read should work: {error}");
        });
        assert_eq!(&reply[..n], b"retry-ok:hello");
    }

    #[test]
    fn transparent_destination_rejects_manual_proxy_ingress() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("listener should bind: {error}");
        });
        let listener_addr = listener.local_addr().unwrap_or_else(|error| {
            unreachable!("listener addr should be available: {error}");
        });
        let client = TcpStream::connect(listener_addr).unwrap_or_else(|error| {
            unreachable!("client should connect: {error}");
        });
        let (server, _) = listener.accept().unwrap_or_else(|error| {
            unreachable!("server should accept: {error}");
        });
        let options = base_options("127.0.0.1:1");

        let error = match resolve_destination(
            &server,
            b"CONNECT example.org:443 HTTP/1.1\r\nHost: example.org\r\n\r\n",
            &options,
        ) {
            Ok(_) => unreachable!("manual proxy ingress must fail closed"),
            Err(error) => error,
        };

        assert!(error.contains("manual_proxy_ingress_forbidden"));
        assert!(error.contains("http_connect"));
        drop(client);
    }
}
