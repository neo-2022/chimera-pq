use super::*;

mod debug_redaction;
mod proof;
mod state_file;
mod transit_guard;

#[test]
fn parse_rejects_empty_token() {
    let args = vec![
        "--mode".to_string(),
        "side-a".to_string(),
        "--local-listen".to_string(),
        "127.0.0.1:0".to_string(),
        "--peer-listen".to_string(),
        "127.0.0.1:0".to_string(),
        "--token".to_string(),
        String::new(),
    ];
    assert!(Options::parse(&args).is_err());
}

#[test]
fn parse_side_a_requires_explicit_listeners() {
    let args = vec![
        "--mode".to_string(),
        "side-a".to_string(),
        "--token".to_string(),
        "abc".to_string(),
    ];
    assert!(Options::parse(&args).is_err());
}

#[test]
fn parse_side_b_options() {
    let args = vec![
        "--mode".to_string(),
        "side-b".to_string(),
        "--server".to_string(),
        "mesh-node.example.invalid:443".to_string(),
        "--token".to_string(),
        "abc".to_string(),
    ];
    let parsed = Options::parse(&args).unwrap_or_else(|error| {
        unreachable!("options should parse: {error}");
    });
    assert_eq!(parsed.mode, Mode::SideB);
    assert_eq!(parsed.pool, 8);
    assert!(!parsed.allow_pool_transit);
    assert!(!parsed.allow_bound_transit);
}

#[test]
fn options_debug_redacts_token() -> Result<(), String> {
    let args = vec![
        "--mode".to_string(),
        "side-b".to_string(),
        "--server".to_string(),
        "mesh-node.example.invalid:443".to_string(),
        "--token".to_string(),
        "SECRET_TOKEN_SENTINEL".to_string(),
    ];
    let parsed = Options::parse(&args)?;
    let debug = format!("{parsed:?}");
    assert!(debug.contains("token"));
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("SECRET_TOKEN_SENTINEL"));
    Ok(())
}

#[test]
fn options_debug_redacts_transit_lane_bindings_file() -> Result<(), String> {
    let args = vec![
        "--mode".to_string(),
        "node".to_string(),
        "--local-listen".to_string(),
        "127.0.0.1:18135".to_string(),
        "--peer-listen".to_string(),
        "0.0.0.0:8443".to_string(),
        "--server".to_string(),
        "peer.example.invalid:8443".to_string(),
        "--token".to_string(),
        "abc".to_string(),
        "--allow-bound-transit".to_string(),
        "true".to_string(),
        "--transit-lane-bindings-file".to_string(),
        "/tmp/SECRET_BINDINGS_SENTINEL.csv".to_string(),
    ];
    let parsed = Options::parse(&args)?;
    let debug = format!("{parsed:?}");
    assert!(debug.contains("transit_lane_bindings_file"));
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("SECRET_BINDINGS_SENTINEL"));
    Ok(())
}

#[test]
fn parse_node_options_requires_ingress_listeners_and_keeps_peer_optional() -> Result<(), String> {
    let args = vec![
        "--mode".to_string(),
        "node".to_string(),
        "--local-listen".to_string(),
        "127.0.0.1:18135".to_string(),
        "--peer-listen".to_string(),
        "0.0.0.0:8443".to_string(),
        "--token".to_string(),
        "abc".to_string(),
    ];
    let parsed = Options::parse(&args)?;
    assert_eq!(parsed.mode, Mode::Node);
    assert_eq!(parsed.local_listen, "127.0.0.1:18135");
    assert_eq!(parsed.peer_listen, "0.0.0.0:8443");
    assert_eq!(parsed.server, "");
    assert_eq!(mode_name(&parsed.mode), "node");
    Ok(())
}

#[test]
fn parse_node_options_defaults_ingress_listeners_to_auto_bind() -> Result<(), String> {
    let args = vec![
        "--mode".to_string(),
        "node".to_string(),
        "--token".to_string(),
        "abc".to_string(),
    ];
    let parsed = Options::parse(&args)?;
    assert_eq!(parsed.mode, Mode::Node);
    assert_eq!(parsed.local_listen, NODE_DEFAULT_LOCAL_LISTEN);
    assert_eq!(parsed.peer_listen, NODE_DEFAULT_PEER_LISTEN);
    assert_eq!(parsed.server, "");
    Ok(())
}

#[test]
fn parse_node_options_accepts_outbound_peer_endpoint() -> Result<(), String> {
    let args = vec![
        "--mode".to_string(),
        "weave-node".to_string(),
        "--local-listen".to_string(),
        "127.0.0.1:18135".to_string(),
        "--peer-listen".to_string(),
        "0.0.0.0:8443".to_string(),
        "--server".to_string(),
        "peer.example.invalid:8443".to_string(),
        "--token".to_string(),
        "abc".to_string(),
    ];
    let parsed = Options::parse(&args)?;
    assert_eq!(parsed.mode, Mode::Node);
    assert_eq!(parsed.server, "peer.example.invalid:8443");
    Ok(())
}

