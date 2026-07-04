use crate::Result;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use url::Url;

use super::serve_state::write_peer_update_state_file;
use super::{
    DISCOVERY_PUBKEY_NAME, DISCOVERY_PUBKEY_ROUTE, DISCOVERY_SNAPSHOT_NAME,
    DISCOVERY_SNAPSHOT_ROUTE, RELEASE_ARCHIVE_NAME, RELEASE_ARCHIVE_ROUTE, RELEASE_CHECKSUM_NAME,
    RELEASE_CHECKSUM_ROUTE, RELEASE_METADATA_ROUTE,
};

const PEER_UPDATE_IO_TIMEOUT: Duration = Duration::from_secs(5);
const PEER_UPDATE_HEADER_TIMEOUT: Duration = Duration::from_secs(10);
const PEER_UPDATE_MAX_ACTIVE_CONNECTIONS: usize = 32;

pub fn serve_release(
    root: &Path,
    listen: &str,
    public_base_url: Option<&str>,
    state_file: Option<&Path>,
) -> Result<()> {
    let version = read_release_version(root)?;
    let checksum = read_release_checksum(root)?;
    let archive = release_archive_path(root);
    let checksum_file = release_checksum_path(root);
    let script = root.join("scripts").join("chimera.sh");
    require_file(&archive)?;
    require_file(&checksum_file)?;
    require_file(&script)?;
    let listener = TcpListener::bind(listen)?;
    let bound_addr = listener.local_addr()?;
    let listen = bound_addr.to_string();
    let public_base_url = advertised_base_url(public_base_url, bound_addr)?;
    let discovery_snapshot_path = sibling_cache_artifact_path(state_file, DISCOVERY_SNAPSHOT_NAME);
    let discovery_pubkey_path = sibling_cache_artifact_path(state_file, DISCOVERY_PUBKEY_NAME);
    let update_bootstrap_url = public_base_url
        .as_deref()
        .map(|base_url| join_url(base_url, "/chimera.sh"))
        .unwrap_or_else(|| "host_header/chimera.sh".to_string());
    if let Some(state_file) = state_file {
        let state_update_bootstrap_url = public_base_url
            .as_deref()
            .map(|base_url| join_url(base_url, "/chimera.sh"));
        write_peer_update_state_file(
            state_file,
            &listen,
            public_base_url.as_deref(),
            state_update_bootstrap_url.as_deref(),
            &version,
            &checksum,
        )?;
    }
    eprintln!(
        "chimera_peer_update_serve=ready listen={} version={} sha256={} update_bootstrap_url={}",
        listen, version, checksum, update_bootstrap_url
    );
    let root = Arc::new(root.to_path_buf());
    let active_connections = Arc::new(AtomicUsize::new(0));
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                if !try_acquire_peer_update_slot(&active_connections) {
                    eprintln!(
                        "chimera_peer_update_request=reject reason=too_many_active_connections"
                    );
                    drop(stream);
                    continue;
                }
                let root = Arc::clone(&root);
                let listen = listen.clone();
                let public_base_url = public_base_url.clone();
                let discovery_snapshot_path = discovery_snapshot_path.clone();
                let discovery_pubkey_path = discovery_pubkey_path.clone();
                let active_connections = Arc::clone(&active_connections);
                thread::spawn(move || {
                    let _slot = PeerUpdateConnectionSlot::new(active_connections);
                    let mut stream = stream;
                    if let Err(error) = prepare_peer_update_stream(&stream).and_then(|_| {
                        handle_release_request(
                            &mut stream,
                            root.as_path(),
                            &listen,
                            public_base_url.as_deref(),
                            discovery_snapshot_path.as_deref(),
                            discovery_pubkey_path.as_deref(),
                        )
                    }) {
                        eprintln!("chimera_peer_update_request=fail reason={error}");
                    }
                });
            }
            Err(error) => {
                eprintln!("chimera_peer_update_accept=fail reason={error}");
            }
        }
    }
    Ok(())
}

