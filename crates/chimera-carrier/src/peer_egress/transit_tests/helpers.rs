use chimera_session::{Frame, FrameKind};

pub(super) fn encoded_frame(kind: FrameKind, packet_number: u64, payload: &[u8]) -> Vec<u8> {
    match (Frame {
        kind,
        packet_number,
        payload: payload.to_vec(),
    })
    .encode()
    {
        Ok(encoded) => encoded,
        Err(error) => unreachable!("frame must encode: {error}"),
    }
}

pub(super) fn binding(
    route: u64,
    lane: u16,
) -> crate::peer_egress::transit_binding::TransitPathBinding {
    crate::peer_egress::transit_binding::TransitPathBinding::new(
        crate::peer_egress::transit_binding::TransitRouteId::new(route)
            .unwrap_or_else(|error| unreachable!("route id must be valid: {error}")),
        crate::peer_egress::transit_binding::TransitLaneId::new(lane)
            .unwrap_or_else(|error| unreachable!("lane id must be valid: {error}")),
    )
}

pub(super) fn tcp_pair() -> Result<(std::net::TcpStream, std::net::TcpStream), String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("bind test listener failed: {error}"))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("read test listener addr failed: {error}"))?;
    let client = std::net::TcpStream::connect(addr)
        .map_err(|error| format!("connect test client failed: {error}"))?;
    let (server, _) = listener
        .accept()
        .map_err(|error| format!("accept test server failed: {error}"))?;
    Ok((client, server))
}

pub(super) fn test_peer_pair() -> Result<
    (
        crate::peer_egress::protocol::SecurePeerStream,
        crate::peer_egress::protocol::SecurePeerStream,
    ),
    String,
> {
    let transcript = chimera_crypto::TranscriptHash::from_messages(&[b"peer-transit-test"]);
    let secrets = chimera_crypto::derive_traffic_secrets(
        chimera_crypto::SuiteId(
            crate::peer_egress::options::AeadSuite::Chacha20Poly1305.suite_id(),
        ),
        &transcript,
        &[11_u8; 32],
    )
    .map_err(|error| format!("derive test secrets failed: {error}"))?;
    let (left, right) = tcp_pair()?;
    Ok((
        crate::peer_egress::protocol::SecurePeerStream {
            stream: left,
            send_secret: secrets.initiator_to_responder().clone(),
            recv_secret: secrets.responder_to_initiator().clone(),
            send_packet: 0,
            recv_packet: 0,
            aead: crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
        },
        crate::peer_egress::protocol::SecurePeerStream {
            stream: right,
            send_secret: secrets.responder_to_initiator().clone(),
            recv_secret: secrets.initiator_to_responder().clone(),
            send_packet: 0,
            recv_packet: 0,
            aead: crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
        },
    ))
}
