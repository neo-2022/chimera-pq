use super::*;

#[test]
fn parse_node_options_accepts_transit_relay_guard_limits() -> Result<(), String> {
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
        "--transit-max-frames-per-direction".to_string(),
        "12".to_string(),
        "--transit-max-bytes-per-direction".to_string(),
        "4096".to_string(),
        "--transit-idle-timeout-ms".to_string(),
        "2500".to_string(),
    ];

    let parsed = Options::parse(&args)?;

    assert_eq!(parsed.transit_max_frames_per_direction, 12);
    assert_eq!(parsed.transit_max_bytes_per_direction, 4096);
    assert_eq!(parsed.transit_idle_timeout_ms, 2500);
    assert_eq!(parsed.transit_relay_limits().max_frames_per_direction, 12);
    Ok(())
}

#[test]
fn parse_node_options_rejects_zero_transit_relay_guard_limits() {
    let base_args = [
        "--mode",
        "node",
        "--local-listen",
        "127.0.0.1:18135",
        "--peer-listen",
        "0.0.0.0:8443",
        "--server",
        "peer.example.invalid:8443",
        "--token",
        "abc",
    ];

    for flag in [
        "--transit-max-frames-per-direction",
        "--transit-max-bytes-per-direction",
        "--transit-idle-timeout-ms",
    ] {
        let mut args = base_args
            .iter()
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>();
        args.push(flag.to_string());
        args.push("0".to_string());
        assert!(Options::parse(&args).is_err(), "{flag} must reject zero");
    }
}