fn try_acquire_peer_update_slot(active_connections: &AtomicUsize) -> bool {
    active_connections
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
            (current < PEER_UPDATE_MAX_ACTIVE_CONNECTIONS).then_some(current + 1)
        })
        .is_ok()
}

struct PeerUpdateConnectionSlot {
    active_connections: Arc<AtomicUsize>,
}

impl PeerUpdateConnectionSlot {
    fn new(active_connections: Arc<AtomicUsize>) -> Self {
        Self { active_connections }
    }
}

impl Drop for PeerUpdateConnectionSlot {
    fn drop(&mut self) {
        self.active_connections.fetch_sub(1, Ordering::AcqRel);
    }
}

fn prepare_peer_update_stream(stream: &TcpStream) -> Result<()> {
    stream.set_read_timeout(Some(PEER_UPDATE_IO_TIMEOUT))?;
    stream.set_write_timeout(Some(PEER_UPDATE_IO_TIMEOUT))?;
    Ok(())
}

fn handle_release_request(
    stream: &mut TcpStream,
    root: &Path,
    listen: &str,
    public_base_url: Option<&str>,
    discovery_snapshot_path: Option<&Path>,
    discovery_pubkey_path: Option<&Path>,
) -> Result<()> {
    let request = read_http_request(stream)?;
    let request_line = request
        .lines()
        .next()
        .ok_or("empty HTTP request")?
        .trim()
        .to_string();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("missing HTTP method")?;
    let raw_path = parts.next().ok_or("missing HTTP path")?;
    if method != "GET" && method != "HEAD" {
        write_simple_response(stream, "405 Method Not Allowed", "method not allowed\n")?;
        return Ok(());
    }
    let path = raw_path.split('?').next().unwrap_or(raw_path);
    let host = request_host_header(&request);
    let base_url = match public_base_url {
        Some(value) => value.to_string(),
        None => infer_base_url(host.as_deref(), listen)?,
    };
    let include_body = method == "GET";
    match path {
        "/health" => {
            let body = format!("status=ok version={}\n", read_release_version(root)?);
            write_bytes_response(
                stream,
                "200 OK",
                "text/plain; charset=utf-8",
                body.as_bytes(),
                include_body,
            )?;
        }
        RELEASE_METADATA_ROUTE => {
            let body = render_metadata_json(root, &base_url)?;
            write_bytes_response(
                stream,
                "200 OK",
                "application/json",
                body.as_bytes(),
                include_body,
            )?;
        }
        DISCOVERY_SNAPSHOT_ROUTE => {
            write_optional_file_response(
                stream,
                discovery_snapshot_path,
                "application/json",
                include_body,
            )?;
        }
        DISCOVERY_PUBKEY_ROUTE => {
            write_optional_file_response(
                stream,
                discovery_pubkey_path,
                "text/plain; charset=utf-8",
                include_body,
            )?;
        }
        "/chimera.sh" => {
            let body = generate_peer_bootstrap_script(root, &base_url)?;
            write_bytes_response(
                stream,
                "200 OK",
                "application/x-sh",
                body.as_bytes(),
                include_body,
            )?;
        }
        RELEASE_ARCHIVE_ROUTE => {
            write_file_response(
                stream,
                &release_archive_path(root),
                "application/octet-stream",
                include_body,
            )?;
        }
        RELEASE_CHECKSUM_ROUTE => {
            write_file_response(
                stream,
                &release_checksum_path(root),
                "text/plain; charset=utf-8",
                include_body,
            )?;
        }
        _ => {
            write_simple_response(stream, "404 Not Found", "not found\n")?;
        }
    }
    Ok(())
}

fn sibling_cache_artifact_path(state_file: Option<&Path>, name: &str) -> Option<PathBuf> {
    state_file.and_then(|path| path.parent().map(|parent| parent.join(name)))
}

