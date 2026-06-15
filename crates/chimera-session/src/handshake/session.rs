use chimera_core::{ChimeraError, ChimeraResult};
use chimera_crypto::{
    SuiteId, TrafficSecrets, TranscriptHash, X25519_PUBLIC_KEY_LEN, X25519SharedSecret,
    derive_hybrid_traffic_secrets, derive_traffic_secrets, ml_kem_768_encapsulate,
};

use super::message::{HandshakeMessage, HybridHandshakeMessage, NONCE_LEN};

pub const TEST_ONLY_SUITE_ID: u16 = 0x0001;
pub const X25519_HKDF_SHA256_SUITE_ID: u16 = 0x0101;
pub const X25519_MLKEM768_HKDF_SHA256_SUITE_ID: u16 = 0x0201;

#[derive(Debug, Clone)]
pub struct EstablishedSession {
    pub suite_id: u16,
    pub transcript_hash: TranscriptHash,
    pub traffic_secrets: TrafficSecrets,
}

pub fn server_accept_client_hello(
    message: &HandshakeMessage,
    server_nonce: [u8; NONCE_LEN],
) -> ChimeraResult<HandshakeMessage> {
    server_accept_client_hello_with_server_key_share(
        message,
        server_nonce,
        [0_u8; X25519_PUBLIC_KEY_LEN],
    )
}

pub fn server_accept_client_hello_with_server_key_share(
    message: &HandshakeMessage,
    server_nonce: [u8; NONCE_LEN],
    server_key_share: [u8; X25519_PUBLIC_KEY_LEN],
) -> ChimeraResult<HandshakeMessage> {
    match message {
        HandshakeMessage::ClientHello {
            suite_id,
            client_nonce,
            ..
        } if *suite_id == TEST_ONLY_SUITE_ID => Ok(HandshakeMessage::ServerHello {
            suite_id: *suite_id,
            client_nonce: *client_nonce,
            server_nonce,
            server_key_share: [0_u8; X25519_PUBLIC_KEY_LEN],
        }),
        HandshakeMessage::ClientHello {
            suite_id,
            client_nonce,
            client_key_share,
        } if *suite_id == X25519_HKDF_SHA256_SUITE_ID
            && *client_key_share != [0_u8; X25519_PUBLIC_KEY_LEN]
            && server_key_share != [0_u8; X25519_PUBLIC_KEY_LEN] =>
        {
            Ok(HandshakeMessage::ServerHello {
                suite_id: *suite_id,
                client_nonce: *client_nonce,
                server_nonce,
                server_key_share,
            })
        }
        HandshakeMessage::ClientHello { suite_id, .. }
            if *suite_id == X25519_HKDF_SHA256_SUITE_ID =>
        {
            Err(ChimeraError::InvalidFrame(
                "X25519 key share must be non-zero".to_string(),
            ))
        }
        HandshakeMessage::ClientHello { suite_id, .. } => Err(ChimeraError::Unsupported(format!(
            "unsupported suite id {suite_id}"
        ))),
        HandshakeMessage::ServerHello { .. } => Err(ChimeraError::InvalidFrame(
            "server expected client hello".to_string(),
        )),
    }
}

pub fn client_finish_handshake(
    client_hello: &HandshakeMessage,
    server_hello: &HandshakeMessage,
) -> ChimeraResult<EstablishedSession> {
    let HandshakeMessage::ClientHello {
        suite_id,
        client_nonce,
        ..
    } = client_hello
    else {
        return Err(ChimeraError::InvalidFrame(
            "client transcript must start with client hello".to_string(),
        ));
    };

    let HandshakeMessage::ServerHello {
        suite_id: server_suite_id,
        client_nonce: echoed_client_nonce,
        server_nonce,
        ..
    } = server_hello
    else {
        return Err(ChimeraError::InvalidFrame(
            "client expected server hello".to_string(),
        ));
    };

    if *suite_id != *server_suite_id {
        return Err(ChimeraError::Unsupported(
            "suite downgrade or mismatch rejected".to_string(),
        ));
    }

    if *suite_id != TEST_ONLY_SUITE_ID && *suite_id != X25519_HKDF_SHA256_SUITE_ID {
        return Err(ChimeraError::Unsupported(format!(
            "unsupported suite id {suite_id}"
        )));
    }

    if client_nonce != echoed_client_nonce {
        return Err(ChimeraError::InvalidFrame(
            "server hello echoed wrong client nonce".to_string(),
        ));
    }

    if *suite_id == TEST_ONLY_SUITE_ID {
        let input_key_material =
            derive_test_only_input_key_material(*suite_id, client_nonce, server_nonce);
        return finish_handshake_with_input_key_material(
            *suite_id,
            client_hello,
            server_hello,
            &input_key_material.expose_for_tests(),
        );
    }

    Err(ChimeraError::Unsupported(
        "X25519 suite requires ECDH-derived shared secret path".to_string(),
    ))
}

