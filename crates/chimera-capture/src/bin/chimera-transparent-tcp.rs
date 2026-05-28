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

const LOCAL_MAGIC: &[u8] = b"CHIMERA-LOCAL/1\n";
const MAX_INITIAL_BYTES: usize = 128 * 1024;
const COPY_BUFFER_BYTES: usize = 1024 * 1024;
const TCP_BUFFER_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
enum DirectMode {
    Auto,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    listen: String,
    gateway_local: String,
    gateway_fallback: Option<String>,
    direct_mode: DirectMode,
    direct_timeout_ms: u64,
    first_response_timeout_ms: u64,
    initial_read_timeout_ms: u64,
    static_destination: Option<String>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut listen = env_value("CHIMERA_TRANSPARENT_TCP_LISTEN");
        let mut gateway_local = env_value("CHIMERA_TRANSPARENT_TCP_GATEWAY_LOCAL");
        let mut gateway_fallback = env_value("CHIMERA_TRANSPARENT_TCP_GATEWAY_FALLBACK");
        let mut direct_mode = env_value("CHIMERA_TRANSPARENT_TCP_DIRECT_MODE")
            .map(|value| parse_direct_mode(&value))
            .transpose()?
            .unwrap_or(DirectMode::Auto);
        let mut direct_timeout_ms = env_value("CHIMERA_TRANSPARENT_TCP_DIRECT_TIMEOUT_MS")
            .map(|value| parse_positive_u64(&value, "direct-timeout-ms"))
            .transpose()?
            .unwrap_or(1200);
        let mut first_response_timeout_ms =
            env_value("CHIMERA_TRANSPARENT_TCP_FIRST_RESPONSE_TIMEOUT_MS")
                .map(|value| parse_positive_u64(&value, "first-response-timeout-ms"))
                .transpose()?
                .unwrap_or(1800);
        let mut initial_read_timeout_ms =
            env_value("CHIMERA_TRANSPARENT_TCP_INITIAL_READ_TIMEOUT_MS")
                .map(|value| parse_positive_u64(&value, "initial-read-timeout-ms"))
                .transpose()?
                .unwrap_or(500);
        let mut static_destination = env_value("CHIMERA_TRANSPARENT_TCP_STATIC_DESTINATION");

        let mut index = 0usize;
        while index < args.len() {
            let flag = args[index].as_str();
            let value = args
                .get(index + 1)
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag {
                "--listen" => listen = Some(value.clone()),
                "--gateway-local" => gateway_local = Some(value.clone()),
                "--gateway-fallback" => gateway_fallback = Some(value.clone()),
                "--direct-mode" => direct_mode = parse_direct_mode(value)?,
                "--direct-timeout-ms" => {
                    direct_timeout_ms = parse_positive_u64(value, "direct-timeout-ms")?;
                }
                "--first-response-timeout-ms" => {
                    first_response_timeout_ms =
                        parse_positive_u64(value, "first-response-timeout-ms")?;
                }
                "--initial-read-timeout-ms" => {
                    initial_read_timeout_ms = parse_positive_u64(value, "initial-read-timeout-ms")?;
                }
                "--static-destination" => static_destination = Some(value.clone()),
                _ => return Err(format!("unknown flag: {flag}")),
            }
            index += 2;
        }