fn read_http_request(stream: &mut TcpStream) -> Result<String> {
    let started = Instant::now();
    let mut request = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        if started.elapsed() > PEER_UPDATE_HEADER_TIMEOUT {
            return Err("HTTP request header timed out".into());
        }
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") || request.len() > 16 * 1024 {
            break;
        }
    }
    if request.len() > 16 * 1024 {
        return Err("HTTP request header too large".into());
    }
    String::from_utf8(request).map_err(|_| "HTTP request is not UTF-8".into())
}

fn request_host_header(request: &str) -> Option<String> {
    for line in request.lines().skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("host") {
            let host = value.trim();
            if !host.is_empty() {
                return Some(host.to_string());
            }
        }
    }
    None
}

pub(super) fn infer_base_url(host: Option<&str>, listen: &str) -> Result<String> {
    let raw = host.unwrap_or(listen).trim();
    validate_header_host(raw)?;
    Ok(format!("http://{raw}"))
}

pub(super) fn validate_header_host(host: &str) -> Result<()> {
    if host.is_empty()
        || host.contains('/')
        || host.contains('\\')
        || host.contains('@')
        || host.contains('"')
        || host.contains('\'')
        || host.contains('`')
        || host.contains('$')
        || host.contains('?')
        || host.contains('#')
        || host.contains('\r')
        || host.contains('\n')
        || host.contains('\t')
        || host.chars().any(char::is_whitespace)
    {
        return Err("invalid update host header".into());
    }
    Ok(())
}

pub(super) fn validate_base_url(base_url: &str) -> Result<()> {
    if base_url != base_url.trim() {
        return Err("peer update base URL contains surrounding spaces".into());
    }
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err("peer update base URL must be http(s)".into());
    }
    if base_url.contains('"')
        || base_url.contains('\'')
        || base_url.contains('`')
        || base_url.contains('$')
        || base_url.contains('@')
        || base_url.contains('?')
        || base_url.contains('#')
        || base_url.contains('\\')
        || base_url.contains('\r')
        || base_url.contains('\n')
        || base_url.contains('\t')
        || base_url.chars().any(char::is_whitespace)
    {
        return Err("peer update base URL contains invalid characters".into());
    }
    Ok(())
}

pub(super) fn advertised_base_url(
    public_base_url: Option<&str>,
    bound_addr: SocketAddr,
) -> Result<Option<String>> {
    let Some(base_url) = public_base_url else {
        return Ok(None);
    };
    validate_base_url(base_url)?;
    let mut parsed = Url::parse(base_url)?;
    if parsed.host_str().is_none() {
        return Err("peer update base URL missing host".into());
    }
    if parsed.port().is_none() || parsed.port() == Some(0) {
        parsed
            .set_port(Some(bound_addr.port()))
            .map_err(|_| "peer update base URL port invalid")?;
    }
    Ok(Some(parsed.as_str().trim_end_matches('/').to_string()))
}

