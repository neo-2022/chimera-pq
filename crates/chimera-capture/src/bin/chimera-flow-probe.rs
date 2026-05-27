#![forbid(unsafe_code)]

use std::env;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use socket2::SockRef;

const BUFFER_BYTES: usize = 1024 * 1024;
const TCP_BUFFER_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    connect: String,
    request: Option<String>,
    bytes: usize,
    connect_timeout_ms: u64,
    min_throughput_mib_s: u64,
    event_name: String,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut connect = env_value("CHIMERA_FLOW_PROBE_CONNECT");
        let mut request = env_value("CHIMERA_FLOW_PROBE_REQUEST");
        let mut bytes = env_value("CHIMERA_FLOW_PROBE_BYTES")
            .map(|value| parse_positive_usize(&value, "bytes"))
            .transpose()?
            .unwrap_or(8 * 1024 * 1024);
        let mut connect_timeout_ms = env_value("CHIMERA_FLOW_PROBE_CONNECT_TIMEOUT_MS")
            .map(|value| parse_positive_u64(&value, "connect-timeout-ms"))
            .transpose()?
            .unwrap_or(3000);
        let mut min_throughput_mib_s = env_value("CHIMERA_FLOW_PROBE_MIN_THROUGHPUT_MIB_S")
            .map(|value| parse_positive_u64(&value, "min-throughput-mib-s"))
            .transpose()?
            .unwrap_or(0);
        let mut event_name = env_value("CHIMERA_FLOW_PROBE_EVENT")
            .unwrap_or_else(|| "chimera_flow_probe".to_string());

        let mut index = 0usize;
        while index < args.len() {
            let flag = args[index].as_str();
            match flag {
                "--connect" => {
                    connect = Some(arg_value(args, index, flag)?);
                    index += 2;
                }
                "--request" => {
                    request = Some(arg_value(args, index, flag)?);
                    index += 2;
                }
                "--request-line" => {
                    let mut line = arg_value(args, index, flag)?;
                    line.push('\n');
                    request = Some(line);
                    index += 2;
                }
                "--bytes" => {
                    bytes = parse_positive_usize(&arg_value(args, index, flag)?, "bytes")?;
                    index += 2;
                }
                "--connect-timeout-ms" => {
                    connect_timeout_ms =
                        parse_positive_u64(&arg_value(args, index, flag)?, "connect-timeout-ms")?;
                    index += 2;
                }
                "--min-throughput-mib-s" => {
                    min_throughput_mib_s =
                        parse_positive_u64(&arg_value(args, index, flag)?, "min-throughput-mib-s")?;
                    index += 2;
                }
                "--event" => {
                    event_name = arg_value(args, index, flag)?;
                    index += 2;
                }
                _ => return Err(format!("unknown argument: {flag}")),
            }
        }

        Ok(Self {
            connect: required_value(connect, "missing --connect or CHIMERA_FLOW_PROBE_CONNECT")?,
            request,
            bytes,
            connect_timeout_ms,
            min_throughput_mib_s,
            event_name,
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
    let mut stream = connect_tcp(&options.connect, options.connect_timeout_ms)?;
    tune_tcp(&stream)?;
    if let Some(request) = options.request.as_deref() {
        stream
            .write_all(request.as_bytes())
            .map_err(|error| format!("write request failed: {error}"))?;
    }
    let started = Instant::now();
    read_exact_bytes(&mut stream, options.bytes)?;
    let elapsed = started.elapsed();
    let seconds = elapsed.as_secs_f64().max(0.000_001);
    let mib = options.bytes as f64 / 1024.0 / 1024.0;
    let throughput_mib_s = mib / seconds;
    if options.min_throughput_mib_s > 0 && throughput_mib_s < options.min_throughput_mib_s as f64 {
        return Err(format!(
            "throughput below gate: actual_mib_s={throughput_mib_s:.2} min_mib_s={}",
            options.min_throughput_mib_s
        ));
    }
    println!(
        "{}=pass bytes={} elapsed_ms={} throughput_mib_s={:.2}",
        options.event_name,
        options.bytes,
        elapsed.as_millis(),
        throughput_mib_s
    );
    Ok(())
}

fn tune_tcp(stream: &TcpStream) -> Result<(), String> {
    stream
        .set_nodelay(true)
        .map_err(|error| format!("set TCP_NODELAY failed: {error}"))?;
    let socket = SockRef::from(stream);
    let _ = socket.set_recv_buffer_size(TCP_BUFFER_BYTES);
    let _ = socket.set_send_buffer_size(TCP_BUFFER_BYTES);
    Ok(())
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
    Err(format!("connect failed: {last_error}"))
}

fn read_exact_bytes(stream: &mut TcpStream, bytes: usize) -> Result<(), String> {
    let mut remaining = bytes;
    let mut buf = vec![0_u8; BUFFER_BYTES.min(bytes)];
    while remaining > 0 {
        let want = remaining.min(buf.len());
        let n = stream
            .read(&mut buf[..want])
            .map_err(|error| format!("read failed: {error}"))?;
        if n == 0 {
            return Err(format!("unexpected EOF with {remaining} bytes remaining"));
        }
        remaining -= n;
    }
    Ok(())
}

fn arg_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("missing value for {flag}"))
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

#[cfg(test)]
mod tests {
    use super::Options;

    #[test]
    fn options_parse_runtime_values() {
        let args = vec![
            "--connect".to_string(),
            "192.0.2.10:443".to_string(),
            "--request".to_string(),
            "SEND 10\n".to_string(),
            "--bytes".to_string(),
            "10".to_string(),
            "--connect-timeout-ms".to_string(),
            "100".to_string(),
            "--min-throughput-mib-s".to_string(),
            "1".to_string(),
            "--event".to_string(),
            "probe".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.connect, "192.0.2.10:443");
        assert_eq!(parsed.request, Some("SEND 10\n".to_string()));
        assert_eq!(parsed.bytes, 10);
        assert_eq!(parsed.connect_timeout_ms, 100);
        assert_eq!(parsed.min_throughput_mib_s, 1);
        assert_eq!(parsed.event_name, "probe");
    }

    #[test]
    fn options_require_connect() {
        assert!(Options::parse(&[]).is_err());
    }

    #[test]
    fn options_request_line_appends_newline() {
        let args = vec![
            "--connect".to_string(),
            "192.0.2.10:443".to_string(),
            "--request-line".to_string(),
            "SEND 10".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.request, Some("SEND 10\n".to_string()));
    }
}
