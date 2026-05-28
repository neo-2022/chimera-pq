use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;

use sha2::{Digest, Sha256};

use chimera_crypto::{
    SuiteId, TrafficSecret, TranscriptHash, X25519Secret, derive_hybrid_traffic_secrets,
    ml_kem_768_decapsulate, ml_kem_768_encapsulate, ml_kem_768_generate_keypair,
};

use crate::peer_egress::options::{
    AeadSuite, SECURE_MAGIC, SECURE_NONCE_LEN, HANDSHAKE_MAGIC, MAX_TOKEN_LEN,
};
use crate::peer_egress::protocol::{read_line_limited, SecurePeerStream};

pub fn authenticate_peer(stream: &mut TcpStream, token: &str) -> Result<(), String> {
    let mut magic = vec![0_u8; HANDSHAKE_MAGIC.len()];
    stream
        .read_exact(&mut magic)
        .map_err(|error| format!("read magic failed: {error}"))?;
    if magic != HANDSHAKE_MAGIC {
        return Err("bad peer magic".to_string());
    }
    let line = read_line_limited(stream, MAX_TOKEN_LEN + 1)?;
    if line != token {
        return Err("bad peer token".to_string());
    }
    Ok(())
}

pub fn establish_secure_peer_client(
    mut stream: TcpStream,
    token: &str,
    aead: AeadSuite,
) -> Result<SecurePeerStream, String> {
    let client_nonce = random_nonce()?;
    let client_x25519 = X25519Secret::from_private_bytes(random_nonce()?);
    let client_x25519_public = client_x25519.public_key_bytes();
    stream
        .write_all(SECURE_MAGIC)
        .and_then(|_| stream.write_all(&aead.suite_id().to_be_bytes()))
        .and_then(|_| stream.write_all(&client_nonce))
        .and_then(|_| stream.write_all(&client_x25519_public))
        .map_err(|error| format!("write secure client hello failed: {error}"))?;
    let mut magic = vec![0_u8; SECURE_MAGIC.len()];
    stream
        .read_exact(&mut magic)
        .map_err(|error| format!("read secure server hello magic failed: {error}"))?;
    if magic != SECURE_MAGIC {
        return Err("bad secure server magic".to_string());
    }
    let mut server_suite_raw = [0_u8; 2];
    stream
        .read_exact(&mut server_suite_raw)
        .map_err(|error| format!("read secure server suite failed: {error}"))?;
    let server_suite = u16::from_be_bytes(server_suite_raw);
    if server_suite != aead.suite_id() {
        return Err("secure server AEAD suite mismatch".to_string());
    }
    let mut server_nonce = [0_u8; SECURE_NONCE_LEN];
    stream
        .read_exact(&mut server_nonce)
        .map_err(|error| format!("read secure server nonce failed: {error}"))?;
    let mut server_x25519_public = [0_u8; 32];
    stream
        .read_exact(&mut server_x25519_public)
        .map_err(|error| format!("read secure server x25519 key failed: {error}"))?;
    let server_ml_kem_public = read_len_prefixed_vec(&mut stream, 4096)?;
    let (ml_kem_ciphertext, pq_shared_secret) = ml_kem_768_encapsulate(&server_ml_kem_public)
        .map_err(|error| {
            format!("ML-KEM-768 encapsulate failed during secure client handshake: {error}")
        })?;
    write_len_prefixed_vec(&mut stream, &ml_kem_ciphertext)?;
    let x25519_shared_secret = client_x25519.diffie_hellman(server_x25519_public);
    let (client_to_server, server_to_client) = derive_secure_peer_secrets(
        aead,
        token,
        &client_nonce,
        &server_nonce,
        &client_x25519_public,
        &server_x25519_public,
        &server_ml_kem_public,
        &ml_kem_ciphertext,
        x25519_shared_secret.as_bytes(),
        &pq_shared_secret,
    )?;
    Ok(SecurePeerStream {
        stream,
        send_secret: client_to_server,
        recv_secret: server_to_client,
        send_packet: 0,
        recv_packet: 0,
        aead,
    })
}