fn join_url(base_url: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

pub(super) fn generate_peer_bootstrap_script(root: &Path, base_url: &str) -> Result<String> {
    validate_base_url(base_url)?;
    let script_path = root.join("scripts").join("chimera.sh");
    let version = read_release_version(root)?;
    let archive_url = join_url(base_url, RELEASE_ARCHIVE_NAME);
    let checksum_url = join_url(base_url, RELEASE_CHECKSUM_NAME);
    let script = fs::read_to_string(script_path)?;
    let mut out = String::with_capacity(script.len() + archive_url.len() + checksum_url.len());
    let mut saw_version = false;
    let mut saw_archive = false;
    let mut saw_checksum = false;
    for line in script.lines() {
        if line.starts_with("VERSION=") {
            out.push_str(&format!("VERSION=\"{version}\"\n"));
            saw_version = true;
        } else if line.starts_with("ARCHIVE_URL_DEFAULT=") {
            out.push_str(&format!("ARCHIVE_URL_DEFAULT=\"{archive_url}\"\n"));
            saw_archive = true;
        } else if line.starts_with("CHECKSUM_URL_DEFAULT=") {
            out.push_str(&format!("CHECKSUM_URL_DEFAULT=\"{checksum_url}\"\n"));
            saw_checksum = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !(saw_version && saw_archive && saw_checksum) {
        return Err("bootstrap script is missing required release metadata".into());
    }
    Ok(out)
}

pub(super) fn render_metadata_json(root: &Path, base_url: &str) -> Result<String> {
    validate_base_url(base_url)?;
    let version = read_release_version(root)?;
    let checksum = read_release_checksum(root)?;
    Ok(format!(
        "{{\"status\":\"ok\",\"kind\":\"chimera_peer_update_metadata\",\"version\":\"{}\",\"archive\":\"{}\",\"checksum\":\"{}\",\"sha256\":\"{}\"}}\n",
        json_escape(&version),
        json_escape(&join_url(base_url, RELEASE_ARCHIVE_NAME)),
        json_escape(&join_url(base_url, RELEASE_CHECKSUM_NAME)),
        json_escape(&checksum)
    ))
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn write_file_response(
    stream: &mut TcpStream,
    path: &Path,
    content_type: &str,
    include_body: bool,
) -> Result<()> {
    require_file(path)?;
    let len = fs::metadata(path)?.len();
    write_response_header(stream, "200 OK", content_type, len)?;
    if include_body {
        let mut file = File::open(path)?;
        io::copy(&mut file, stream)?;
        stream.flush()?;
    }
    Ok(())
}

fn write_optional_file_response(
    stream: &mut TcpStream,
    path: Option<&Path>,
    content_type: &str,
    include_body: bool,
) -> Result<()> {
    let Some(path) = path else {
        write_simple_response(stream, "404 Not Found", "not found\n")?;
        return Ok(());
    };
    if !path.is_file() {
        write_simple_response(stream, "404 Not Found", "not found\n")?;
        return Ok(());
    }
    write_file_response(stream, path, content_type, include_body)
}

fn write_bytes_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
    include_body: bool,
) -> Result<()> {
    write_response_header(stream, status, content_type, body.len() as u64)?;
    if include_body {
        stream.write_all(body)?;
        stream.flush()?;
    }
    Ok(())
}

fn write_simple_response(stream: &mut TcpStream, status: &str, body: &str) -> Result<()> {
    write_bytes_response(
        stream,
        status,
        "text/plain; charset=utf-8",
        body.as_bytes(),
        true,
    )
}

fn write_response_header(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    len: u64,
) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n"
    )?;
    Ok(())
}

fn read_release_version(root: &Path) -> Result<String> {
    let version = fs::read_to_string(root.join(".chimera_release_version"))?
        .trim()
        .to_string();
    if version.is_empty()
        || version.contains('"')
        || version.contains('\'')
        || version.contains('\r')
        || version.contains('\n')
    {
        return Err("invalid CHIMERA release version metadata".into());
    }
    Ok(version)
}

pub(super) fn read_release_checksum(root: &Path) -> Result<String> {
    let checksum = fs::read_to_string(release_checksum_path(root))?
        .split_whitespace()
        .next()
        .ok_or("empty release checksum file")?
        .to_string();
    if checksum.len() != 64 || !checksum.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err("invalid CHIMERA release checksum metadata".into());
    }
    Ok(checksum)
}

fn release_archive_path(root: &Path) -> PathBuf {
    root.join("releases").join(RELEASE_ARCHIVE_NAME)
}

fn release_checksum_path(root: &Path) -> PathBuf {
    root.join("releases").join(RELEASE_CHECKSUM_NAME)
}

fn require_file(path: &Path) -> Result<()> {
    if !path.is_file() {
        return Err(format!("required file not found: {}", path.display()).into());
    }
    Ok(())
}
