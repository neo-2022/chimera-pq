use super::*;
use crate::peer_egress::lane_binding::{TransitLaneDocument, TransitLaneRegistration};
use crate::peer_egress::options::{AeadSuite, Mode, Options};
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};

fn binding(route: u64, lane: u16) -> TransitPathBinding {
    TransitPathBinding::new(
        TransitRouteId::new(route).unwrap_or_else(|error| unreachable!("{error}")),
        TransitLaneId::new(lane).unwrap_or_else(|error| unreachable!("{error}")),
    )
}

fn registration(route: u64, lane: u16, endpoint: &str) -> Result<TransitLaneRegistration, String> {
    TransitLaneRegistration::new(binding(route, lane), endpoint.to_string())
}

fn options_with_lane_file(path: &str, allow_bound_transit: bool) -> Options {
    Options {
        mode: Mode::Node,
        local_listen: "127.0.0.1:18135".to_string(),
        peer_listen: "127.0.0.1:0".to_string(),
        state_file: None,
        server: String::new(),
        token: "test-token".to_string(),
        pool: 1,
        bench_bytes: 1024,
        target: String::new(),
        connect_timeout_ms: 100,
        min_throughput_mib_s: 0,
        connections: 1,
        aead: AeadSuite::Chacha20Poly1305,
        reverse_connect: false,
        allow_pool_transit: false,
        allow_bound_transit,
        transit_lane_bindings_file: Some(path.to_string()),
        transit_max_frames_per_direction:
            crate::peer_egress::transit_guard::DEFAULT_TRANSIT_MAX_FRAMES_PER_DIRECTION,
        transit_max_bytes_per_direction:
            crate::peer_egress::transit_guard::DEFAULT_TRANSIT_MAX_BYTES_PER_DIRECTION,
        transit_idle_timeout_ms: crate::peer_egress::transit_guard::DEFAULT_TRANSIT_IDLE_TIMEOUT_MS,
        transit_payload_bytes: 64,
        transit_packet_number: 1,
        transit_route_id: None,
        transit_lane_index: None,
    }
}

#[test]
fn live_registry_rejects_any_registration_only_document() -> Result<(), String> {
    let document = TransitLaneDocument::new(vec![registration(77, 1, "198.51.100.77:443")?], None);
    let error = match validate_live_transit_lane_document_contract(
        &options_with_lane_file("/tmp/chimera-registration-only.csv", false),
        &document,
    ) {
        Ok(_) => return Err("live transit document without bound transit must fail".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("allow_bound_transit=true"));

    let error = match validate_live_transit_lane_document_contract(
        &options_with_lane_file("/tmp/chimera-registration-only.csv", true),
        &document,
    ) {
        Ok(_) => return Err("bound transit must reject registration-only document".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("mesh plan snapshot"));
    Ok(())
}
