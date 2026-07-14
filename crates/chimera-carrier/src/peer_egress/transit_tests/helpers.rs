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

pub(super) fn bound_payload(
    binding: crate::peer_egress::transit_binding::TransitPathBinding,
    kind: FrameKind,
    packet_number: u64,
    payload: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let sealed = encoded_frame(kind, packet_number, payload);
    let frame = crate::peer_egress::transit::validate_transit_relay_frame(&sealed)?;
    let bound = crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(binding, frame);
    Ok((
        crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&bound),
        sealed,
    ))
}

pub(super) fn assert_bound_payload(
    payload: &[u8],
    binding: crate::peer_egress::transit_binding::TransitPathBinding,
    expected_sealed: &[u8],
) -> Result<(), String> {
    if !payload.starts_with(&[crate::peer_egress::transit_binding::BOUND_TRANSIT_MAGIC]) {
        return Err("forwarded payload must preserve bound transit magic".to_string());
    }
    let parsed = crate::peer_egress::transit_binding::validate_bound_transit_relay_frame(payload)?;
    if parsed.binding() != binding {
        return Err("forwarded payload binding mismatch".to_string());
    }
    if parsed.frame().sealed_bytes() != expected_sealed {
        return Err("forwarded sealed bytes mismatch".to_string());
    }
    Ok(())
}

pub(super) fn assert_bytes_eq_redacted(
    actual: &[u8],
    expected: &[u8],
    context: &str,
) -> Result<(), String> {
    if actual != expected {
        return Err(format!(
            "{context}: byte mismatch actual_len={} expected_len={}",
            actual.len(),
            expected.len()
        ));
    }
    Ok(())
}

pub(super) fn read_first_bound_frame(
    source_reader: &mut crate::peer_egress::protocol::SecurePeerStream,
) -> Result<crate::peer_egress::transit_binding::BoundTransitRelayFrame, String> {
    match crate::peer_egress::wire::read_peer_message(
        source_reader,
        crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
    )? {
        crate::peer_egress::wire::PeerMessage::BoundSealedTransit(frame) => Ok(frame),
        other => Err(format!("unexpected first bound message: {other:?}")),
    }
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
        crate::peer_egress::protocol::SecurePeerStream::new(
            left,
            secrets.initiator_to_responder().clone(),
            secrets.responder_to_initiator().clone(),
            crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
        ),
        crate::peer_egress::protocol::SecurePeerStream::new(
            right,
            secrets.responder_to_initiator().clone(),
            secrets.initiator_to_responder().clone(),
            crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
        ),
    ))
}
