#![forbid(unsafe_code)]

use chimera_core::{ChimeraError, ChimeraResult};
use chimera_crypto::{
    SuiteId, TrafficSecret, TrafficSecrets, TranscriptHash, X25519_PUBLIC_KEY_LEN,
    X25519SharedSecret, decrypt_chacha20poly1305, derive_hybrid_traffic_secrets,
    derive_traffic_secrets, encrypt_chacha20poly1305, ml_kem_768_encapsulate,
};

pub const FRAME_VERSION: u8 = 1;
pub const HANDSHAKE_VERSION: u8 = 1;
pub const MAX_PAYLOAD_LEN: usize = 16 * 1024;
pub const NONCE_LEN: usize = 32;
pub const TEST_ONLY_SUITE_ID: u16 = 0x0001;
pub const X25519_HKDF_SHA256_SUITE_ID: u16 = 0x0101;
pub const X25519_MLKEM768_HKDF_SHA256_SUITE_ID: u16 = 0x0201;
const HEADER_LEN: usize = 13;
const CLIENT_HELLO_TYPE: u8 = 1;
const SERVER_HELLO_TYPE: u8 = 2;
const HYBRID_CLIENT_HELLO_TYPE: u8 = 3;
const HYBRID_SERVER_HELLO_TYPE: u8 = 4;
const HANDSHAKE_LEN: usize = 1 + 1 + 2 + NONCE_LEN + NONCE_LEN + X25519_PUBLIC_KEY_LEN;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub packet_number: u64,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn encode(&self) -> ChimeraResult<Vec<u8>> {
        if self.payload.len() > MAX_PAYLOAD_LEN {
            return Err(ChimeraError::InvalidFrame("payload too large".to_string()));
        }

        let payload_len = u32::try_from(self.payload.len())
            .map_err(|_| ChimeraError::InvalidFrame("payload length overflow".to_string()))?;
        let mut encoded = Vec::with_capacity(HEADER_LEN + self.payload.len());
        encoded.push(FRAME_VERSION);
        encoded.extend_from_slice(&self.packet_number.to_be_bytes());
        encoded.extend_from_slice(&payload_len.to_be_bytes());
        encoded.extend_from_slice(&self.payload);
        Ok(encoded)
    }

    pub fn decode(input: &[u8]) -> ChimeraResult<Self> {
        if input.len() < HEADER_LEN {
            return Err(ChimeraError::InvalidFrame("frame too short".to_string()));
        }

        if input[0] != FRAME_VERSION {
            return Err(ChimeraError::InvalidFrame(
                "unsupported frame version".to_string(),
            ));
        }

        let packet_number = read_u64(&input[1..9])?;
        let payload_len = read_u32(&input[9..13])? as usize;

        if payload_len > MAX_PAYLOAD_LEN {
            return Err(ChimeraError::InvalidFrame("payload too large".to_string()));
        }

        if input.len() != HEADER_LEN + payload_len {
            return Err(ChimeraError::InvalidFrame(
                "payload length mismatch".to_string(),
            ));
        }

        Ok(Self {
            packet_number,
            payload: input[HEADER_LEN..].to_vec(),
        })
    }
}

pub fn encrypt_frame_payload(
    packet_number: u64,
    plaintext: &[u8],
    traffic_secret: &TrafficSecret,
) -> ChimeraResult<Frame> {
    if plaintext.len() > MAX_PAYLOAD_LEN {
        return Err(ChimeraError::InvalidFrame("payload too large".to_string()));
    }
    let aad = frame_aad(packet_number);
    let payload = encrypt_chacha20poly1305(traffic_secret, packet_number, &aad, plaintext)?;
    Ok(Frame {
        packet_number,
        payload,
    })
}

pub fn decrypt_frame_payload(
    frame: &Frame,
    traffic_secret: &TrafficSecret,
) -> ChimeraResult<Vec<u8>> {
    let aad = frame_aad(frame.packet_number);
    decrypt_chacha20poly1305(traffic_secret, frame.packet_number, &aad, &frame.payload)
}

fn frame_aad(packet_number: u64) -> [u8; 9] {
    let mut aad = [0_u8; 9];
    aad[0] = FRAME_VERSION;
    aad[1..].copy_from_slice(&packet_number.to_be_bytes());
    aad
}

