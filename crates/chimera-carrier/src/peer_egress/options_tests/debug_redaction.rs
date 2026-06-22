use super::*;

#[test]
fn options_debug_redacts_transit_route_and_lane_identifiers() -> Result<(), String> {
    let args = vec![
        "--mode".to_string(),
        "bound-transit-inject".to_string(),
        "--server".to_string(),
        "127.0.0.1:18135".to_string(),
        "--transit-payload-bytes".to_string(),
        "64".to_string(),
        "--transit-packet-number".to_string(),
        "9".to_string(),
        "--transit-route-id".to_string(),
        "77".to_string(),
        "--transit-lane-index".to_string(),
        "3".to_string(),
    ];
    let parsed = Options::parse(&args)?;
    let debug = format!("{parsed:?}");

    assert!(debug.contains("transit_route_id"));
    assert!(debug.contains("transit_lane_index"));
    assert!(debug.contains("<opaque>"));
    assert!(!debug.contains("Some(77)"));
    assert!(!debug.contains("Some(3)"));
    Ok(())
}
