use super::*;

#[test]
fn parse_sealed_transit_inject_allows_tokenless_proof_mode() -> Result<(), String> {
    let args = vec![
        "--mode".to_string(),
        "sealed-transit-inject".to_string(),
        "--server".to_string(),
        "127.0.0.1:18135".to_string(),
        "--transit-payload-bytes".to_string(),
        "128".to_string(),
        "--transit-packet-number".to_string(),
        "5".to_string(),
    ];
    let parsed = Options::parse(&args)?;

    assert_eq!(parsed.mode, Mode::SealedTransitInject);
    assert_eq!(parsed.server, "127.0.0.1:18135");
    assert_eq!(parsed.token, "");
    assert_eq!(parsed.transit_payload_bytes, 128);
    assert_eq!(parsed.transit_packet_number, 5);
    Ok(())
}

#[test]
fn parse_non_proof_mode_rejects_transit_proof_cli_flags() {
    let args = vec![
        "--mode".to_string(),
        "bench".to_string(),
        "--token".to_string(),
        "abc".to_string(),
        "--transit-payload-bytes".to_string(),
        "64".to_string(),
    ];
    let error = match Options::parse(&args) {
        Ok(_) => "bench mode with transit proof flag should fail".to_string(),
        Err(error) => error,
    };

    assert!(error.contains("transit proof flags"));
}

#[test]
fn parse_bound_transit_inject_requires_route_and_lane() {
    let args = vec![
        "--mode".to_string(),
        "bound-transit-inject".to_string(),
        "--server".to_string(),
        "127.0.0.1:18135".to_string(),
    ];
    let error = match Options::parse(&args) {
        Ok(_) => "bound transit inject without binding should fail".to_string(),
        Err(error) => error,
    };

    assert!(error.contains("transit-route-id"));
}

#[test]
fn parse_bound_transit_inject_accepts_explicit_binding() -> Result<(), String> {
    let args = vec![
        "--mode".to_string(),
        "bound-transit-inject".to_string(),
        "--server".to_string(),
        "127.0.0.1:18135".to_string(),
        "--transit-route-id".to_string(),
        "77".to_string(),
        "--transit-lane-index".to_string(),
        "0".to_string(),
        "--transit-payload-bytes".to_string(),
        "64".to_string(),
    ];
    let parsed = Options::parse(&args)?;

    assert_eq!(parsed.mode, Mode::BoundTransitInject);
    assert_eq!(parsed.transit_route_id, Some(77));
    assert_eq!(parsed.transit_lane_index, Some(0));
    assert_eq!(mode_name(&parsed.mode), "bound-transit-inject");
    Ok(())
}