#[test]
fn parse_node_options_accepts_explicit_pool_transit_policy() -> Result<(), String> {
    let args = vec![
        "--mode".to_string(),
        "node".to_string(),
        "--local-listen".to_string(),
        "127.0.0.1:18135".to_string(),
        "--peer-listen".to_string(),
        "0.0.0.0:8443".to_string(),
        "--server".to_string(),
        "peer.example.invalid:8443".to_string(),
        "--token".to_string(),
        "abc".to_string(),
        "--allow-pool-transit".to_string(),
        "true".to_string(),
    ];
    let parsed = Options::parse(&args)?;
    assert!(parsed.allow_pool_transit);
    assert!(!parsed.allow_bound_transit);
    Ok(())
}

#[test]
fn parse_node_options_accepts_bound_transit_policy_separately() -> Result<(), String> {
    let args = vec![
        "--mode".to_string(),
        "node".to_string(),
        "--local-listen".to_string(),
        "127.0.0.1:18135".to_string(),
        "--peer-listen".to_string(),
        "0.0.0.0:8443".to_string(),
        "--server".to_string(),
        "peer.example.invalid:8443".to_string(),
        "--token".to_string(),
        "abc".to_string(),
        "--allow-bound-transit".to_string(),
        "true".to_string(),
        "--transit-lane-bindings-file".to_string(),
        "/tmp/chimera-test-bindings.csv".to_string(),
    ];
    let parsed = Options::parse(&args)?;
    assert!(!parsed.allow_pool_transit);
    assert!(parsed.allow_bound_transit);
    assert_eq!(
        parsed.transit_lane_bindings_file.as_deref(),
        Some("/tmp/chimera-test-bindings.csv")
    );
    Ok(())
}

#[test]
fn parse_bench_options() {
    let args = vec![
        "--mode".to_string(),
        "bench".to_string(),
        "--token".to_string(),
        "abc".to_string(),
        "--bench-bytes".to_string(),
        "1024".to_string(),
        "--min-throughput-mib-s".to_string(),
        "100".to_string(),
        "--connections".to_string(),
        "4".to_string(),
    ];
    let parsed = Options::parse(&args).unwrap_or_else(|error| {
        unreachable!("options should parse: {error}");
    });
    assert_eq!(parsed.mode, Mode::Bench);
    assert_eq!(parsed.bench_bytes, 1024);
    assert_eq!(parsed.min_throughput_mib_s, 100);
    assert_eq!(parsed.connections, 4);
}

#[test]
fn parse_probe_requires_target() {
    let args = vec![
        "--mode".to_string(),
        "probe".to_string(),
        "--server".to_string(),
        "127.0.0.1:1".to_string(),
        "--token".to_string(),
        "abc".to_string(),
    ];
    assert!(Options::parse(&args).is_err());
}

#[test]
fn parse_download_probe_options() {
    let args = vec![
        "--mode".to_string(),
        "download-probe".to_string(),
        "--server".to_string(),
        "127.0.0.1:1".to_string(),
        "--target".to_string(),
        "node.example.invalid:443".to_string(),
        "--token".to_string(),
        "abc".to_string(),
        "--connections".to_string(),
        "2".to_string(),
    ];
    let parsed = Options::parse(&args).unwrap_or_else(|error| {
        unreachable!("options should parse: {error}");
    });
    assert_eq!(parsed.mode, Mode::DownloadProbe);
    assert_eq!(parsed.connections, 2);
}

#[test]
fn parse_aead_options() {
    let args = vec![
        "--mode".to_string(),
        "bench".to_string(),
        "--token".to_string(),
        "abc".to_string(),
        "--aead".to_string(),
        "aes256gcm".to_string(),
    ];
    let parsed = Options::parse(&args).unwrap_or_else(|error| {
        unreachable!("options should parse: {error}");
    });
    assert_eq!(parsed.aead, AeadSuite::Aes256Gcm);

    let mut bad = args;
    bad[5] = "weak".to_string();
    assert!(Options::parse(&bad).is_err());
}

#[test]
fn parse_rejects_zero_connect_timeout() {
    let args = vec![
        "--mode".to_string(),
        "bench".to_string(),
        "--token".to_string(),
        "abc".to_string(),
        "--connect-timeout-ms".to_string(),
        "0".to_string(),
    ];
    assert!(Options::parse(&args).is_err());
}

#[test]
fn parse_rejects_zero_connections() {
    let args = vec![
        "--mode".to_string(),
        "bench".to_string(),
        "--token".to_string(),
        "abc".to_string(),
        "--connections".to_string(),
        "0".to_string(),
    ];
    assert!(Options::parse(&args).is_err());
}
