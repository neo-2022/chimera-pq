#![forbid(unsafe_code)]

use chimera_core::{ChimeraError, ChimeraResult};
use chimera_crypto::{X25519Secret, X25519SharedSecret};
use chimera_session::{
    EstablishedSession, HandshakeMessage, NONCE_LEN, RekeyPolicy, RekeyReason, RekeyState,
    X25519_HKDF_SHA256_SUITE_ID, finish_handshake_with_x25519_shared_secret,
    server_accept_client_hello, server_accept_client_hello_with_server_key_share,
    server_finish_handshake,
};

#[derive(Debug, Clone)]
pub struct GatewayHandshake {
    client_hello: HandshakeMessage,
    server_hello: HandshakeMessage,
    x25519_shared_secret: Option<X25519SharedSecret>,
}

#[derive(Debug, Clone)]
pub struct SessionRuntime {
    session: EstablishedSession,
    rekey_state: RekeyState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStatus {
    pub suite_id: u16,
    pub rekey_required: bool,
    pub rekey_reason: Option<RekeyReason>,
}

impl SessionRuntime {
    pub fn new(
        session: EstablishedSession,
        rekey_policy: RekeyPolicy,
        established_at_seconds: u64,
    ) -> ChimeraResult<Self> {
        Ok(Self {
            session,
            rekey_state: RekeyState::new(rekey_policy, established_at_seconds)?,
        })
    }

    pub fn on_packet_forwarded(&mut self) {
        self.rekey_state.on_packet_sent();
    }

    pub fn status(&self, now_seconds: u64) -> SessionStatus {
        SessionStatus {
            suite_id: self.session.suite_id,
            rekey_required: self.rekey_state.should_rekey(now_seconds),
            rekey_reason: self.rekey_state.rekey_reason(now_seconds),
        }
    }

    pub fn mark_rekey_complete(&mut self, now_seconds: u64) {
        self.rekey_state.reset_after_rekey(now_seconds);
    }
}

impl GatewayHandshake {
    pub fn accept_client_hello_bytes(
        client_hello_bytes: &[u8],
        server_nonce: [u8; NONCE_LEN],
    ) -> ChimeraResult<Self> {
        let client_hello = HandshakeMessage::decode(client_hello_bytes)?;
        let server_hello = server_accept_client_hello(&client_hello, server_nonce)?;
        Ok(Self {
            client_hello,
            server_hello,
            x25519_shared_secret: None,
        })
    }

    pub fn accept_x25519_client_hello_bytes(
        client_hello_bytes: &[u8],
        server_nonce: [u8; NONCE_LEN],
        server_private_key: [u8; 32],
    ) -> ChimeraResult<Self> {
        let client_hello = HandshakeMessage::decode(client_hello_bytes)?;
        let (suite_id, client_key_share) = match &client_hello {
            HandshakeMessage::ClientHello {
                suite_id,
                client_key_share,
                ..
            } => (*suite_id, *client_key_share),
            HandshakeMessage::ServerHello { .. } => {
                return Err(ChimeraError::InvalidFrame(
                    "server expected client hello".to_string(),
                ));
            }
        };

        if suite_id != X25519_HKDF_SHA256_SUITE_ID {
            return Err(ChimeraError::Unsupported(
                "X25519 handshake requires X25519 suite id".to_string(),
            ));
        }

        let server_secret = X25519Secret::from_private_bytes(server_private_key);
        let server_key_share = server_secret.public_key_bytes();
        let server_hello = server_accept_client_hello_with_server_key_share(
            &client_hello,
            server_nonce,
            server_key_share,
        )?;
        let shared_secret = server_secret.diffie_hellman(client_key_share);
        Ok(Self {
            client_hello,
            server_hello,
            x25519_shared_secret: Some(shared_secret),
        })
    }

    pub fn server_hello_bytes(&self) -> Vec<u8> {
        self.server_hello.encode()
    }

    pub fn finish(&self) -> ChimeraResult<EstablishedSession> {
        if let Some(shared_secret) = &self.x25519_shared_secret {
            return finish_handshake_with_x25519_shared_secret(
                &self.client_hello,
                &self.server_hello,
                shared_secret,
            );
        }
        server_finish_handshake(&self.client_hello, &self.server_hello)
    }
}

#[cfg(test)]
mod tests {
    use super::{GatewayHandshake, SessionRuntime};
    use chimera_session::{
        HandshakeMessage, NONCE_LEN, RekeyPolicy, RekeyReason, TEST_ONLY_SUITE_ID,
        X25519_HKDF_SHA256_SUITE_ID,
    };

    #[test]
    fn gateway_accepts_test_only_client_hello() {
        let client_hello = HandshakeMessage::ClientHello {
            suite_id: TEST_ONLY_SUITE_ID,
            client_nonce: [1_u8; 32],
            client_key_share: [0_u8; 32],
        };
        let gateway =
            GatewayHandshake::accept_client_hello_bytes(&client_hello.encode(), [2_u8; 32]);
        assert!(gateway.is_ok());
    }

    #[test]
    fn gateway_rejects_malformed_client_hello() {
        let gateway = GatewayHandshake::accept_client_hello_bytes(&[1, 2, 3], [2_u8; 32]);
        assert!(gateway.is_err());
    }

    #[test]
    fn gateway_accepts_x25519_client_hello() {
        let client_secret = chimera_crypto::X25519Secret::from_private_bytes([11_u8; 32]);
        let client_hello = HandshakeMessage::ClientHello {
            suite_id: X25519_HKDF_SHA256_SUITE_ID,
            client_nonce: [3_u8; NONCE_LEN],
            client_key_share: client_secret.public_key_bytes(),
        };
        let gateway = GatewayHandshake::accept_x25519_client_hello_bytes(
            &client_hello.encode(),
            [4_u8; NONCE_LEN],
            [12_u8; 32],
        );
        assert!(gateway.is_ok());
    }

    #[test]
    fn gateway_session_runtime_reports_age_rekey_reason() {
        let client_hello = HandshakeMessage::ClientHello {
            suite_id: TEST_ONLY_SUITE_ID,
            client_nonce: [1_u8; 32],
            client_key_share: [0_u8; 32],
        };
        let gateway_handshake =
            match GatewayHandshake::accept_client_hello_bytes(&client_hello.encode(), [2_u8; 32]) {
                Ok(gateway) => gateway,
                Err(error) => unreachable!("gateway should accept client hello: {error}"),
            };
        let established = match gateway_handshake.finish() {
            Ok(established) => established,
            Err(error) => unreachable!("gateway handshake should finish: {error}"),
        };
        let runtime = match SessionRuntime::new(
            established,
            RekeyPolicy {
                max_session_age_seconds: 5,
                max_packets_per_key: 100,
            },
            10,
        ) {
            Ok(runtime) => runtime,
            Err(error) => unreachable!("runtime should initialize: {error}"),
        };

        assert_eq!(
            runtime.status(15).rekey_reason,
            Some(RekeyReason::SessionAgeExceeded)
        );
    }
}
