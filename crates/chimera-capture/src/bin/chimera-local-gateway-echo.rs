#![forbid(unsafe_code)]

use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

const LOCAL_MAGIC: &[u8] = b"CHIMERA-LOCAL/1\n";

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    listen: String,
    prefix: Vec<u8>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut listen = env_value("CHIMERA_LOCAL_GATEWAY_ECHO_LISTEN");
        let mut prefix = env_value("CHIMERA_LOCAL_GATEWAY_ECHO_PREFIX")
            .unwrap_or_else(|| "gateway:".to_string())
            .into_bytes();
        let mut index = 0usize;
        while index < args.len() {
            let flag = args[index].as_str();
            let value = args
                .get(index + 1)
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag {
                "--listen" => listen = Some(value.clone()),
                "--prefix" => prefix = value.as_bytes().to_vec(),
                _ => return Err(format!("unknown flag: {flag}")),
            }
            index += 2;
        }
        Ok(Self {
            listen: listen
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "missing --listen".to_string())?,
            prefix,
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
        .map_err(|error| format!("bind local gateway echo failed: {error}"))?;
    println!("chimera_local_gateway_echo=ready listen={}", options.listen);
    for incoming in listener.incoming() {
        let Ok(stream) = incoming else {
            continue;
        };
        let prefix = options.prefix.clone();
        thread::spawn(move || {
            let _ = handle_client(stream, &prefix);
        });
    }
    Ok(())
}

fn handle_client(mut stream: TcpStream, prefix: &[u8]) -> Result<(), String> {
    let mut magic = vec![0_u8; LOCAL_MAGIC.len()];
    stream
        .read_exact(&mut magic)
        .map_err(|error| format!("read local magic failed: {error}"))?;
    if magic != LOCAL_MAGIC {
        return Err("bad local magic".to_string());
    }
    let request = read_line_limited(&mut stream, 512)?;
    if !request.starts_with("CONNECT ") {
        return Err("bad connect request".to_string());
    }
    stream
        .write_all(b"OK\n")
        .map_err(|error| format!("write OK failed: {error}"))?;
    let mut buf = vec![0_u8; 128 * 1024];
    let n = stream
        .read(&mut buf)
        .map_err(|error| format!("read payload failed: {error}"))?;
    stream
        .write_all(prefix)
        .and_then(|_| stream.write_all(&buf[..n]))
        .map_err(|error| format!("write echo failed: {error}"))
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

#[cfg(test)]
mod tests {
    use super::Options;

    #[test]
    fn options_parse() {
        let args = vec![
            "--listen".to_string(),
            "127.0.0.1:0".to_string(),
            "--prefix".to_string(),
            "ok:".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.listen, "127.0.0.1:0");
        assert_eq!(parsed.prefix, b"ok:");
    }
}