fn read_u64(bytes: &[u8]) -> ChimeraResult<u64> {
    let array: [u8; 8] = bytes
        .try_into()
        .map_err(|_| ChimeraError::InvalidFrame("invalid u64 field".to_string()))?;
    Ok(u64::from_be_bytes(array))
}

fn read_u32(bytes: &[u8]) -> ChimeraResult<u32> {
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| ChimeraError::InvalidFrame("invalid u32 field".to_string()))?;
    Ok(u32::from_be_bytes(array))
}

#[derive(Debug, Clone, Default)]
pub struct ReplayWindow {
    highest_seen: Option<u64>,
}

impl ReplayWindow {
    pub fn accept(&mut self, packet_number: u64) -> ChimeraResult<()> {
        if self
            .highest_seen
            .is_some_and(|highest| packet_number <= highest)
        {
            return Err(ChimeraError::ReplayDetected);
        }

        self.highest_seen = Some(packet_number);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RekeyPolicy {
    pub max_session_age_seconds: u64,
    pub max_packets_per_key: u64,
}

impl RekeyPolicy {
    pub fn validate(self) -> ChimeraResult<Self> {
        if self.max_session_age_seconds == 0 {
            return Err(ChimeraError::InvalidConfig(
                "rekey max_session_age_seconds must be greater than zero".to_string(),
            ));
        }
        if self.max_packets_per_key == 0 {
            return Err(ChimeraError::InvalidConfig(
                "rekey max_packets_per_key must be greater than zero".to_string(),
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RekeyState {
    policy: RekeyPolicy,
    established_at_seconds: u64,
    sent_packets_with_current_key: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RekeyReason {
    SessionAgeExceeded,
    PacketLimitExceeded,
}

impl RekeyState {
    pub fn new(policy: RekeyPolicy, established_at_seconds: u64) -> ChimeraResult<Self> {
        Ok(Self {
            policy: policy.validate()?,
            established_at_seconds,
            sent_packets_with_current_key: 0,
        })
    }

    pub fn on_packet_sent(&mut self) {
        self.sent_packets_with_current_key = self.sent_packets_with_current_key.saturating_add(1);
    }

    pub fn rekey_reason(&self, now_seconds: u64) -> Option<RekeyReason> {
        let age_seconds = now_seconds.saturating_sub(self.established_at_seconds);
        if age_seconds >= self.policy.max_session_age_seconds {
            return Some(RekeyReason::SessionAgeExceeded);
        }
        if self.sent_packets_with_current_key >= self.policy.max_packets_per_key {
            return Some(RekeyReason::PacketLimitExceeded);
        }
        None
    }

    pub fn should_rekey(&self, now_seconds: u64) -> bool {
        self.rekey_reason(now_seconds).is_some()
    }

    pub fn reset_after_rekey(&mut self, now_seconds: u64) {
        self.established_at_seconds = now_seconds;
        self.sent_packets_with_current_key = 0;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeMessage {
    ClientHello {
        suite_id: u16,
        client_nonce: [u8; NONCE_LEN],
        client_key_share: [u8; X25519_PUBLIC_KEY_LEN],
    },
    ServerHello {
        suite_id: u16,
        client_nonce: [u8; NONCE_LEN],
        server_nonce: [u8; NONCE_LEN],
        server_key_share: [u8; X25519_PUBLIC_KEY_LEN],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HybridHandshakeMessage {
    ClientHello {
        suite_id: u16,
        client_nonce: [u8; NONCE_LEN],
        client_x25519_key_share: [u8; X25519_PUBLIC_KEY_LEN],
        client_ml_kem_768_encapsulation_key: Vec<u8>,
    },
    ServerHello {
        suite_id: u16,
        client_nonce: [u8; NONCE_LEN],
        server_nonce: [u8; NONCE_LEN],
        server_x25519_key_share: [u8; X25519_PUBLIC_KEY_LEN],
        ml_kem_768_ciphertext: Vec<u8>,
    },
}

impl HybridHandshakeMessage {
    pub fn encode(&self) -> ChimeraResult<Vec<u8>> {
        let mut encoded = Vec::new();
        encoded.push(HANDSHAKE_VERSION);
        match self {
            Self::ClientHello {
                suite_id,
                client_nonce,
                client_x25519_key_share,
                client_ml_kem_768_encapsulation_key,
            } => {
                push_len_u16(
                    client_ml_kem_768_encapsulation_key.len(),
                    "ML-KEM public key",
                )?;
                encoded.push(HYBRID_CLIENT_HELLO_TYPE);
                encoded.extend_from_slice(&suite_id.to_be_bytes());
                encoded.extend_from_slice(client_nonce);
                encoded.extend_from_slice(client_x25519_key_share);
                push_vec_u16(&mut encoded, client_ml_kem_768_encapsulation_key)?;
            }
            Self::ServerHello {
                suite_id,
                client_nonce,
                server_nonce,
                server_x25519_key_share,
                ml_kem_768_ciphertext,
            } => {
                push_len_u16(ml_kem_768_ciphertext.len(), "ML-KEM ciphertext")?;
                encoded.push(HYBRID_SERVER_HELLO_TYPE);
                encoded.extend_from_slice(&suite_id.to_be_bytes());
                encoded.extend_from_slice(client_nonce);
                encoded.extend_from_slice(server_nonce);
                encoded.extend_from_slice(server_x25519_key_share);
                push_vec_u16(&mut encoded, ml_kem_768_ciphertext)?;
            }
        }
        Ok(encoded)
    }

    pub fn decode(input: &[u8]) -> ChimeraResult<Self> {
        if input.len() < 1 + 1 + 2 + NONCE_LEN + X25519_PUBLIC_KEY_LEN + 2 {
            return Err(ChimeraError::InvalidFrame(
                "hybrid handshake too short".to_string(),
            ));
        }
        if input[0] != HANDSHAKE_VERSION {
            return Err(ChimeraError::InvalidFrame(
                "unsupported hybrid handshake version".to_string(),
            ));
        }
        let message_type = input[1];
        let suite_id = u16::from_be_bytes([input[2], input[3]]);
        match message_type {
            HYBRID_CLIENT_HELLO_TYPE => {
                let min_len = 1 + 1 + 2 + NONCE_LEN + X25519_PUBLIC_KEY_LEN + 2;
                if input.len() < min_len {
                    return Err(ChimeraError::InvalidFrame(
                        "hybrid client hello too short".to_string(),
                    ));
                }
                let client_nonce = read_nonce(&input[4..36])?;
                let client_x25519_key_share = read_key_share(&input[36..68])?;
                let (client_ml_kem_768_encapsulation_key, consumed) = read_vec_u16(&input[68..])?;
                if 68 + consumed != input.len() {
                    return Err(ChimeraError::InvalidFrame(
                        "hybrid client hello has trailing bytes".to_string(),
                    ));
                }
                Ok(Self::ClientHello {
                    suite_id,
                    client_nonce,
                    client_x25519_key_share,
                    client_ml_kem_768_encapsulation_key,
                })
            }
            HYBRID_SERVER_HELLO_TYPE => {
                let min_len = 1 + 1 + 2 + NONCE_LEN + NONCE_LEN + X25519_PUBLIC_KEY_LEN + 2;
                if input.len() < min_len {
                    return Err(ChimeraError::InvalidFrame(
                        "hybrid server hello too short".to_string(),
                    ));
                }
                let client_nonce = read_nonce(&input[4..36])?;
                let server_nonce = read_nonce(&input[36..68])?;
                let server_x25519_key_share = read_key_share(&input[68..100])?;
                let (ml_kem_768_ciphertext, consumed) = read_vec_u16(&input[100..])?;
                if 100 + consumed != input.len() {
                    return Err(ChimeraError::InvalidFrame(
                        "hybrid server hello has trailing bytes".to_string(),
                    ));
                }
                Ok(Self::ServerHello {
                    suite_id,
                    client_nonce,
                    server_nonce,
                    server_x25519_key_share,
                    ml_kem_768_ciphertext,
                })
            }
            _ => Err(ChimeraError::InvalidFrame(
                "unknown hybrid handshake message type".to_string(),
            )),
        }
    }
}

impl HandshakeMessage {
    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(HANDSHAKE_LEN);
        encoded.push(HANDSHAKE_VERSION);
        match self {
            Self::ClientHello {
                suite_id,
                client_nonce,
                client_key_share,
            } => {
                encoded.push(CLIENT_HELLO_TYPE);
                encoded.extend_from_slice(&suite_id.to_be_bytes());
                encoded.extend_from_slice(client_nonce);
                encoded.extend_from_slice(&[0_u8; NONCE_LEN]);
                encoded.extend_from_slice(client_key_share);
            }
            Self::ServerHello {
                suite_id,
                client_nonce,
                server_nonce,
                server_key_share,
            } => {
                encoded.push(SERVER_HELLO_TYPE);
                encoded.extend_from_slice(&suite_id.to_be_bytes());
                encoded.extend_from_slice(client_nonce);
                encoded.extend_from_slice(server_nonce);
                encoded.extend_from_slice(server_key_share);
            }
        }
        encoded
    }

    pub fn decode(input: &[u8]) -> ChimeraResult<Self> {
        if input.len() != HANDSHAKE_LEN {
            return Err(ChimeraError::InvalidFrame(
                "invalid handshake length".to_string(),
            ));
        }

        if input[0] != HANDSHAKE_VERSION {
            return Err(ChimeraError::InvalidFrame(
                "unsupported handshake version".to_string(),
            ));
        }

        let message_type = input[1];
        let suite_id = u16::from_be_bytes([input[2], input[3]]);
        let client_nonce = read_nonce(&input[4..36])?;
        let server_nonce = read_nonce(&input[36..68])?;
        let key_share = read_key_share(&input[68..100])?;

        match message_type {
            CLIENT_HELLO_TYPE => {
                if server_nonce != [0_u8; NONCE_LEN] {
                    return Err(ChimeraError::InvalidFrame(
                        "client hello must not carry server nonce".to_string(),
                    ));
                }
                Ok(Self::ClientHello {
                    suite_id,
                    client_nonce,
                    client_key_share: key_share,
                })
            }
            SERVER_HELLO_TYPE => Ok(Self::ServerHello {
                suite_id,
                client_nonce,
                server_nonce,
                server_key_share: key_share,
            }),
            _ => Err(ChimeraError::InvalidFrame(
                "unknown handshake message type".to_string(),
            )),
        }
    }
}

fn read_nonce(bytes: &[u8]) -> ChimeraResult<[u8; NONCE_LEN]> {
    bytes
        .try_into()
        .map_err(|_| ChimeraError::InvalidFrame("invalid nonce field".to_string()))
}

fn read_key_share(bytes: &[u8]) -> ChimeraResult<[u8; X25519_PUBLIC_KEY_LEN]> {
    bytes
        .try_into()
        .map_err(|_| ChimeraError::InvalidFrame("invalid key share field".to_string()))
}

fn push_len_u16(len: usize, field: &str) -> ChimeraResult<()> {
    if len == 0 || len > u16::MAX as usize {
        return Err(ChimeraError::InvalidFrame(format!(
            "{field} length is invalid"
        )));
    }
    Ok(())
}

fn push_vec_u16(out: &mut Vec<u8>, value: &[u8]) -> ChimeraResult<()> {
    push_len_u16(value.len(), "variable field")?;
    let len = u16::try_from(value.len())
        .map_err(|_| ChimeraError::InvalidFrame("variable field too large".to_string()))?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(value);
    Ok(())
}

fn read_vec_u16(input: &[u8]) -> ChimeraResult<(Vec<u8>, usize)> {
    if input.len() < 2 {
        return Err(ChimeraError::InvalidFrame(
            "variable field length missing".to_string(),
        ));
    }
    let len = u16::from_be_bytes([input[0], input[1]]) as usize;
    if len == 0 {
        return Err(ChimeraError::InvalidFrame(
            "variable field is empty".to_string(),
        ));
    }
    if input.len() < 2 + len {
        return Err(ChimeraError::InvalidFrame(
            "variable field truncated".to_string(),
        ));
    }
    Ok((input[2..2 + len].to_vec(), 2 + len))
}

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

#[cfg(test)]
mod tests {
    use super::{
        Frame, HandshakeMessage, HybridHandshakeMessage, RekeyPolicy, RekeyReason, RekeyState,
        ReplayWindow, TEST_ONLY_SUITE_ID, X25519_MLKEM768_HKDF_SHA256_SUITE_ID,
        client_finish_handshake, decrypt_frame_payload, encrypt_frame_payload,
        finish_hybrid_handshake_with_shared_secrets, server_accept_client_hello,
        server_accept_hybrid_client_hello, server_finish_handshake,
    };
    use chimera_crypto::{X25519Secret, ml_kem_768_decapsulate, ml_kem_768_generate_keypair};

    #[test]
    fn frame_round_trip() {
        let frame = Frame {
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
                .client_to_gateway
                .expose_for_tests(),
            server_session
                .traffic_secrets
                .client_to_gateway
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
                .client_to_gateway
                .expose_for_tests(),
            server_session
                .traffic_secrets
                .client_to_gateway
                .expose_for_tests()
        );
        assert_eq!(
            client_session
                .traffic_secrets
                .gateway_to_client
                .expose_for_tests(),
            server_session
                .traffic_secrets
                .gateway_to_client
                .expose_for_tests()
        );
    }

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
            &client_session.traffic_secrets.client_to_gateway,
        )
        .unwrap_or_else(|error| unreachable!("frame should encrypt: {error}"));
        assert_ne!(encrypted.payload, b"application payload");

        let decrypted = decrypt_frame_payload(
            &encrypted,
            &server_session.traffic_secrets.client_to_gateway,
        )
        .unwrap_or_else(|error| unreachable!("frame should decrypt: {error}"));
        assert_eq!(decrypted, b"application payload");
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
        let mut encrypted =
            encrypt_frame_payload(7, b"payload", &session.traffic_secrets.client_to_gateway)
                .unwrap_or_else(|error| unreachable!("frame should encrypt: {error}"));
        encrypted.packet_number = 8;

        let decrypted =
            decrypt_frame_payload(&encrypted, &session.traffic_secrets.client_to_gateway);
        assert!(decrypted.is_err());
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
        let second_server_hello = match server_accept_client_hello(&second_client_hello, [9_u8; 32])
        {
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
            first.traffic_secrets.client_to_gateway.expose_for_tests(),
            second.traffic_secrets.client_to_gateway.expose_for_tests()
        );
    }

    #[test]
    fn rekey_policy_rejects_zero_thresholds() {
        assert!(
            RekeyPolicy {
                max_session_age_seconds: 0,
                max_packets_per_key: 1
            }
            .validate()
            .is_err()
        );
        assert!(
            RekeyPolicy {
                max_session_age_seconds: 1,
                max_packets_per_key: 0
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn rekey_triggers_by_packet_count() {
        let mut state = match RekeyState::new(
            RekeyPolicy {
                max_session_age_seconds: 60,
                max_packets_per_key: 3,
            },
            100,
        ) {
            Ok(state) => state,
            Err(error) => unreachable!("rekey state should be created: {error}"),
        };

        assert!(!state.should_rekey(100));
        state.on_packet_sent();
        state.on_packet_sent();
        assert!(!state.should_rekey(100));
        state.on_packet_sent();
        assert!(state.should_rekey(100));
        assert_eq!(
            state.rekey_reason(100),
            Some(RekeyReason::PacketLimitExceeded)
        );
    }

    #[test]
    fn rekey_triggers_by_session_age() {
        let state = match RekeyState::new(
            RekeyPolicy {
                max_session_age_seconds: 10,
                max_packets_per_key: 100,
            },
            500,
        ) {
            Ok(state) => state,
            Err(error) => unreachable!("rekey state should be created: {error}"),
        };

        assert!(!state.should_rekey(509));
        assert!(state.should_rekey(510));
        assert!(state.should_rekey(511));
        assert_eq!(
            state.rekey_reason(510),
            Some(RekeyReason::SessionAgeExceeded)
        );
    }

    #[test]
    fn rekey_reset_clears_triggers() {
        let mut state = match RekeyState::new(
            RekeyPolicy {
                max_session_age_seconds: 5,
                max_packets_per_key: 2,
            },
            10,
        ) {
            Ok(state) => state,
            Err(error) => unreachable!("rekey state should be created: {error}"),
        };

        state.on_packet_sent();
        state.on_packet_sent();
        assert!(state.should_rekey(12));

        state.reset_after_rekey(12);
        assert!(!state.should_rekey(12));
        assert!(!state.should_rekey(16));
        assert!(state.should_rekey(17));
    }
}
