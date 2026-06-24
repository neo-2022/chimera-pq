use super::server::advertised_base_url;
use std::fs;
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
    let root = temp_dir("chimera-bootstrap-auto-port-test")?;
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

fn wait_for_state_file(path: &std::path::Path) -> TestResult<String> {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        match fs::read_to_string(path) {
            Ok(text) if !text.trim().is_empty() => return Ok(text),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Box::new(error)),
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err("state file was not written".into())
}

#[test]
fn advertised_base_url_uses_os_selected_port_when_port_is_omitted() -> TestResult {
    let bound_addr = "127.0.0.1:34567".parse()?;
    assert_eq!(
        advertised_base_url(Some("http://node.example"), bound_addr)?,
        Some("http://node.example:34567".to_string())
    );
    assert_eq!(
        advertised_base_url(Some("http://node.example:0"), bound_addr)?,
        Some("http://node.example:34567".to_string())
    );
    assert_eq!(
        advertised_base_url(Some("http://node.example:18179"), bound_addr)?,
        Some("http://node.example:18179".to_string())
    );
    Ok(())
}

#[test]
fn peer_release_state_file_records_os_selected_update_url() -> TestResult {
    let root = fixture_root()?;
    let state_file = root.join("peer-update-state.json");
    let server_root = root.clone();
    let server_state_file = state_file.clone();
    let _server = thread::spawn(move || {
        let _ = super::server::serve_release(
            &server_root,
            "127.0.0.1:0",
            Some("http://node.example"),
            Some(&server_state_file),
        );
    });

    let body = wait_for_state_file(&state_file)?;
    let state: serde_json::Value = serde_json::from_str(&body)?;
    let listen = state
        .get("listen")
        .and_then(serde_json::Value::as_str)
        .ok_or("state missing listen")?;
    let update_bootstrap_url = state
        .get("update_bootstrap_url")
        .and_then(serde_json::Value::as_str)
        .ok_or("state missing update_bootstrap_url")?;

    assert!(listen.starts_with("127.0.0.1:"));
    assert!(!listen.ends_with(":0"));
    let port = listen.rsplit_once(':').ok_or("listen missing port")?.1;
    assert_eq!(
        update_bootstrap_url,
        format!("http://node.example:{port}/chimera.sh")
    );
    assert_eq!(
        state.get("kind").and_then(serde_json::Value::as_str),
        Some("chimera_peer_update_serve_state")
    );
    assert_eq!(
        state.get("status").and_then(serde_json::Value::as_str),
        Some("ready")
    );
    fs::remove_dir_all(root)?;
    Ok(())
}