        Ok(Self {
            listen: required_value(listen, "missing --listen or CHIMERA_TRANSPARENT_TCP_LISTEN")?,
            gateway_local: required_value(
                gateway_local,
                "missing --gateway-local or CHIMERA_TRANSPARENT_TCP_GATEWAY_LOCAL",
            )?,
            gateway_fallback,
            direct_mode,
            direct_timeout_ms,
            first_response_timeout_ms,
            initial_read_timeout_ms,
            static_destination,
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
    eprintln!("event=transparent_flow_accepted destination={destination}");
    if options.direct_mode == DirectMode::Disabled {
        let gateway = connect_gateway_with_fallback(
            &options.gateway_local,
            options.gateway_fallback.as_deref(),
            &destination,
            &initial,
            options.direct_timeout_ms,
        )?;
        eprintln!(
            "event=transparent_route_selected route=gateway reason=direct_mode_disabled destination={destination}"
        );
        return relay_plain(client, gateway);
    }
    match try_direct(&destination, &initial, options) {
        Ok((target, first_response)) => {
            eprintln!("event=transparent_route_selected route=direct destination={destination}");
            relay_after_probe(client, target, first_response)
        }
        Err(error) => {
            eprintln!("event=transparent_direct_failed reason={error}");
            let gateway = connect_gateway_with_fallback(
                &options.gateway_local,
                options.gateway_fallback.as_deref(),
                &destination,
                &initial,
                options.direct_timeout_ms,
            )?;
            eprintln!("event=transparent_route_selected route=gateway destination={destination}");
            relay_plain(client, gateway)
        }
    }
}

fn connect_gateway_with_fallback(
    gateway_local: &str,
    gateway_fallback: Option<&str>,
    destination: &SocketAddr,
    initial: &[u8],
    timeout_ms: u64,
) -> Result<TcpStream, String> {
    match connect_gateway(gateway_local, destination, initial, timeout_ms) {
        Ok(stream) => Ok(stream),
        Err(err) => match gateway_fallback {
            Some(fallback) => {
                eprintln!(
                    "event=gateway_fallback_trying fallback={fallback} reason=\"{err}\""
                );
                connect_gateway(fallback, destination, initial, timeout_ms)
            }
            None => Err(err),
        },
    }
}

fn try_direct(
    destination: &SocketAddr,
    initial: &[u8],
    options: &Options,
) -> Result<(TcpStream, Vec<u8>), String> {
    let mut target = connect_tcp(&destination.to_string(), options.direct_timeout_ms)
        .map_err(|error| format!("direct connect failed: {error}"))?;
    tune_tcp(&target)?;
    if !initial.is_empty() {
        target
            .write_all(initial)
            .map_err(|error| format!("direct initial write failed: {error}"))?;
        target
            .set_read_timeout(Some(Duration::from_millis(
                options.first_response_timeout_ms,
            )))
            .map_err(|error| format!("direct set read timeout failed: {error}"))?;
        let mut first = vec![0_u8; COPY_BUFFER_BYTES];
        match target.read(&mut first) {
            Ok(0) => Err("direct closed before first response".to_string()),
            Ok(n) => {
                first.truncate(n);
                target
                    .set_read_timeout(None)
                    .map_err(|error| format!("direct clear read timeout failed: {error}"))?;
                Ok((target, first))
            }
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                Err("direct first response timed out".to_string())
            }
            Err(error) => Err(format!("direct first response read failed: {error}")),
        }
    } else {
        Ok((target, Vec::new()))
    }
}

fn connect_gateway(
    gateway_local: &str,
    destination: &SocketAddr,
    initial: &[u8],
    timeout_ms: u64,
) -> Result<TcpStream, String> {
    let mut gateway = connect_tcp(gateway_local, timeout_ms)
        .map_err(|error| format!("gateway connect failed: {error}"))?;
    tune_tcp(&gateway)?;
    let host = destination.ip().to_string();
    gateway
        .write_all(LOCAL_MAGIC)
        .and_then(|_| {
            gateway.write_all(format!("CONNECT {host} {}\n", destination.port()).as_bytes())
        })
        .map_err(|error| format!("gateway connect request write failed: {error}"))?;
    let ack = read_line_limited(&mut gateway, 16)?;
    if ack != "OK" {
        return Err("gateway connect request rejected".to_string());
    }
    if !initial.is_empty() {
        gateway
            .write_all(initial)
            .map_err(|error| format!("gateway initial write failed: {error}"))?;
    }
    Ok(gateway)
}

fn relay_after_probe(
    mut client: TcpStream,
    target: TcpStream,
    first_response: Vec<u8>,
) -> Result<(), String> {
    if !first_response.is_empty() {
        client
            .write_all(&first_response)
            .map_err(|error| format!("write first response to client failed: {error}"))?;
    }
    relay_plain(client, target)
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
    options: &Options,
) -> Result<SocketAddr, String> {
    match options.static_destination.as_deref() {
        Some(dest) => parse_socket_addr(dest),
        None => {
            if let Some(addr) = parse_proxy_destination(initial) {
                Ok(addr)
            } else {
                original_destination(client)
            }
        }
    }
}

fn parse_proxy_destination(initial: &[u8]) -> Option<SocketAddr> {
    let text = std::str::from_utf8(initial).ok()?;
    if let Some(rest) = text.strip_prefix("CONNECT ") {
        let host_port = rest.split_whitespace().next()?;
        let (host, port) = host_port.rsplit_once(':')?;
        let port: u16 = port.parse().ok()?;
        let addr = resolve_host_to_v4(host, port)?;
        return Some(addr);
    }
    if let Some(rest) = text.strip_prefix("GET http://") {
        let host_part = rest.splitn(2, '/').next()?;
        let (host, resolved_port) = if let Some((h, p)) = host_part.rsplit_once(':') {
            (h, p.parse::<u16>().ok().unwrap_or(80))
        } else {
            (host_part, 80u16)
        };
        let addr = resolve_host_to_v4(host, resolved_port)?;
        return Some(addr);
    }
    if let Some(rest) = text.strip_prefix("POST http://") {
        let host_part = rest.splitn(2, '/').next()?;
        let (host, resolved_port) = if let Some((h, p)) = host_part.rsplit_once(':') {
            (h, p.parse::<u16>().ok().unwrap_or(80))
        } else {
            (host_part, 80u16)
        };
        let addr = resolve_host_to_v4(host, resolved_port)?;
        return Some(addr);
    }
    if initial.first() == Some(&5) && initial.len() >= 4 {
        let addr_type = initial[3];
        if addr_type == 1 && initial.len() >= 10 {
            let ip = Ipv4Addr::new(initial[4], initial[5], initial[6], initial[7]);
            let port = u16::from_be_bytes([initial[8], initial[9]]);
            if !ip.is_loopback() && !ip.is_private() {
                return Some(SocketAddr::V4(SocketAddrV4::new(ip, port)));
            }
        }
    }
    None
}

fn resolve_host_to_v4(host: &str, port: u16) -> Option<SocketAddr> {
    let host = host.trim_end_matches('.');
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return Some(SocketAddr::V4(SocketAddrV4::new(ip, port)));
    }
    std::net::ToSocketAddrs::to_socket_addrs(&(host, port))
        .ok()?
        .find_map(|addr| match addr {
            SocketAddr::V4(v4) => Some(SocketAddr::V4(v4)),
            _ => None,
        })
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
            Err(error) => last_error = format!("{addr}: {error}"),
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

fn parse_socket_addr(value: &str) -> Result<SocketAddr, String> {
    value
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid socket address: {error}"))
}

fn parse_direct_mode(value: &str) -> Result<DirectMode, String> {
    match value {
        "auto" => Ok(DirectMode::Auto),
        "disabled" => Ok(DirectMode::Disabled),
        _ => Err("direct-mode must be auto or disabled".to_string()),
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

#[cfg(test)]
mod tests {
    use super::{DirectMode, Options, handle_client, parse_socket_addr, read_line_limited};
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[test]
    fn options_parse_runtime_values() {
        let args = vec![
            "--listen".to_string(),
            "127.0.0.1:0".to_string(),
            "--gateway-local".to_string(),
            "127.0.0.1:1".to_string(),
            "--direct-mode".to_string(),
            "disabled".to_string(),
            "--direct-timeout-ms".to_string(),
            "100".to_string(),
            "--first-response-timeout-ms".to_string(),
            "200".to_string(),
            "--initial-read-timeout-ms".to_string(),
            "50".to_string(),
            "--static-destination".to_string(),
            "127.0.0.1:2".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.listen, "127.0.0.1:0");
        assert_eq!(parsed.gateway_local, "127.0.0.1:1");
        assert_eq!(parsed.direct_mode, DirectMode::Disabled);
        assert_eq!(parsed.direct_timeout_ms, 100);
        assert_eq!(parsed.first_response_timeout_ms, 200);
        assert_eq!(parsed.initial_read_timeout_ms, 50);
        assert_eq!(parsed.static_destination, Some("127.0.0.1:2".to_string()));
        assert_eq!(parsed.gateway_fallback, None);
    }

    #[test]
    fn options_parse_gateway_fallback() {
        let args = vec![
            "--listen".to_string(),
            "127.0.0.1:0".to_string(),
            "--gateway-local".to_string(),
            "127.0.0.1:1".to_string(),
            "--gateway-fallback".to_string(),
            "127.0.0.1:3".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.gateway_fallback, Some("127.0.0.1:3".to_string()));
    }

    #[test]
    fn options_require_listener() {
        let args = vec!["--gateway-local".to_string(), "127.0.0.1:1".to_string()];
        assert!(Options::parse(&args).is_err());
    }

    #[test]
    fn parse_socket_addr_rejects_bad_value() {
        assert!(parse_socket_addr("bad").is_err());
    }

    #[test]
    fn transparent_tcp_uses_direct_when_direct_responds() {
        let target = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("target listener should bind: {error}");
        });
        let target_addr = target.local_addr().unwrap_or_else(|error| {
            unreachable!("target addr should be available: {error}");
        });
        thread::spawn(move || {
            let Ok((mut stream, _)) = target.accept() else {
                return;
            };
            let mut buf = [0_u8; 16];
            let Ok(n) = stream.read(&mut buf) else {
                return;
            };
            let _ = stream.write_all(b"direct:");
            let _ = stream.write_all(&buf[..n]);
        });

        let gateway = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("gateway listener should bind: {error}");
        });
        let gateway_addr = gateway.local_addr().unwrap_or_else(|error| {
            unreachable!("gateway addr should be available: {error}");
        });

