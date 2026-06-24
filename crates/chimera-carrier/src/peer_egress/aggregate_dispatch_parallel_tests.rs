use super::{claim_aggregate_transit_shards, forward_claimed_aggregate_transit_shards};
use crate::peer_egress::aggregate_wire::AggregateObjectId;
use crate::peer_egress::options::AeadSuite;
use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::transit::validate_transit_relay_frame;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
use crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher;
use chimera_mesh::{MeshDiscoveryRecord, MeshJoinRequest, MeshRuntime};
use chimera_session::{Frame, FrameKind};
use std::net::Shutdown;

fn record(
    node_id: &str,
    endpoint: &str,
    region: &str,
    load: u8,
    reliability: u8,
) -> MeshDiscoveryRecord {
    MeshDiscoveryRecord {
        node_id: node_id.to_string(),
        endpoint: endpoint.to_string(),
        region: region.to_string(),
        load_score: load,
        reliability_score: reliability,
    }
}

fn plan() -> Result<chimera_mesh::MeshPathPlan, String> {
    let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
    runtime.merge_discovery(
        "seed-b",
        &[
            record("node-a", "198.51.100.31:443", "eu", 20, 90),
            record("node-b", "198.51.100.32:443", "eu", 22, 91),
            record("node-c", "198.51.100.33:443", "eu", 24, 92),
        ],
    )?;
    runtime.plan_path_from_dps_payload(
        &MeshJoinRequest {
            namespace: "cef-public".to_string(),
            node_name: "node-client".to_string(),
            invite_token: None,
        },
        "mesh_allowed_regions=eu;mesh_max_peers=3;mesh_max_selected_per_region=3;mesh_multipath_mode=aggregate_buffered;mesh_route_binding_id=7301",
    )
}

fn transit_frame(payload: &[u8]) -> Result<crate::peer_egress::transit::TransitRelayFrame, String> {
    let encoded = Frame {
        kind: FrameKind::Data,
        packet_number: 77,
        payload: payload.to_vec(),
    }
    .encode()
    .map_err(|error| format!("test frame encode failed: {error}"))?;
    validate_transit_relay_frame(&encoded)
}

fn aggregate_id() -> Result<AggregateObjectId, String> {
    AggregateObjectId::new(42)
}

fn binding(route: u64, lane: u16) -> Result<TransitPathBinding, String> {
    Ok(TransitPathBinding::new(
        TransitRouteId::new(route)?,
        TransitLaneId::new(lane)?,
    ))
}

fn test_peer_pair() -> Result<(SecurePeerStream, SecurePeerStream), String> {
    let transcript = chimera_crypto::TranscriptHash::from_messages(&[b"aggregate-dispatch-pair"]);
    let secrets = chimera_crypto::derive_traffic_secrets(
        chimera_crypto::SuiteId(AeadSuite::Chacha20Poly1305.suite_id()),
        &transcript,
        &[37_u8; 32],
    )
    .map_err(|error| format!("test secrets derive failed: {error}"))?;
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("bind test listener failed: {error}"))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("read listener addr failed: {error}"))?;
    let client = std::net::TcpStream::connect(addr)
        .map_err(|error| format!("connect test client failed: {error}"))?;
    let (server, _) = listener
        .accept()
        .map_err(|error| format!("accept test peer failed: {error}"))?;
    Ok((
        SecurePeerStream {
            stream: client,
            send_secret: secrets.initiator_to_responder().clone(),
            recv_secret: secrets.responder_to_initiator().clone(),
            send_packet: 0,
            recv_packet: 0,
            aead: AeadSuite::Chacha20Poly1305,
        },
        SecurePeerStream {
            stream: server,
            send_secret: secrets.responder_to_initiator().clone(),
            recv_secret: secrets.initiator_to_responder().clone(),
            send_packet: 0,
            recv_packet: 0,
            aead: AeadSuite::Chacha20Poly1305,
        },
    ))
}

#[test]
fn aggregate_dispatch_parallel_forward_fails_closed_on_lane_error() -> Result<(), String> {
    let plan = plan()?;
    let frame = transit_frame(b"SECRET_AGGREGATE_DISPATCH_PARALLEL_FAIL_PAYLOAD")?;
    let dispatcher = new_shared_transit_dispatcher();
    let first = binding(7301, 1)?;
    let second = binding(7301, 2)?;
    let third = binding(7301, 3)?;
    let (claim_first, _inspect_first) = test_peer_pair()?;
    let (claim_second, _inspect_second) = test_peer_pair()?;
    let (claim_third, _inspect_third) = test_peer_pair()?;
    dispatcher.register(first, claim_first)?;
    dispatcher.register(second, claim_second)?;
    dispatcher.register(third, claim_third)?;

    let claimed =
        claim_aggregate_transit_shards(&plan, &frame, aggregate_id()?, Some(dispatcher.clone()))?;
    claimed[1]
        .peer
        .stream
        .shutdown(Shutdown::Both)
        .map_err(|error| format!("shutdown claimed peer failed: {error}"))?;

    let error = match forward_claimed_aggregate_transit_shards(claimed) {
        Ok(()) => return Err("broken aggregate lane must fail closed".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("write secure frame failed"));
    assert!(!error.contains("SECRET_AGGREGATE_DISPATCH_PARALLEL_FAIL_PAYLOAD"));
    Ok(())
}
