use chimera_core::{ChimeraError, ChimeraResult};
use chimera_crypto::X25519_PUBLIC_KEY_LEN;

pub const HANDSHAKE_VERSION: u8 = 1;
pub const NONCE_LEN: usize = 32;

const CLIENT_HELLO_TYPE: u8 = 1;
const SERVER_HELLO_TYPE: u8 = 2;
const HYBRID_CLIENT_HELLO_TYPE: u8 = 3;
const HYBRID_SERVER_HELLO_TYPE: u8 = 4;
const HANDSHAKE_LEN: usize = 1 + 1 + 2 + NONCE_LEN + NONCE_LEN + X25519_PUBLIC_KEY_LEN;

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
