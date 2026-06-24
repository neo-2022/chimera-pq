use std::io::Write;
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use chimera_session::{Frame, FrameKind};

use crate::peer_egress::net::tune_tcp;
use crate::peer_egress::options::Options;
use crate::peer_egress::transit::validate_transit_relay_frame;
use crate::peer_egress::transit_binding::{
    BoundTransitRelayFrame, TransitLaneId, TransitPathBinding, TransitRouteId,
    encode_bound_transit_relay_frame,
};

const MAX_PROOF_PAYLOAD_BYTES: usize = 16 * 1024;
const PROOF_BYTE: u8 = b'P';

pub fn run_sealed_transit_inject(options: Options) -> Result<(), String> {
    let sealed = build_sealed_transit_payload(&options)?;
    write_payload_to_local_ingress(&options.server, &sealed, options.connect_timeout_ms)?;
    println!(
        "chimera_peer_egress_sealed_transit_inject=ok bytes={}",
        sealed.len()
    );
    Ok(())
}

pub fn run_bound_transit_inject(options: Options) -> Result<(), String> {
    let bound = build_bound_transit_payload(&options)?;
    write_payload_to_local_ingress(&options.server, &bound, options.connect_timeout_ms)?;
    println!(
        "chimera_peer_egress_bound_transit_inject=ok bytes={}",
        bound.len()
    );
    Ok(())
}

fn build_sealed_transit_payload(options: &Options) -> Result<Vec<u8>, String> {
    if options.transit_payload_bytes > MAX_PROOF_PAYLOAD_BYTES {
        return Err(format!(
            "transit-payload-bytes must be <= {MAX_PROOF_PAYLOAD_BYTES}"
        ));
    }
    let frame = Frame {
        kind: FrameKind::Data,
        packet_number: options.transit_packet_number,
        payload: vec![PROOF_BYTE; options.transit_payload_bytes],
    };
    frame
        .encode()
        .map_err(|error| format!("encode sealed transit proof frame failed: {error}"))
}

fn build_bound_transit_payload(options: &Options) -> Result<Vec<u8>, String> {
    let route_id = options
        .transit_route_id
        .ok_or_else(|| "bound transit inject route id missing".to_string())?;
    let lane_index = options
        .transit_lane_index
        .ok_or_else(|| "bound transit inject lane index missing".to_string())?;
    let sealed = build_sealed_transit_payload(options)?;
    let frame = validate_transit_relay_frame(&sealed)?;
    let binding = TransitPathBinding::new(
        TransitRouteId::new(route_id)?,
        TransitLaneId::from_zero_based_lane_index(lane_index)?,
    );
    let bound = BoundTransitRelayFrame::new(binding, frame);
    Ok(encode_bound_transit_relay_frame(&bound))
}

fn write_payload_to_local_ingress(
    server: &str,
    payload: &[u8],
    connect_timeout_ms: u64,
) -> Result<(), String> {
    let addr = server.trim();
    if addr.is_empty() {
        return Err("transit inject server is empty".to_string());
    }
    let timeout = Duration::from_millis(connect_timeout_ms);
    let socket_addr = resolve_single_socket_addr(addr)?;
    let mut stream = TcpStream::connect_timeout(&socket_addr, timeout)
        .map_err(|error| format!("connect transit proof local ingress failed: {error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("set transit proof read timeout failed: {error}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| format!("set transit proof write timeout failed: {error}"))?;
    tune_tcp(&stream)?;
    stream
        .write_all(payload)
        .map_err(|error| format!("write transit proof payload failed: {error}"))?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("shutdown transit proof writer failed: {error}"))?;
    Ok(())
}

fn resolve_single_socket_addr(addr: &str) -> Result<SocketAddr, String> {
    let mut addrs = addr
        .to_socket_addrs()
        .map_err(|error| format!("resolve transit proof local ingress failed: {error}"))?;
    addrs
        .next()
        .ok_or_else(|| "resolve transit proof local ingress returned no address".to_string())
}

#[cfg(test)]
mod tests {
    use super::{build_bound_transit_payload, build_sealed_transit_payload};
    use crate::peer_egress::options::{AeadSuite, Mode, Options};

    fn proof_options() -> Options {
        Options {
            mode: Mode::SealedTransitInject,
            local_listen: String::new(),
            peer_listen: String::new(),
            state_file: None,
            server: "127.0.0.1:1".to_string(),
            token: String::new(),
            pool: 1,
            bench_bytes: 1024,
            target: String::new(),
            connect_timeout_ms: 100,
            min_throughput_mib_s: 0,
            connections: 1,
            aead: AeadSuite::Chacha20Poly1305,
            reverse_connect: false,
            allow_pool_transit: false,
            allow_bound_transit: false,
            transit_lane_bindings_file: None,
            transit_max_frames_per_direction:
                crate::peer_egress::transit_guard::DEFAULT_TRANSIT_MAX_FRAMES_PER_DIRECTION,
            transit_max_bytes_per_direction:
                crate::peer_egress::transit_guard::DEFAULT_TRANSIT_MAX_BYTES_PER_DIRECTION,
            transit_idle_timeout_ms:
                crate::peer_egress::transit_guard::DEFAULT_TRANSIT_IDLE_TIMEOUT_MS,
            transit_payload_bytes: 32,
            transit_packet_number: 9,
            transit_route_id: Some(77),
            transit_lane_index: Some(0),
        }
    }

    #[test]
    fn sealed_transit_proof_payload_is_valid_sealed_frame() -> Result<(), String> {
        let options = proof_options();
        let sealed = build_sealed_transit_payload(&options)?;
        let parsed = crate::peer_egress::transit::validate_transit_relay_frame(&sealed)?;

        assert_eq!(parsed.packet_number(), 9);
        assert_eq!(parsed.payload_len(), 32);
        assert_eq!(parsed.sealed_bytes(), sealed.as_slice());
        Ok(())
    }

    #[test]
    fn bound_transit_proof_payload_preserves_opaque_binding_and_sealed_frame() -> Result<(), String>
    {
        let options = proof_options();
        let bound = build_bound_transit_payload(&options)?;
        let parsed =
            crate::peer_egress::transit_binding::validate_bound_transit_relay_frame(&bound)?;
        let debug = format!("{parsed:?}");

        assert_eq!(parsed.binding().route_id().get(), 77);
        assert_eq!(parsed.binding().lane_id().get(), 1);
        assert_eq!(parsed.frame().payload_len(), 32);
        assert!(debug.contains("<opaque>"));
        assert!(debug.contains("<sealed>"));
        assert!(!debug.contains("77"));
        Ok(())
    }

    #[test]
    fn proof_payload_rejects_oversized_payload() {
        let mut options = proof_options();
        options.transit_payload_bytes = 16 * 1024 + 1;

        let error = match build_sealed_transit_payload(&options) {
            Ok(_) => "unexpected proof success".to_string(),
            Err(error) => error,
        };

        assert!(error.contains("transit-payload-bytes"));
    }
}