pub fn finish_handshake_with_x25519_shared_secret(
    client_hello: &HandshakeMessage,
    server_hello: &HandshakeMessage,
    shared_secret: &X25519SharedSecret,
) -> ChimeraResult<EstablishedSession> {
    let suite_id = validated_suite_id(client_hello, server_hello)?;
    if suite_id != X25519_HKDF_SHA256_SUITE_ID {
        return Err(ChimeraError::Unsupported(
            "X25519 shared secret supplied for non-X25519 suite".to_string(),
        ));
    }
    finish_handshake_with_input_key_material(
        suite_id,
        client_hello,
        server_hello,
        shared_secret.as_bytes(),
    )
}

pub fn server_accept_hybrid_client_hello(
    message: &HybridHandshakeMessage,
    server_nonce: [u8; NONCE_LEN],
    server_x25519_key_share: [u8; X25519_PUBLIC_KEY_LEN],
) -> ChimeraResult<(HybridHandshakeMessage, [u8; 32])> {
    match message {
        HybridHandshakeMessage::ClientHello {
            suite_id,
            client_nonce,
            client_x25519_key_share,
            client_ml_kem_768_encapsulation_key,
        } if *suite_id == X25519_MLKEM768_HKDF_SHA256_SUITE_ID
            && *client_x25519_key_share != [0_u8; X25519_PUBLIC_KEY_LEN]
            && server_x25519_key_share != [0_u8; X25519_PUBLIC_KEY_LEN] =>
        {
            let (ciphertext, pq_shared_secret) =
                ml_kem_768_encapsulate(client_ml_kem_768_encapsulation_key)?;
            Ok((
                HybridHandshakeMessage::ServerHello {
                    suite_id: *suite_id,
                    client_nonce: *client_nonce,
                    server_nonce,
                    server_x25519_key_share,
                    ml_kem_768_ciphertext: ciphertext,
                },
                pq_shared_secret,
            ))
        }
        HybridHandshakeMessage::ClientHello { suite_id, .. }
            if *suite_id == X25519_MLKEM768_HKDF_SHA256_SUITE_ID =>
        {
            Err(ChimeraError::InvalidFrame(
                "hybrid key shares must be non-zero".to_string(),
            ))
        }
        HybridHandshakeMessage::ClientHello { suite_id, .. } => Err(ChimeraError::Unsupported(
            format!("unsupported hybrid suite id {suite_id}"),
        )),
        HybridHandshakeMessage::ServerHello { .. } => Err(ChimeraError::InvalidFrame(
            "server expected hybrid client hello".to_string(),
        )),
    }
}

pub fn finish_hybrid_handshake_with_shared_secrets(
    client_hello: &HybridHandshakeMessage,
    server_hello: &HybridHandshakeMessage,
    x25519_shared_secret: &X25519SharedSecret,
    ml_kem_768_shared_secret: &[u8; 32],
) -> ChimeraResult<EstablishedSession> {
    let suite_id = validated_hybrid_suite_id(client_hello, server_hello)?;
    if suite_id != X25519_MLKEM768_HKDF_SHA256_SUITE_ID {
        return Err(ChimeraError::Unsupported(
            "hybrid shared secrets supplied for non-hybrid suite".to_string(),
        ));
    }
    let transcript_hash =
        TranscriptHash::from_messages(&[&client_hello.encode()?, &server_hello.encode()?]);
    let traffic_secrets = derive_hybrid_traffic_secrets(
        SuiteId(suite_id),
        &transcript_hash,
        x25519_shared_secret.as_bytes(),
        ml_kem_768_shared_secret,
    )?;
    Ok(EstablishedSession {
        suite_id,
        transcript_hash,
        traffic_secrets,
    })
}

