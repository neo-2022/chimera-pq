use super::server::{
    generate_peer_bootstrap_script, infer_base_url, read_release_checksum, render_metadata_json,
    validate_base_url, validate_header_host,
};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn temp_dir(prefix: &str) -> TestResult<std::path::PathBuf> {
    let mut base = std::env::temp_dir();
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

fn fixture_root() -> TestResult<std::path::PathBuf> {
    let root = temp_dir("chimera-bootstrap-test")?;
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
        let _ = super::server::serve_release(&server_root, &server_addr, None, None);
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
    assert!(
        script.contains(
            "ARCHIVE_URL_DEFAULT=\"http://node.example:18179/chimera-pq-release.tar.gz\""
        )
    );
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
fn base_url_validation_rejects_injection_characters() {
    let invalid = [
        " http://node.example:18179",
        "http://node.example:18179 ",
        "http://node example:18179",
        "http://node.example:18179?archive=evil",
        "http://node.example:18179#frag",
        "http://node.example:18179\\evil",
        "http://node.example:18179$evil",
        "http://node.example:18179`evil",
        "http://user@node.example:18179",
        "ftp://node.example:18179",
    ];
    for value in invalid {
        assert!(
            validate_base_url(value).is_err(),
            "accepted invalid base URL: {value:?}"
        );
    }
}

#[test]
fn host_validation_rejects_injection_characters() {
    let invalid = [
        "",
        "node example:18179",
        "node.example:18179/path",
        "node.example:18179?archive=evil",
        "node.example:18179#frag",
        "node.example:18179\\evil",
        "node.example:18179$evil",
        "node.example:18179`evil",
        "user@node.example:18179",
        "node.example:18179\nX-Evil: yes",
        "node.example:18179\t",
    ];
    for value in invalid {
        assert!(
            validate_header_host(value).is_err(),
            "accepted invalid host: {value:?}"
        );
    }
}

#[test]
fn inferred_base_url_uses_valid_host_only() -> TestResult {
    assert_eq!(
        infer_base_url(Some("node.example:18179"), "127.0.0.1:18179")?,
        "http://node.example:18179"
    );
    assert!(infer_base_url(Some("node.example:18179?evil"), "127.0.0.1:18179").is_err());
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