pub fn establish_secure_peer_server(
    mut stream: TcpStream,
    token: &str,
    aead: AeadSuite,
) -> Result<SecurePeerStream, String> {
    let mut magic = vec![0_u8; SECURE_MAGIC.len()];
    stream
        .read_exact(&mut magic)
        .map_err(|error| format!("read secure client hello magic failed: {error}"))?;
    if magic != SECURE_MAGIC {
        return Err("bad secure client magic".to_string());
    }
    let mut client_suite_raw = [0_u8; 2];
    stream
        .read_exact(&mut client_suite_raw)
        .map_err(|error| format!("read secure client suite failed: {error}"))?;
    let client_suite = u16::from_be_bytes(client_suite_raw);
    if client_suite != aead.suite_id() {
        return Err("secure client AEAD suite mismatch".to_string());
    }
    let mut client_nonce = [0_u8; SECURE_NONCE_LEN];
    stream
        .read_exact(&mut client_nonce)
        .map_err(|error| format!("read secure client nonce failed: {error}"))?;
    let mut client_x25519_public = [0_u8; 32];
    stream
        .read_exact(&mut client_x25519_public)
        .map_err(|error| format!("read secure client x25519 key failed: {error}"))?;
    let server_nonce = random_nonce()?;
    let server_x25519 = X25519Secret::from_private_bytes(random_nonce()?);
    let server_x25519_public = server_x25519.public_key_bytes();
    let (server_ml_kem_decapsulation_key, server_ml_kem_public) = ml_kem_768_generate_keypair();
    stream
        .write_all(SECURE_MAGIC)
        .and_then(|_| stream.write_all(&aead.suite_id().to_be_bytes()))
        .and_then(|_| stream.write_all(&server_nonce))
        .and_then(|_| stream.write_all(&server_x25519_public))
        .map_err(|error| format!("write secure server hello failed: {error}"))?;
    write_len_prefixed_vec(&mut stream, &server_ml_kem_public)?;
    let ml_kem_ciphertext = read_len_prefixed_vec(&mut stream, 4096)?;
    let pq_shared_secret =
        ml_kem_768_decapsulate(&server_ml_kem_decapsulation_key, &ml_kem_ciphertext).map_err(
            |error| {
                format!("ML-KEM-768 decapsulate failed during secure server handshake: {error}")
            },
        )?;
    let x25519_shared_secret = server_x25519.diffie_hellman(client_x25519_public);
    let (client_to_server, server_to_client) = derive_secure_peer_secrets(
        aead,
        token,
        &client_nonce,
        &server_nonce,
        &client_x25519_public,
        &server_x25519_public,
        &server_ml_kem_public,
        &ml_kem_ciphertext,
        x25519_shared_secret.as_bytes(),
        &pq_shared_secret,
    )?;
    Ok(SecurePeerStream {
        stream,
        send_secret: server_to_client,
        recv_secret: client_to_server,
        send_packet: 0,
        recv_packet: 0,
        aead,
    })
}

#[allow(clippy::too_many_arguments)]
fn derive_secure_peer_secrets(
    aead: AeadSuite,
    token: &str,
    client_nonce: &[u8; SECURE_NONCE_LEN],
    server_nonce: &[u8; SECURE_NONCE_LEN],
    client_x25519_public: &[u8; 32],
    server_x25519_public: &[u8; 32],
    ml_kem_public: &[u8],
    ml_kem_ciphertext: &[u8],
    x25519_shared_secret: &[u8; 32],
    pq_shared_secret: &[u8; 32],
) -> Result<(TrafficSecret, TrafficSecret), String> {
    let token_digest = Sha256::digest(token.as_bytes());
    let transcript = TranscriptHash::from_messages(&[
        b"peer-egress-secure-v3-x25519-mlkem768-aead",
        aead.wire_name().as_bytes(),
        token_digest.as_ref(),
        client_nonce,
        server_nonce,
        client_x25519_public,
        server_x25519_public,
        ml_kem_public,
        ml_kem_ciphertext,
    ]);
    let secrets = derive_hybrid_traffic_secrets(
        SuiteId(aead.suite_id()),
        &transcript,
        x25519_shared_secret,
        pq_shared_secret,
    )
    .map_err(|error| format!("derive secure peer secrets failed: {error}"))?;
    Ok((
        secrets.client_to_gateway.clone(),
        secrets.gateway_to_client.clone(),
    ))
}

fn random_nonce() -> Result<[u8; SECURE_NONCE_LEN], String> {
    let mut nonce = [0_u8; SECURE_NONCE_LEN];
    let mut file =
        File::open("/dev/urandom").map_err(|error| format!("open urandom failed: {error}"))?;
    file.read_exact(&mut nonce)
        .map_err(|error| format!("read urandom failed: {error}"))?;
    Ok(nonce)
}

fn write_len_prefixed_vec(stream: &mut TcpStream, value: &[u8]) -> Result<(), String> {
    let len =
        u16::try_from(value.len()).map_err(|_| "length-prefixed value too large".to_string())?;
    if len == 0 {
        return Err("length-prefixed value is empty".to_string());
    }
    stream
        .write_all(&len.to_be_bytes())
        .and_then(|_| stream.write_all(value))
        .map_err(|error| format!("write length-prefixed value failed: {error}"))
}

fn read_len_prefixed_vec(stream: &mut TcpStream, max_len: usize) -> Result<Vec<u8>, String> {
    let mut len_raw = [0_u8; 2];
    stream
        .read_exact(&mut len_raw)
        .map_err(|error| format!("read length-prefixed length failed: {error}"))?;
    let len = u16::from_be_bytes(len_raw) as usize;
    if len == 0 || len > max_len {
        return Err("length-prefixed value length invalid".to_string());
    }
    let mut value = vec![0_u8; len];
    stream
        .read_exact(&mut value)
        .map_err(|error| format!("read length-prefixed value failed: {error}"))?;
    Ok(value)
}
