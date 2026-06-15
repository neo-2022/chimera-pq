use crate::{
    Frame, FrameKind, HandshakeMessage, MAX_PAYLOAD_LEN, ReplayWindow, TEST_ONLY_SUITE_ID,
    client_finish_handshake, encrypt_fin_frame_payload, encrypt_frame_payload,
    forward_sealed_transit_frame, server_accept_client_hello, validate_sealed_transit_frame,
};

#[test]
fn frame_round_trip() {
    let frame = Frame {
        kind: FrameKind::Data,
        packet_number: 7,
        payload: b"hello".to_vec(),
    };

    let encoded = match frame.encode() {
        Ok(encoded) => encoded,
        Err(error) => unreachable!("frame should encode: {error}"),
    };
    let decoded = match Frame::decode(&encoded) {
        Ok(decoded) => decoded,
        Err(error) => unreachable!("frame should decode: {error}"),
    };

    assert_eq!(decoded, frame);
}

#[test]
fn malformed_frame_is_rejected() {
    let decoded = Frame::decode(&[1, 2, 3]);
    assert!(decoded.is_err());
}

#[test]
fn replayed_packet_is_rejected() {
    let mut window = ReplayWindow::default();
    assert!(window.accept(10).is_ok());
    assert!(window.accept(10).is_err());
    assert!(window.accept(9).is_err());
    assert!(window.accept(11).is_ok());
}

#[test]
fn sealed_transit_forwards_encoded_frame_without_payload_access() {
    let payload = b"third-party closed payload";
    let frame = encrypted_frame_payload(77, payload);
    let encoded = frame
        .encode()
        .unwrap_or_else(|error| unreachable!("frame should encode: {error}"));

    let transit = validate_sealed_transit_frame(&encoded)
        .unwrap_or_else(|error| unreachable!("transit frame should validate: {error}"));
    assert_eq!(transit.kind(), FrameKind::Data);
    assert_eq!(transit.packet_number(), 77);
    assert_eq!(transit.payload_len(), frame.payload.len());
    assert_eq!(transit.encoded(), encoded.as_slice());
    assert!(
        !transit
            .encoded()
            .windows(payload.len())
            .any(|w| w == payload)
    );
    let debug = format!("{transit:?}");
    assert!(debug.contains("<sealed>"));
    assert!(!debug.contains("third-party closed payload"));
    assert!(!debug.contains(&format!("{:?}", frame.payload)));

    let forwarded = forward_sealed_transit_frame(&encoded)
        .unwrap_or_else(|error| unreachable!("transit frame should forward: {error}"));
    assert_eq!(forwarded, encoded);
}

#[test]
fn sealed_transit_forwards_fin_frame_without_payload_access() {
    let payload = b"third-party stream finished";
    let frame = encrypted_fin_frame_payload(78, payload);
    let encoded = frame
        .encode()
        .unwrap_or_else(|error| unreachable!("FIN frame should encode: {error}"));

    let transit = validate_sealed_transit_frame(&encoded)
        .unwrap_or_else(|error| unreachable!("FIN transit frame should validate: {error}"));
    assert_eq!(transit.kind(), FrameKind::Fin);
    assert_eq!(transit.packet_number(), 78);
    assert!(
        !transit
            .encoded()
            .windows(payload.len())
            .any(|w| w == payload)
    );
    let debug = format!("{transit:?}");
    assert!(debug.contains("<sealed>"));
    assert!(!debug.contains("third-party stream finished"));

    let forwarded = forward_sealed_transit_frame(&encoded)
        .unwrap_or_else(|error| unreachable!("FIN transit frame should forward: {error}"));
    assert_eq!(forwarded, encoded);
}

#[test]
fn sealed_transit_rejects_malformed_envelope_without_decrypting() {
    let oversized = Frame {
        kind: FrameKind::Data,
        packet_number: 1,
        payload: vec![0_u8; MAX_PAYLOAD_LEN + 1],
    }
    .encode();
    assert!(oversized.is_err());

    let mut malformed = vec![1_u8; 14];
    malformed[1] = FrameKind::Data.encode();
    malformed[10..14].copy_from_slice(&(MAX_PAYLOAD_LEN as u32 + 1).to_be_bytes());
    assert!(validate_sealed_transit_frame(&malformed).is_err());

    let encoded = Frame {
        kind: FrameKind::Data,
        packet_number: 2,
        payload: vec![1, 2, 3],
    }
    .encode();
    let mut truncated =
        encoded.unwrap_or_else(|error| unreachable!("frame should encode: {error}"));
    truncated.pop();
    assert!(forward_sealed_transit_frame(&truncated).is_err());
}

fn encrypted_frame_payload(packet_number: u64, payload: &[u8]) -> Frame {
    let client_hello = HandshakeMessage::ClientHello {
        suite_id: TEST_ONLY_SUITE_ID,
        client_nonce: [21_u8; 32],
        client_key_share: [0_u8; 32],
    };
    let server_hello = server_accept_client_hello(&client_hello, [22_u8; 32])
        .unwrap_or_else(|error| unreachable!("server should accept client hello: {error}"));
    let client_session = client_finish_handshake(&client_hello, &server_hello)
        .unwrap_or_else(|error| unreachable!("client should finish handshake: {error}"));
    encrypt_frame_payload(
        packet_number,
        payload,
        client_session.traffic_secrets.initiator_to_responder(),
    )
    .unwrap_or_else(|error| unreachable!("frame should encrypt: {error}"))
}

fn encrypted_fin_frame_payload(packet_number: u64, payload: &[u8]) -> Frame {
    let client_hello = HandshakeMessage::ClientHello {
        suite_id: TEST_ONLY_SUITE_ID,
        client_nonce: [41_u8; 32],
        client_key_share: [0_u8; 32],
    };
    let server_hello = server_accept_client_hello(&client_hello, [42_u8; 32])
        .unwrap_or_else(|error| unreachable!("server should accept client hello: {error}"));
    let client_session = client_finish_handshake(&client_hello, &server_hello)
        .unwrap_or_else(|error| unreachable!("client should finish handshake: {error}"));
    encrypt_fin_frame_payload(
        packet_number,
        payload,
        client_session.traffic_secrets.initiator_to_responder(),
    )
    .unwrap_or_else(|error| unreachable!("FIN frame should encrypt: {error}"))
}
