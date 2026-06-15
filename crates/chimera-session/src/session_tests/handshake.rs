use crate::{
    HandshakeMessage, HybridHandshakeMessage, TEST_ONLY_SUITE_ID,
    X25519_MLKEM768_HKDF_SHA256_SUITE_ID, client_finish_handshake,
    finish_hybrid_handshake_with_shared_secrets, server_accept_client_hello,
    server_accept_hybrid_client_hello, server_finish_handshake,
};
use chimera_crypto::{X25519Secret, ml_kem_768_decapsulate, ml_kem_768_generate_keypair};

#[test]
fn handshake_messages_round_trip() {
    let message = HandshakeMessage::ClientHello {
        suite_id: TEST_ONLY_SUITE_ID,
        client_nonce: [7_u8; 32],
        client_key_share: [0_u8; 32],
    };

    let decoded = match HandshakeMessage::decode(&message.encode()) {
        Ok(decoded) => decoded,
        Err(error) => unreachable!("handshake should decode: {error}"),
    };

    assert_eq!(decoded, message);
}

#[test]
fn hybrid_handshake_messages_round_trip() {
    let message = HybridHandshakeMessage::ClientHello {
        suite_id: X25519_MLKEM768_HKDF_SHA256_SUITE_ID,
        client_nonce: [7_u8; 32],
        client_x25519_key_share: [8_u8; 32],
        client_ml_kem_768_encapsulation_key: vec![9_u8; 1184],
    };
    let encoded = message
        .encode()
        .unwrap_or_else(|error| unreachable!("hybrid handshake should encode: {error}"));
    let decoded = HybridHandshakeMessage::decode(&encoded)
        .unwrap_or_else(|error| unreachable!("hybrid handshake should decode: {error}"));
    assert_eq!(decoded, message);
}

#[test]
fn client_and_server_derive_same_traffic_secret() {
    let client_hello = HandshakeMessage::ClientHello {
        suite_id: TEST_ONLY_SUITE_ID,
        client_nonce: [1_u8; 32],
        client_key_share: [0_u8; 32],
    };
    let server_hello = match server_accept_client_hello(&client_hello, [2_u8; 32]) {
        Ok(message) => message,
        Err(error) => unreachable!("server should accept client hello: {error}"),
    };

    let client_session = match client_finish_handshake(&client_hello, &server_hello) {
        Ok(session) => session,
        Err(error) => unreachable!("client should finish handshake: {error}"),
    };
    let server_session = match server_finish_handshake(&client_hello, &server_hello) {
        Ok(session) => session,
        Err(error) => unreachable!("server should finish handshake: {error}"),
    };

    assert_eq!(
        client_session
            .traffic_secrets
            .initiator_to_responder()
            .expose_for_tests(),
        server_session
            .traffic_secrets
            .initiator_to_responder()
            .expose_for_tests()
    );
}

#[test]
fn hybrid_x25519_mlkem768_handshake_derives_same_traffic_secret() {
    let client_x25519 = X25519Secret::from_private_bytes([11_u8; 32]);
    let server_x25519 = X25519Secret::from_private_bytes([12_u8; 32]);
    let (client_ml_kem_decapsulation_key, client_ml_kem_encapsulation_key) =
        ml_kem_768_generate_keypair();
    let client_hello = HybridHandshakeMessage::ClientHello {
        suite_id: X25519_MLKEM768_HKDF_SHA256_SUITE_ID,
        client_nonce: [13_u8; 32],
        client_x25519_key_share: client_x25519.public_key_bytes(),
        client_ml_kem_768_encapsulation_key: client_ml_kem_encapsulation_key,
    };
    let (server_hello, server_pq_secret) = server_accept_hybrid_client_hello(
        &client_hello,
        [14_u8; 32],
        server_x25519.public_key_bytes(),
    )
    .unwrap_or_else(|error| unreachable!("server should accept hybrid hello: {error}"));

    let HybridHandshakeMessage::ServerHello {
        ml_kem_768_ciphertext,
        ..
    } = &server_hello
    else {
        unreachable!("server produced server hello");
    };
    let client_pq_secret =
        ml_kem_768_decapsulate(&client_ml_kem_decapsulation_key, ml_kem_768_ciphertext)
            .unwrap_or_else(|error| unreachable!("client should decapsulate: {error}"));
    let client_x25519_shared = client_x25519.diffie_hellman(server_x25519.public_key_bytes());
    let server_x25519_shared = server_x25519.diffie_hellman(client_x25519.public_key_bytes());

    let client_session = finish_hybrid_handshake_with_shared_secrets(
        &client_hello,
        &server_hello,
        &client_x25519_shared,
        &client_pq_secret,
    )
    .unwrap_or_else(|error| unreachable!("client hybrid finish should work: {error}"));
    let server_session = finish_hybrid_handshake_with_shared_secrets(
        &client_hello,
        &server_hello,
        &server_x25519_shared,
        &server_pq_secret,
    )
    .unwrap_or_else(|error| unreachable!("server hybrid finish should work: {error}"));

    assert_eq!(
        client_session
            .traffic_secrets
            .initiator_to_responder()
            .expose_for_tests(),
        server_session
            .traffic_secrets
            .initiator_to_responder()
            .expose_for_tests()
    );
    assert_eq!(
        client_session
            .traffic_secrets
            .responder_to_initiator()
            .expose_for_tests(),
        server_session
            .traffic_secrets
            .responder_to_initiator()
            .expose_for_tests()
    );
}

#[test]
fn suite_downgrade_is_rejected() {
    let client_hello = HandshakeMessage::ClientHello {
        suite_id: TEST_ONLY_SUITE_ID,
        client_nonce: [3_u8; 32],
        client_key_share: [0_u8; 32],
    };
    let server_hello = HandshakeMessage::ServerHello {
        suite_id: 0x9999,
        client_nonce: [3_u8; 32],
        server_nonce: [4_u8; 32],
        server_key_share: [0_u8; 32],
    };

    assert!(client_finish_handshake(&client_hello, &server_hello).is_err());
}

#[test]
fn transcript_change_changes_traffic_secret() {
    let first_client_hello = HandshakeMessage::ClientHello {
        suite_id: TEST_ONLY_SUITE_ID,
        client_nonce: [5_u8; 32],
        client_key_share: [0_u8; 32],
    };
    let second_client_hello = HandshakeMessage::ClientHello {
        suite_id: TEST_ONLY_SUITE_ID,
        client_nonce: [6_u8; 32],
        client_key_share: [0_u8; 32],
    };
    let first_server_hello = match server_accept_client_hello(&first_client_hello, [9_u8; 32]) {
        Ok(message) => message,
        Err(error) => unreachable!("server should accept first hello: {error}"),
    };
    let second_server_hello = match server_accept_client_hello(&second_client_hello, [9_u8; 32]) {
        Ok(message) => message,
        Err(error) => unreachable!("server should accept second hello: {error}"),
    };

    let first = match client_finish_handshake(&first_client_hello, &first_server_hello) {
        Ok(session) => session,
        Err(error) => unreachable!("first handshake should finish: {error}"),
    };
    let second = match client_finish_handshake(&second_client_hello, &second_server_hello) {
        Ok(session) => session,
        Err(error) => unreachable!("second handshake should finish: {error}"),
    };

    assert_ne!(
        first
            .traffic_secrets
            .initiator_to_responder()
            .expose_for_tests(),
        second
            .traffic_secrets
            .initiator_to_responder()
            .expose_for_tests()
    );
}
