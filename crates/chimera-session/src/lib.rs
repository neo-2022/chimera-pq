#![forbid(unsafe_code)]

mod frame;
mod handshake;
mod rekey;
mod replay;

pub use frame::{
    FRAME_VERSION, Frame, FrameKind, MAX_PAYLOAD_LEN, SealedTransitFrame, decrypt_frame_payload,
    encrypt_fin_frame_payload, encrypt_frame_payload, forward_sealed_transit_frame,
    validate_sealed_transit_frame,
};
pub use handshake::{
    EstablishedSession, HANDSHAKE_VERSION, HandshakeMessage, HybridHandshakeMessage, NONCE_LEN,
    TEST_ONLY_SUITE_ID, X25519_HKDF_SHA256_SUITE_ID, X25519_MLKEM768_HKDF_SHA256_SUITE_ID,
    client_finish_handshake, finish_handshake_with_x25519_shared_secret,
    finish_hybrid_handshake_with_shared_secrets, server_accept_client_hello,
    server_accept_client_hello_with_server_key_share, server_accept_hybrid_client_hello,
    server_finish_handshake,
};
pub use rekey::{RekeyPolicy, RekeyReason, RekeyState};
pub use replay::ReplayWindow;

#[cfg(test)]
mod session_tests;
