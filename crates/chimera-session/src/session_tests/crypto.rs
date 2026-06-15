use crate::{
    FrameKind, HandshakeMessage, TEST_ONLY_SUITE_ID, client_finish_handshake,
    decrypt_frame_payload, encrypt_fin_frame_payload, encrypt_frame_payload,
    server_accept_client_hello, server_finish_handshake,
};

#[test]
fn encrypted_frame_round_trips_after_handshake() {
    let client_hello = HandshakeMessage::ClientHello {
        suite_id: TEST_ONLY_SUITE_ID,
        client_nonce: [1_u8; 32],
        client_key_share: [0_u8; 32],
    };
    let server_hello = server_accept_client_hello(&client_hello, [2_u8; 32])
        .unwrap_or_else(|error| unreachable!("server should accept client hello: {error}"));
    let client_session = client_finish_handshake(&client_hello, &server_hello)
        .unwrap_or_else(|error| unreachable!("client should finish handshake: {error}"));
    let server_session = server_finish_handshake(&client_hello, &server_hello)
        .unwrap_or_else(|error| unreachable!("server should finish handshake: {error}"));

    let encrypted = encrypt_frame_payload(
        42,
        b"application payload",
        client_session.traffic_secrets.initiator_to_responder(),
    )
    .unwrap_or_else(|error| unreachable!("frame should encrypt: {error}"));
    assert_ne!(encrypted.payload, b"application payload");

    let decrypted = decrypt_frame_payload(
        &encrypted,
        server_session.traffic_secrets.initiator_to_responder(),
    )
    .unwrap_or_else(|error| unreachable!("frame should decrypt: {error}"));
    assert_eq!(decrypted, b"application payload");
}

#[test]
fn fin_frame_round_trips_after_handshake() {
    let client_hello = HandshakeMessage::ClientHello {
        suite_id: TEST_ONLY_SUITE_ID,
        client_nonce: [31_u8; 32],
        client_key_share: [0_u8; 32],
    };
    let server_hello = server_accept_client_hello(&client_hello, [32_u8; 32])
        .unwrap_or_else(|error| unreachable!("server should accept client hello: {error}"));
    let client_session = client_finish_handshake(&client_hello, &server_hello)
        .unwrap_or_else(|error| unreachable!("client should finish handshake: {error}"));
    let server_session = server_finish_handshake(&client_hello, &server_hello)
        .unwrap_or_else(|error| unreachable!("server should finish handshake: {error}"));

    let encrypted = encrypt_fin_frame_payload(
        43,
        b"stream finished",
        client_session.traffic_secrets.initiator_to_responder(),
    )
    .unwrap_or_else(|error| unreachable!("FIN frame should encrypt: {error}"));
    assert_eq!(encrypted.kind, FrameKind::Fin);

    let decrypted = decrypt_frame_payload(
        &encrypted,
        server_session.traffic_secrets.initiator_to_responder(),
    )
    .unwrap_or_else(|error| unreachable!("FIN frame should decrypt: {error}"));
    assert_eq!(decrypted, b"stream finished");
}

#[test]
fn encrypted_frame_rejects_wrong_packet_number_aad() {
    let client_hello = HandshakeMessage::ClientHello {
        suite_id: TEST_ONLY_SUITE_ID,
        client_nonce: [3_u8; 32],
        client_key_share: [0_u8; 32],
    };
    let server_hello = server_accept_client_hello(&client_hello, [4_u8; 32])
        .unwrap_or_else(|error| unreachable!("server should accept client hello: {error}"));
    let session = client_finish_handshake(&client_hello, &server_hello)
        .unwrap_or_else(|error| unreachable!("client should finish handshake: {error}"));
    let mut encrypted = encrypt_frame_payload(
        7,
        b"payload",
        session.traffic_secrets.initiator_to_responder(),
    )
    .unwrap_or_else(|error| unreachable!("frame should encrypt: {error}"));
    encrypted.packet_number = 8;

    let decrypted =
        decrypt_frame_payload(&encrypted, session.traffic_secrets.initiator_to_responder());
    assert!(decrypted.is_err());
}
