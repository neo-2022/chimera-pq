use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use tar::Archive;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const RELEASE_ARCHIVE_NAME: &str = "chimera-pq-release.tar.gz";
const RELEASE_CHECKSUM_NAME: &str = "chimera-pq-release.tar.gz.sha256";
const PEER_UPDATE_IO_TIMEOUT: Duration = Duration::from_secs(5);
const PEER_UPDATE_HEADER_TIMEOUT: Duration = Duration::from_secs(10);
const PEER_UPDATE_MAX_ACTIVE_CONNECTIONS: usize = 32;

fn main() {
    if let Err(err) = run() {
        eprintln!("chimera-bootstrap error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("download") => {
            let url = take_flag_value(&mut args, "--url")?;
            let output = take_flag_value(&mut args, "--output")?;
            download_to_file(&url, Path::new(&output))
        }
        Some("verify") => {
            let file = take_flag_value(&mut args, "--file")?;
            let sha256 = take_flag_value(&mut args, "--sha256")?;
            verify_sha256(Path::new(&file), &sha256)
        }
        Some("extract") => {
            let archive = take_flag_value(&mut args, "--archive")?;
            let dest = take_flag_value(&mut args, "--dest")?;
            let strip_components =
                take_optional_usize(&mut args, "--strip-components")?.unwrap_or(0);
            extract_tar_gz(Path::new(&archive), Path::new(&dest), strip_components)
        }
        Some("install") => {
            let url = take_flag_value(&mut args, "--url")?;
            let checksum_url = take_flag_value(&mut args, "--checksum-url")?;
            let dest = take_flag_value(&mut args, "--dest")?;
            let strip_components =
                take_optional_usize(&mut args, "--strip-components")?.unwrap_or(1);
            install_bundle(&url, &checksum_url, Path::new(&dest), strip_components)
        }
        Some("serve-release") => {
            let root = take_flag_value(&mut args, "--root")?;
            let listen = take_flag_value(&mut args, "--listen")?;
            let base_url = take_optional_string(&mut args, "--base-url")?;
            serve_release(Path::new(&root), &listen, base_url.as_deref())
        }
        _ => {
            eprintln!(
                "usage: chimera-bootstrap <download|verify|extract|install|serve-release> --flags..."
            );
            Err("invalid arguments".into())
        }
    }
}

fn take_flag_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    match args.next() {
        Some(actual) if actual == flag => {}
        Some(actual) => return Err(format!("expected {flag}, got {actual}").into()),
        None => return Err(format!("missing {flag}").into()),
    }
    args.next()
        .ok_or_else(|| format!("missing value for {flag}").into())
}

