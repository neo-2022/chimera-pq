#![forbid(unsafe_code)]

use chimera_core::{ChimeraError, ChimeraResult};
use chimera_crypto::X25519Secret;
use chimera_session::{
    EstablishedSession, HandshakeMessage, NONCE_LEN, RekeyPolicy, RekeyReason, RekeyState,
    TEST_ONLY_SUITE_ID, X25519_HKDF_SHA256_SUITE_ID, client_finish_handshake,
    finish_handshake_with_x25519_shared_secret,
};

#[derive(Debug)]
pub struct ClientHandshake {
    client_hello: HandshakeMessage,
    x25519_secret: Option<X25519Secret>,
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

    pub fn on_packet_sent(&mut self) {
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

impl ClientHandshake {
    pub fn new_test_only(client_nonce: [u8; NONCE_LEN]) -> Self {
        Self {
            client_hello: HandshakeMessage::ClientHello {
                suite_id: TEST_ONLY_SUITE_ID,
                client_nonce,
                client_key_share: [0_u8; 32],
            },
            x25519_secret: None,
        }
    }

    pub fn new_x25519(client_nonce: [u8; NONCE_LEN], client_private_key: [u8; 32]) -> Self {
        let secret = X25519Secret::from_private_bytes(client_private_key);
        let client_key_share = secret.public_key_bytes();
        Self {
            client_hello: HandshakeMessage::ClientHello {
                suite_id: X25519_HKDF_SHA256_SUITE_ID,
                client_nonce,
                client_key_share,
            },
            x25519_secret: Some(secret),
        }
    }

    pub fn client_hello(&self) -> &HandshakeMessage {
        &self.client_hello
    }

    pub fn client_hello_bytes(&self) -> Vec<u8> {
        self.client_hello.encode()
    }

    pub fn finish_from_server_hello_bytes(
        &self,
        server_hello_bytes: &[u8],
    ) -> ChimeraResult<EstablishedSession> {
        let server_hello = HandshakeMessage::decode(server_hello_bytes)?;
        if let Some(secret) = &self.x25519_secret {
            let HandshakeMessage::ServerHello {
                suite_id,
                server_key_share,
                ..
            } = &server_hello
            else {
                return Err(ChimeraError::InvalidFrame(
                    "client expected server hello".to_string(),
                ));
            };
            if *suite_id == X25519_HKDF_SHA256_SUITE_ID {
                let shared_secret = secret.diffie_hellman(*server_key_share);
                return finish_handshake_with_x25519_shared_secret(
                    &self.client_hello,
                    &server_hello,
                    &shared_secret,
                );
            }
        }
        client_finish_handshake(&self.client_hello, &server_hello)
    }
}

pub fn reject_empty_server_hello(bytes: &[u8]) -> ChimeraResult<()> {
    if bytes.is_empty() {
        return Err(ChimeraError::InvalidFrame(
            "server hello is empty".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ClientHandshake, SessionRuntime, reject_empty_server_hello};
    use chimera_session::{NONCE_LEN, server_accept_client_hello_with_server_key_share};
    use chimera_session::{RekeyPolicy, RekeyReason};

    #[test]
    fn client_hello_bytes_are_not_empty() {
        let handshake = ClientHandshake::new_test_only([1_u8; 32]);
        assert!(!handshake.client_hello_bytes().is_empty());
    }

    #[test]
    fn empty_server_hello_is_rejected() {
        assert!(reject_empty_server_hello(&[]).is_err());
    }

    #[test]
    fn x25519_handshake_finishes_with_server_key_share() {
        let client = ClientHandshake::new_x25519([5_u8; NONCE_LEN], [7_u8; 32]);
        let server_secret = chimera_crypto::X25519Secret::from_private_bytes([9_u8; 32]);
        let server_hello = match server_accept_client_hello_with_server_key_share(
            client.client_hello(),
            [6_u8; NONCE_LEN],
            server_secret.public_key_bytes(),
        ) {
            Ok(server_hello) => server_hello,
            Err(error) => unreachable!("server hello should be created: {error}"),
        };

        let session = client.finish_from_server_hello_bytes(&server_hello.encode());
        assert!(session.is_ok());
    }

    #[test]
    fn session_runtime_reports_rekey_reason() {
        let client = ClientHandshake::new_test_only([1_u8; NONCE_LEN]);
        let server_hello = match server_accept_client_hello_with_server_key_share(
            client.client_hello(),
            [2_u8; NONCE_LEN],
            [0_u8; 32],
        ) {
            Ok(server_hello) => server_hello,
            Err(error) => unreachable!("server hello should be created: {error}"),
        };
        let established = match client.finish_from_server_hello_bytes(&server_hello.encode()) {
            Ok(established) => established,
            Err(error) => unreachable!("handshake should finish: {error}"),
        };
        let mut runtime = match SessionRuntime::new(
            established,
            RekeyPolicy {
                max_session_age_seconds: 60,
                max_packets_per_key: 2,
            },
            1_000,
        ) {
            Ok(runtime) => runtime,
            Err(error) => unreachable!("runtime should initialize: {error}"),
        };

        runtime.on_packet_sent();
        assert_eq!(runtime.status(1_010).rekey_reason, None);
        runtime.on_packet_sent();
        assert_eq!(
            runtime.status(1_010).rekey_reason,
            Some(RekeyReason::PacketLimitExceeded)
        );

        runtime.mark_rekey_complete(1_010);
        assert_eq!(runtime.status(1_010).rekey_reason, None);
    }
}