fn validated_hybrid_suite_id(
    client_hello: &HybridHandshakeMessage,
    server_hello: &HybridHandshakeMessage,
) -> ChimeraResult<u16> {
    let HybridHandshakeMessage::ClientHello {
        suite_id,
        client_nonce,
        ..
    } = client_hello
    else {
        return Err(ChimeraError::InvalidFrame(
            "hybrid transcript must start with client hello".to_string(),
        ));
    };
    let HybridHandshakeMessage::ServerHello {
        suite_id: server_suite_id,
        client_nonce: echoed_client_nonce,
        ..
    } = server_hello
    else {
        return Err(ChimeraError::InvalidFrame(
            "hybrid client expected server hello".to_string(),
        ));
    };
    if suite_id != server_suite_id {
        return Err(ChimeraError::Unsupported(
            "hybrid suite downgrade or mismatch rejected".to_string(),
        ));
    }
    if client_nonce != echoed_client_nonce {
        return Err(ChimeraError::InvalidFrame(
            "hybrid server hello echoed wrong client nonce".to_string(),
        ));
    }
    Ok(*suite_id)
}

fn validated_suite_id(
    client_hello: &HandshakeMessage,
    server_hello: &HandshakeMessage,
) -> ChimeraResult<u16> {
    let HandshakeMessage::ClientHello {
        suite_id,
        client_nonce,
        ..
    } = client_hello
    else {
        return Err(ChimeraError::InvalidFrame(
            "client transcript must start with client hello".to_string(),
        ));
    };

    let HandshakeMessage::ServerHello {
        suite_id: server_suite_id,
        client_nonce: echoed_client_nonce,
        ..
    } = server_hello
    else {
        return Err(ChimeraError::InvalidFrame(
            "client expected server hello".to_string(),
        ));
    };

    if *suite_id != *server_suite_id {
        return Err(ChimeraError::Unsupported(
            "suite downgrade or mismatch rejected".to_string(),
        ));
    }

    if client_nonce != echoed_client_nonce {
        return Err(ChimeraError::InvalidFrame(
            "server hello echoed wrong client nonce".to_string(),
        ));
    }

    Ok(*suite_id)
}

fn finish_handshake_with_input_key_material(
    suite_id: u16,
    client_hello: &HandshakeMessage,
    server_hello: &HandshakeMessage,
    input_key_material: &[u8],
) -> ChimeraResult<EstablishedSession> {
    let transcript_hash =
        TranscriptHash::from_messages(&[&client_hello.encode(), &server_hello.encode()]);
    let traffic_secrets =
        derive_traffic_secrets(SuiteId(suite_id), &transcript_hash, input_key_material)?;

    Ok(EstablishedSession {
        suite_id,
        transcript_hash,
        traffic_secrets,
    })
}

pub fn server_finish_handshake(
    client_hello: &HandshakeMessage,
    server_hello: &HandshakeMessage,
) -> ChimeraResult<EstablishedSession> {
    client_finish_handshake(client_hello, server_hello)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestOnlyInputKeyMaterial {
    bytes: [u8; 32],
}

impl TestOnlyInputKeyMaterial {
    fn expose_for_tests(&self) -> [u8; 32] {
        self.bytes
    }
}

fn derive_test_only_input_key_material(
    suite_id: u16,
    client_nonce: &[u8; NONCE_LEN],
    server_nonce: &[u8; NONCE_LEN],
) -> TestOnlyInputKeyMaterial {
    // This is deliberately not a KEM/ECDH shared secret. It is only temporary
    // input key material for M2 key-schedule wiring tests until real X25519 and
    // ML-KEM inputs are added.
    let mut bytes = [0_u8; 32];
    for index in 0..32 {
        let suite_byte = if index % 2 == 0 {
            (suite_id >> 8) as u8
        } else {
            suite_id as u8
        };
        bytes[index] = client_nonce[index]
            ^ server_nonce[31 - index]
            ^ suite_byte
            ^ (index as u8).wrapping_mul(17);
    }
    TestOnlyInputKeyMaterial { bytes }
}