fn take_optional_usize(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<Option<usize>> {
    let mut collected = Vec::new();
    for item in args.by_ref() {
        collected.push(item);
    }
    if collected.is_empty() {
        return Ok(None);
    }
    if collected.len() != 2 || collected[0] != flag {
        return Err(format!("unexpected trailing arguments: {:?}", collected).into());
    }
    let parsed = collected[1]
        .parse::<usize>()
        .map_err(|e| format!("invalid {} value: {}", flag, e))?;
    Ok(Some(parsed))
}

fn take_optional_string(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<Option<String>> {
    let mut collected = Vec::new();
    for item in args.by_ref() {
        collected.push(item);
    }
    if collected.is_empty() {
        return Ok(None);
    }
    if collected.len() != 2 || collected[0] != flag {
        return Err(format!("unexpected trailing arguments: {:?}", collected).into());
    }
    Ok(Some(collected[1].clone()))
}

fn download_to_file(url: &str, output: &Path) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let resp = ureq::get(url).call()?;
    let mut reader = resp.into_reader();
    let mut file = File::create(output)?;
    io::copy(&mut reader, &mut file)?;
    file.flush()?;
    Ok(())
}

fn verify_sha256(file: &Path, expected_hex: &str) -> Result<()> {
    let mut hasher = Sha256::new();
    let mut reader = File::open(file)?;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected_hex.trim() {
        return Err(format!(
            "sha256 mismatch expected={} actual={}",
            expected_hex.trim(),
            actual
        )
        .into());
    }
    Ok(())
}

fn extract_tar_gz(archive: &Path, dest: &Path, strip_components: usize) -> Result<()> {
    fs::create_dir_all(dest)?;
    let file = File::open(archive)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    for entry_res in archive.entries()? {
        let mut entry = entry_res?;
        let path = entry.path()?.to_path_buf();
        let stripped = strip_path_components(&path, strip_components)?;
        if stripped.as_os_str().is_empty() {
            continue;
        }
        let out_path = dest.join(stripped);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        entry.unpack(out_path)?;
    }
    Ok(())
}

fn install_bundle(
    url: &str,
    checksum_url: &str,
    dest: &Path,
    strip_components: usize,
) -> Result<()> {
    let tmp_dir = temp_dir("chimera-bootstrap")?;
    let archive = tmp_dir.join("bundle.tar.gz");
    let checksum = tmp_dir.join("bundle.tar.gz.sha256");
    download_to_file(url, &archive)?;
    download_to_file(checksum_url, &checksum)?;
    let expected = fs::read_to_string(&checksum)?
        .split_whitespace()
        .next()
        .ok_or("empty checksum file")?
        .to_string();
    verify_sha256(&archive, &expected)?;
    extract_tar_gz(&archive, dest, strip_components)?;
    Ok(())
}

fn serve_release(root: &Path, listen: &str, public_base_url: Option<&str>) -> Result<()> {
    let version = read_release_version(root)?;
    let checksum = read_release_checksum(root)?;
    let archive = release_archive_path(root);
    let checksum_file = release_checksum_path(root);
    let script = root.join("scripts").join("chimera.sh");
    require_file(&archive)?;
    require_file(&checksum_file)?;
    require_file(&script)?;
    if let Some(base_url) = public_base_url {
        validate_base_url(base_url)?;
    }
    let listener = TcpListener::bind(listen)?;
    eprintln!(
        "chimera_peer_update_serve=ready listen={} version={} sha256={}",
        listen, version, checksum
    );
    let root = Arc::new(root.to_path_buf());
    let listen = listen.to_string();
    let public_base_url = public_base_url.map(str::to_string);
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
        "/metadata.json" => {
            let body = render_metadata_json(root, &base_url)?;
            write_bytes_response(
                stream,
                "200 OK",
                "application/json",
                body.as_bytes(),
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
        "/chimera-pq-release.tar.gz" => {
            write_file_response(
                stream,
                &release_archive_path(root),
                "application/octet-stream",
                include_body,
            )?;
        }
        "/chimera-pq-release.tar.gz.sha256" => {
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

fn infer_base_url(host: Option<&str>, listen: &str) -> Result<String> {
    let raw = host.unwrap_or(listen).trim();
    validate_header_host(raw)?;
    Ok(format!("http://{raw}"))
}

fn validate_header_host(host: &str) -> Result<()> {
    if host.is_empty()
        || host.contains('/')
        || host.contains('\\')
        || host.contains('"')
        || host.contains('\'')
        || host.contains('\r')
        || host.contains('\n')
    {
        return Err("invalid update host header".into());
    }
    Ok(())
}

fn validate_base_url(base_url: &str) -> Result<()> {
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err("peer update base URL must be http(s)".into());
    }
    if base_url.contains('"')
        || base_url.contains('\'')
        || base_url.contains('\r')
        || base_url.contains('\n')
    {
        return Err("peer update base URL contains invalid characters".into());
    }
    Ok(())
}

fn join_url(base_url: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

fn generate_peer_bootstrap_script(root: &Path, base_url: &str) -> Result<String> {
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

fn render_metadata_json(root: &Path, base_url: &str) -> Result<String> {
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

fn read_release_checksum(root: &Path) -> Result<String> {
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

fn temp_dir(prefix: &str) -> Result<PathBuf> {
    let mut base = env::temp_dir();
    let unique = format!(
        "{}-{}-{}",
        prefix,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    base.push(unique);
    fs::create_dir_all(&base)?;
    Ok(base)
}

fn strip_path_components(path: &Path, strip_components: usize) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    for (idx, comp) in path.components().enumerate() {
        if idx < strip_components {
            continue;
        }
        match comp {
            std::path::Component::Normal(part) => out.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                return Err("archive entry contains parent dir path".into());
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err("archive entry contains absolute path".into());
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{generate_peer_bootstrap_script, read_release_checksum, render_metadata_json};
    use std::fs;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::{Duration, Instant};

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

    fn fixture_root() -> Result<std::path::PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let root = super::temp_dir("chimera-bootstrap-test")?;
        fs::create_dir_all(root.join("scripts"))?;
        fs::create_dir_all(root.join("releases"))?;
        fs::write(root.join(".chimera_release_version"), "1.2.3\n")?;
        fs::write(
            root.join("scripts").join("chimera.sh"),
            "#!/usr/bin/env bash\nVERSION=\"0.0.0-dev\"\nARCHIVE_URL_DEFAULT=\"https://example.invalid/archive.tar.gz\"\nCHECKSUM_URL_DEFAULT=\"https://example.invalid/archive.tar.gz.sha256\"\n",
        )?;
        fs::write(
            root.join("releases").join("chimera-pq-release.tar.gz"),
            b"bundle",
        )?;
        fs::write(
            root.join("releases")
                .join("chimera-pq-release.tar.gz.sha256"),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  chimera-pq-release.tar.gz\n",
        )?;
        Ok(root)
    }

    fn fixture_server_addr() -> TestResult<(std::path::PathBuf, String, thread::JoinHandle<()>)> {
        let root = fixture_root()?;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?.to_string();
        drop(listener);
        let server_root = root.clone();
        let server_addr = addr.clone();
        let handle = thread::spawn(move || {
            let _ = super::serve_release(&server_root, &server_addr, None);
        });
        wait_for_server(&addr)?;
        Ok((root, addr, handle))
    }

    fn wait_for_server(addr: &str) -> TestResult {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if TcpStream::connect(addr).is_ok() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(20));
        }
        Err("server did not start".into())
    }

    fn get_path(addr: &str, path: &str) -> TestResult<String> {
        let mut stream = TcpStream::connect(addr)?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
        )?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        Ok(response)
    }

    #[test]
    fn peer_bootstrap_rewrites_release_urls() -> TestResult {
        let root = fixture_root()?;
        let script = generate_peer_bootstrap_script(&root, "http://node.example:18179")?;
        assert!(script.contains("VERSION=\"1.2.3\""));
        assert!(script.contains(
            "ARCHIVE_URL_DEFAULT=\"http://node.example:18179/chimera-pq-release.tar.gz\""
        ));
        assert!(script.contains(
            "CHECKSUM_URL_DEFAULT=\"http://node.example:18179/chimera-pq-release.tar.gz.sha256\""
        ));
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn metadata_contains_version_and_sha() -> TestResult {
        let root = fixture_root()?;
        let metadata = render_metadata_json(&root, "http://node.example:18179")?;
        assert!(metadata.contains("\"version\":\"1.2.3\""));
        assert!(metadata.contains("\"kind\":\"chimera_peer_update_metadata\""));
        assert!(metadata.contains(
            "\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\""
        ));
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn checksum_reader_rejects_missing_checksum() -> TestResult {
        let root = fixture_root()?;
        fs::write(
            root.join("releases")
                .join("chimera-pq-release.tar.gz.sha256"),
            "\n",
        )?;
        assert!(read_release_checksum(&root).is_err());
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn peer_release_idle_tcp_client_does_not_block_metadata() -> TestResult {
        let (root, addr, _server) = fixture_server_addr()?;
        let idle = TcpStream::connect(&addr)?;
        idle.set_read_timeout(Some(Duration::from_millis(200)))?;

        let response = get_path(&addr, "/metadata.json")?;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"kind\":\"chimera_peer_update_metadata\""));
        drop(idle);
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn peer_release_partial_header_client_does_not_block_health() -> TestResult {
        let (root, addr, _server) = fixture_server_addr()?;
        let mut slow = TcpStream::connect(&addr)?;
        slow.set_write_timeout(Some(Duration::from_secs(1)))?;
        slow.write_all(b"GET /metadata.json HTTP/1.1\r\nHost: ")?;

        let response = get_path(&addr, "/health")?;

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("status=ok version=1.2.3"));
        drop(slow);
        fs::remove_dir_all(root)?;
        Ok(())
    }
}
