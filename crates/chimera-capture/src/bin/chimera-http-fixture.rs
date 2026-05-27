#![forbid(unsafe_code)]

use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    listen: String,
    body_bytes: usize,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut listen = env_value("CHIMERA_HTTP_FIXTURE_LISTEN");
        let mut body_bytes = env_value("CHIMERA_HTTP_FIXTURE_BODY_BYTES")
            .map(|value| parse_positive_usize(&value, "body-bytes"))
            .transpose()?
            .unwrap_or(4096);
        let mut index = 0usize;
        while index < args.len() {
            let flag = args[index].as_str();
            match flag {
                "--listen" => {
                    listen = Some(arg_value(args, index, flag)?);
                    index += 2;
                }
                "--body-bytes" => {
                    body_bytes =
                        parse_positive_usize(&arg_value(args, index, flag)?, "body-bytes")?;
                    index += 2;
                }
                _ => return Err(format!("unknown argument: {flag}")),
            }
        }
        Ok(Self {
            listen: required_value(listen, "missing --listen or CHIMERA_HTTP_FIXTURE_LISTEN")?,
            body_bytes,
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
        .map_err(|error| format!("bind http fixture failed: {error}"))?;
    println!("chimera_http_fixture=ready listen={}", options.listen);
    for incoming in listener.incoming() {
        let Ok(stream) = incoming else {
            continue;
        };
        let body_bytes = options.body_bytes;
        thread::spawn(move || {
            let _ = handle_http(stream, body_bytes);
        });
    }
    Ok(())
}

fn handle_http(mut stream: TcpStream, body_bytes: usize) -> Result<(), String> {
    let mut request = [0_u8; 2048];
    let _ = stream.read(&mut request);
    let body = build_body(body_bytes);
    let header = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .and_then(|_| stream.write_all(&body))
        .map_err(|error| format!("write http response failed: {error}"))
}

fn build_body(body_bytes: usize) -> Vec<u8> {
    let prefix = b"<html><title>CHIMERA fixture</title><body>chimera-http-fixture ";
    let suffix = b"</body></html>";
    let mut body = Vec::with_capacity(body_bytes.max(prefix.len() + suffix.len()));
    body.extend_from_slice(prefix);
    while body.len() + suffix.len() < body_bytes {
        body.extend_from_slice(b"CHIMERA_OK ");
    }
    body.extend_from_slice(suffix);
    body.truncate(body_bytes.max(prefix.len() + suffix.len()));
    body
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

#[cfg(test)]
mod tests {
    use super::{Options, build_body};

    #[test]
    fn options_parse() {
        let args = vec![
            "--listen".to_string(),
            "127.0.0.1:0".to_string(),
            "--body-bytes".to_string(),
            "100".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.listen, "127.0.0.1:0");
        assert_eq!(parsed.body_bytes, 100);
    }

    #[test]
    fn body_contains_marker() {
        let body = build_body(128);
        assert!(String::from_utf8_lossy(&body).contains("CHIMERA_OK"));
    }
}
