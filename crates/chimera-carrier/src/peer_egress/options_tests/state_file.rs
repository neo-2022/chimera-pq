use super::*;

#[test]
fn write_resolved_state_file_creates_private_file() -> Result<(), String> {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "chimera_peer_egress_state_write_{}.state",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    write_resolved_state_file(
        path.to_str().ok_or_else(|| "state path utf8".to_string())?,
        &Mode::Node,
        "127.0.0.1:11111",
        "198.51.100.44:45678",
    )?;
    let body = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    assert!(body.contains("resolved_peer_listen=198.51.100.44:45678"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path)
            .map_err(|error| error.to_string())?
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn split_host_port_accepts_valid_target() {
    let parsed = split_host_port("node.example.invalid:443")
        .unwrap_or_else(|error| unreachable!("target should parse: {error}"));
    assert_eq!(parsed, ("node.example.invalid".to_string(), 443));
}

#[test]
fn throughput_gate_rejects_slow_path() {
    assert!(enforce_min_throughput(99.9, 100).is_err());
    assert!(enforce_min_throughput(100.0, 100).is_ok());
}