        let transparent = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("transparent listener should bind: {error}");
        });
        let transparent_addr = transparent.local_addr().unwrap_or_else(|error| {
            unreachable!("transparent addr should be available: {error}");
        });
        let options = Options {
            listen: transparent_addr.to_string(),
            gateway_local: gateway_addr.to_string(),
            gateway_fallback: None,
            direct_mode: DirectMode::Auto,
            direct_timeout_ms: 500,
            first_response_timeout_ms: 500,
            initial_read_timeout_ms: 500,
            static_destination: Some(target_addr.to_string()),
        };
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
        let mut reply = [0_u8; 12];
        client.read_exact(&mut reply).unwrap_or_else(|error| {
            unreachable!("client read should work: {error}");
        });
        assert_eq!(&reply, b"direct:hello");
    }

    #[test]
    fn transparent_tcp_falls_back_to_gateway_when_direct_is_down() {
        let closed_target = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("closed target listener should bind: {error}");
        });
        let closed_target_addr = closed_target.local_addr().unwrap_or_else(|error| {
            unreachable!("closed target addr should be available: {error}");
        });
        drop(closed_target);

        let gateway = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("gateway listener should bind: {error}");
        });
        let gateway_addr = gateway.local_addr().unwrap_or_else(|error| {
            unreachable!("gateway addr should be available: {error}");
        });
        thread::spawn(move || {
            let Ok((mut stream, _)) = gateway.accept() else {
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
            let _ = stream.write_all(b"gateway:");
            let _ = stream.write_all(&buf[..n]);
        });

        let transparent = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("transparent listener should bind: {error}");
        });
        let transparent_addr = transparent.local_addr().unwrap_or_else(|error| {
            unreachable!("transparent addr should be available: {error}");
        });
        let options = Options {
            listen: transparent_addr.to_string(),
            gateway_local: gateway_addr.to_string(),
            gateway_fallback: None,
            direct_mode: DirectMode::Auto,
            direct_timeout_ms: 500,
            first_response_timeout_ms: 500,
            initial_read_timeout_ms: 500,
            static_destination: Some(closed_target_addr.to_string()),
        };
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
        let mut reply = [0_u8; 13];
        client.read_exact(&mut reply).unwrap_or_else(|error| {
            unreachable!("client read should work: {error}");
        });
        assert_eq!(&reply, b"gateway:hello");
    }

    #[test]
    fn transparent_tcp_direct_disabled_uses_gateway_without_direct_probe() {
        let direct_target = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("direct target listener should bind: {error}");
        });
        let direct_target_addr = direct_target.local_addr().unwrap_or_else(|error| {
            unreachable!("direct target addr should be available: {error}");
        });

        let gateway = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("gateway listener should bind: {error}");
        });
        let gateway_addr = gateway.local_addr().unwrap_or_else(|error| {
            unreachable!("gateway addr should be available: {error}");
        });
        thread::spawn(move || {
            let Ok((mut stream, _)) = gateway.accept() else {
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
            let _ = stream.write_all(b"forced-gw:");
            let _ = stream.write_all(&buf[..n]);
        });

        let transparent = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("transparent listener should bind: {error}");
        });
        let transparent_addr = transparent.local_addr().unwrap_or_else(|error| {
            unreachable!("transparent addr should be available: {error}");
        });
        let options = Options {
            listen: transparent_addr.to_string(),
            gateway_local: gateway_addr.to_string(),
            gateway_fallback: None,
            direct_mode: DirectMode::Disabled,
            direct_timeout_ms: 500,
            first_response_timeout_ms: 500,
            initial_read_timeout_ms: 500,
            static_destination: Some(direct_target_addr.to_string()),
        };
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
        let mut reply = [0_u8; 15];
        client.read_exact(&mut reply).unwrap_or_else(|error| {
            unreachable!("client read should work: {error}");
        });
        assert_eq!(&reply, b"forced-gw:hello");
        drop(direct_target);
    }

    #[test]
    fn transparent_tcp_uses_gateway_fallback_when_gateway_local_is_down() {
        let gateway = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("fallback gateway listener should bind: {error}");
        });
        let gateway_addr = gateway.local_addr().unwrap_or_else(|error| {
            unreachable!("fallback gateway addr should be available: {error}");
        });
        thread::spawn(move || {
            let Ok((mut stream, _)) = gateway.accept() else {
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
            let _ = stream.write_all(b"fallback-works:");
            let _ = stream.write_all(&buf[..n]);
        });

        let transparent = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("transparent listener should bind: {error}");
        });
        let transparent_addr = transparent.local_addr().unwrap_or_else(|error| {
            unreachable!("transparent addr should be available: {error}");
        });
        let closed_gateway = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
            unreachable!("closed gateway listener should bind: {error}");
        });
        let closed_gateway_addr = closed_gateway.local_addr().unwrap_or_else(|error| {
            unreachable!("closed gateway addr should be available: {error}");
        });
        drop(closed_gateway);

        let options = Options {
            listen: transparent_addr.to_string(),
            gateway_local: closed_gateway_addr.to_string(),
            gateway_fallback: Some(gateway_addr.to_string()),
            direct_mode: DirectMode::Disabled,
            direct_timeout_ms: 500,
            first_response_timeout_ms: 500,
            initial_read_timeout_ms: 500,
            static_destination: Some("127.0.0.1:80".to_string()),
        };
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
}
