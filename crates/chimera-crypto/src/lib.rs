#![forbid(unsafe_code)]

use aes_gcm::{Aes256Gcm, Key as AesKey, Nonce as AesNonce};
use chacha20poly1305::aead::{Aead, AeadInPlace, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, Nonce};
use hkdf::Hkdf;
use ml_kem::MlKem768;
use ml_kem::kem::{Decapsulate, Encapsulate, Kem, KeyExport};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

use chimera_core::{ChimeraError, ChimeraResult};

pub const TRAFFIC_SECRET_LEN: usize = 32;
pub const X25519_PUBLIC_KEY_LEN: usize = 32;
pub const X25519_SHARED_SECRET_LEN: usize = 32;
pub const ML_KEM_768_SHARED_SECRET_LEN: usize = 32;
const AEAD_NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuiteId(pub u16);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptHash([u8; 32]);

impl TranscriptHash {
    pub fn from_messages(messages: &[&[u8]]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"CHIMERA-PQ transcript v1");
        for message in messages {
            hasher.update((message.len() as u64).to_be_bytes());
            hasher.update(message);
        }
        let digest = hasher.finalize();
        let mut out = [0_u8; 32];
        out.copy_from_slice(&digest);
        Self(out)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct TrafficSecret([u8; TRAFFIC_SECRET_LEN]);

impl TrafficSecret {
    pub fn expose_for_tests(&self) -> [u8; TRAFFIC_SECRET_LEN] {
        self.0
    }
}

impl core::fmt::Debug for TrafficSecret {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("TrafficSecret(<redacted>)")
    }
}

#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct TrafficSecrets {
    pub client_to_gateway: TrafficSecret,
    pub gateway_to_client: TrafficSecret,
}

pub fn derive_traffic_secrets(
    suite_id: SuiteId,
    transcript_hash: &TranscriptHash,
    input_key_material: &[u8],
) -> ChimeraResult<TrafficSecrets> {
    if input_key_material.len() < 32 {
        return Err(ChimeraError::InvalidConfig(
            "input key material must be at least 32 bytes".to_string(),
        ));
    }

    let mut salt = Vec::with_capacity(2 + transcript_hash.as_bytes().len());
    salt.extend_from_slice(&suite_id.0.to_be_bytes());
    salt.extend_from_slice(transcript_hash.as_bytes());

    let hk = Hkdf::<Sha256>::new(Some(&salt), input_key_material);
    let client_to_gateway = expand_secret(&hk, b"client-to-gateway traffic secret")?;
    let gateway_to_client = expand_secret(&hk, b"gateway-to-client traffic secret")?;

    Ok(TrafficSecrets {
        client_to_gateway: TrafficSecret(client_to_gateway),
        gateway_to_client: TrafficSecret(gateway_to_client),
    })
}

pub fn derive_hybrid_traffic_secrets(
    suite_id: SuiteId,
    transcript_hash: &TranscriptHash,
    classical_input_key_material: &[u8],
    pq_input_key_material: &[u8],
) -> ChimeraResult<TrafficSecrets> {
    if classical_input_key_material.len() < 32 {
        return Err(ChimeraError::InvalidConfig(
            "classical input key material must be at least 32 bytes".to_string(),
        ));
    }
    if pq_input_key_material.len() < 32 {
        return Err(ChimeraError::InvalidConfig(
            "PQ input key material must be at least 32 bytes".to_string(),
        ));
    }
    let mut hybrid_ikm =
        Vec::with_capacity(18 + classical_input_key_material.len() + pq_input_key_material.len());
    hybrid_ikm.extend_from_slice(b"classical:");
    hybrid_ikm.extend_from_slice(classical_input_key_material);
    hybrid_ikm.extend_from_slice(b";pq:");
    hybrid_ikm.extend_from_slice(pq_input_key_material);
    derive_traffic_secrets(suite_id, transcript_hash, &hybrid_ikm)
}

pub fn encrypt_chacha20poly1305(
    secret: &TrafficSecret,
    packet_number: u64,
    associated_data: &[u8],
    plaintext: &[u8],
) -> ChimeraResult<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&secret.0));
    cipher
        .encrypt(
            Nonce::from_slice(&packet_nonce(packet_number)),
            Payload {
                msg: plaintext,
                aad: associated_data,
            },
        )
        .map_err(|_| ChimeraError::InvalidFrame("AEAD encrypt failed".to_string()))
}

pub fn encrypt_chacha20poly1305_in_place(
    secret: &TrafficSecret,
    packet_number: u64,
    associated_data: &[u8],
    buffer: &mut Vec<u8>,
) -> ChimeraResult<()> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&secret.0));
    cipher
        .encrypt_in_place(
            Nonce::from_slice(&packet_nonce(packet_number)),
            associated_data,
            buffer,
        )
        .map_err(|_| ChimeraError::InvalidFrame("AEAD in-place encrypt failed".to_string()))
}

pub fn decrypt_chacha20poly1305(
    secret: &TrafficSecret,
    packet_number: u64,
    associated_data: &[u8],
    ciphertext: &[u8],
) -> ChimeraResult<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&secret.0));
    cipher
        .decrypt(
            Nonce::from_slice(&packet_nonce(packet_number)),
            Payload {
                msg: ciphertext,
                aad: associated_data,
            },
        )
        .map_err(|_| ChimeraError::InvalidFrame("AEAD decrypt failed".to_string()))
}

pub fn decrypt_chacha20poly1305_in_place(
    secret: &TrafficSecret,
    packet_number: u64,
    associated_data: &[u8],
    buffer: &mut Vec<u8>,
) -> ChimeraResult<()> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&secret.0));
    cipher
        .decrypt_in_place(
            Nonce::from_slice(&packet_nonce(packet_number)),
            associated_data,
            buffer,
        )
        .map_err(|_| ChimeraError::InvalidFrame("AEAD in-place decrypt failed".to_string()))
}

pub fn encrypt_aes256gcm(
    secret: &TrafficSecret,
    packet_number: u64,
    associated_data: &[u8],
    plaintext: &[u8],
) -> ChimeraResult<Vec<u8>> {
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(&secret.0));
    cipher
        .encrypt(
            AesNonce::from_slice(&packet_nonce(packet_number)),
            Payload {
                msg: plaintext,
                aad: associated_data,
            },
        )
        .map_err(|_| ChimeraError::InvalidFrame("AES-256-GCM encrypt failed".to_string()))
}

pub fn encrypt_aes256gcm_in_place(
    secret: &TrafficSecret,
    packet_number: u64,
    associated_data: &[u8],
    buffer: &mut Vec<u8>,
) -> ChimeraResult<()> {
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(&secret.0));
    cipher
        .encrypt_in_place(
            AesNonce::from_slice(&packet_nonce(packet_number)),
            associated_data,
            buffer,
        )
        .map_err(|_| ChimeraError::InvalidFrame("AES-256-GCM in-place encrypt failed".to_string()))
}

pub fn decrypt_aes256gcm(
    secret: &TrafficSecret,
    packet_number: u64,
    associated_data: &[u8],
    ciphertext: &[u8],
) -> ChimeraResult<Vec<u8>> {
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(&secret.0));
    cipher
        .decrypt(
            AesNonce::from_slice(&packet_nonce(packet_number)),
            Payload {
                msg: ciphertext,
                aad: associated_data,
            },
        )
        .map_err(|_| ChimeraError::InvalidFrame("AES-256-GCM decrypt failed".to_string()))
}

pub fn decrypt_aes256gcm_in_place(
    secret: &TrafficSecret,
    packet_number: u64,
    associated_data: &[u8],
    buffer: &mut Vec<u8>,
) -> ChimeraResult<()> {
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(&secret.0));
    cipher
        .decrypt_in_place(
            AesNonce::from_slice(&packet_nonce(packet_number)),
            associated_data,
            buffer,
        )
        .map_err(|_| ChimeraError::InvalidFrame("AES-256-GCM in-place decrypt failed".to_string()))
}

pub struct MlKem768DecapsulationKey(<MlKem768 as Kem>::DecapsulationKey);

impl core::fmt::Debug for MlKem768DecapsulationKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("MlKem768DecapsulationKey(<redacted>)")
    }
}

pub fn ml_kem_768_generate_keypair() -> (MlKem768DecapsulationKey, Vec<u8>) {
    let (decapsulation_key, encapsulation_key) = MlKem768::generate_keypair();
    let encapsulation_key_bytes = encapsulation_key.to_bytes();
    (
        MlKem768DecapsulationKey(decapsulation_key),
        encapsulation_key_bytes.as_slice().to_vec(),
    )
}

pub fn ml_kem_768_encapsulate(
    encapsulation_key_bytes: &[u8],
) -> ChimeraResult<(Vec<u8>, [u8; ML_KEM_768_SHARED_SECRET_LEN])> {
    let encapsulation_key =
        <MlKem768 as Kem>::EncapsulationKey::new(encapsulation_key_bytes.try_into().map_err(
            |_| ChimeraError::InvalidConfig("invalid ML-KEM-768 public key length".to_string()),
        )?)
        .map_err(|_| ChimeraError::InvalidConfig("invalid ML-KEM-768 public key".to_string()))?;
    let (ciphertext, shared_key) = encapsulation_key.encapsulate();
    let mut out = [0_u8; ML_KEM_768_SHARED_SECRET_LEN];
    out.copy_from_slice(shared_key.as_ref());
    Ok((ciphertext.as_slice().to_vec(), out))
}

pub fn ml_kem_768_decapsulate(
    decapsulation_key: &MlKem768DecapsulationKey,
    ciphertext: &[u8],
) -> ChimeraResult<[u8; ML_KEM_768_SHARED_SECRET_LEN]> {
    let shared_key = decapsulation_key
        .0
        .decapsulate_slice(ciphertext)
        .map_err(|_| ChimeraError::InvalidConfig("invalid ML-KEM-768 ciphertext".to_string()))?;
    let mut out = [0_u8; ML_KEM_768_SHARED_SECRET_LEN];
    out.copy_from_slice(shared_key.as_ref());
    Ok(out)
}

fn packet_nonce(packet_number: u64) -> [u8; AEAD_NONCE_LEN] {
    let mut nonce = [0_u8; AEAD_NONCE_LEN];
    nonce[4..].copy_from_slice(&packet_number.to_be_bytes());
    nonce
}

fn expand_secret(hk: &Hkdf<Sha256>, info: &[u8]) -> ChimeraResult<[u8; TRAFFIC_SECRET_LEN]> {
    let mut out = [0_u8; TRAFFIC_SECRET_LEN];
    hk.expand(info, &mut out).map_err(|_| {
        ChimeraError::InvalidConfig("HKDF expand failed for traffic secret".to_string())
    })?;
    Ok(out)
}

pub struct X25519Secret {
    secret: StaticSecret,
}

impl X25519Secret {
    pub fn from_private_bytes(private_bytes: [u8; 32]) -> Self {
        Self {
            secret: StaticSecret::from(private_bytes),
        }
    }

    pub fn public_key_bytes(&self) -> [u8; X25519_PUBLIC_KEY_LEN] {
        PublicKey::from(&self.secret).to_bytes()
    }

    pub fn diffie_hellman(
        &self,
        peer_public_key: [u8; X25519_PUBLIC_KEY_LEN],
    ) -> X25519SharedSecret {
        let peer_public = PublicKey::from(peer_public_key);
        X25519SharedSecret {
            bytes: *self.secret.diffie_hellman(&peer_public).as_bytes(),
        }
    }
}

impl core::fmt::Debug for X25519Secret {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("X25519Secret(<redacted>)")
    }
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct X25519SharedSecret {
    bytes: [u8; X25519_SHARED_SECRET_LEN],
}

impl X25519SharedSecret {
    pub fn as_bytes(&self) -> &[u8; X25519_SHARED_SECRET_LEN] {
        &self.bytes
    }

    pub fn expose_for_tests(&self) -> [u8; X25519_SHARED_SECRET_LEN] {
        self.bytes
    }
}

impl core::fmt::Debug for X25519SharedSecret {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("X25519SharedSecret(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SuiteId, TranscriptHash, X25519Secret, decrypt_aes256gcm, decrypt_chacha20poly1305,
        decrypt_chacha20poly1305_in_place, derive_hybrid_traffic_secrets, derive_traffic_secrets,
        encrypt_aes256gcm, encrypt_aes256gcm_in_place, encrypt_chacha20poly1305,
        encrypt_chacha20poly1305_in_place, ml_kem_768_decapsulate, ml_kem_768_encapsulate,
        ml_kem_768_generate_keypair,
    };

    #[test]
    fn derives_deterministic_traffic_secrets() {
        let transcript = TranscriptHash::from_messages(&[b"client", b"server"]);
        let ikm = [7_u8; 32];

        let first = match derive_traffic_secrets(SuiteId(1), &transcript, &ikm) {
            Ok(secrets) => secrets,
            Err(error) => unreachable!("first derivation should work: {error}"),
        };
        let second = match derive_traffic_secrets(SuiteId(1), &transcript, &ikm) {
            Ok(secrets) => secrets,
            Err(error) => unreachable!("second derivation should work: {error}"),
        };

        assert_eq!(
            first.client_to_gateway.expose_for_tests(),
            second.client_to_gateway.expose_for_tests()
        );
        assert_eq!(
            first.gateway_to_client.expose_for_tests(),
            second.gateway_to_client.expose_for_tests()
        );
    }

    #[test]
    fn separates_traffic_directions() {
        let transcript = TranscriptHash::from_messages(&[b"client", b"server"]);
        let ikm = [8_u8; 32];
        let secrets = match derive_traffic_secrets(SuiteId(1), &transcript, &ikm) {
            Ok(secrets) => secrets,
            Err(error) => unreachable!("derivation should work: {error}"),
        };

        assert_ne!(
            secrets.client_to_gateway.expose_for_tests(),
            secrets.gateway_to_client.expose_for_tests()
        );
    }

    #[test]
    fn transcript_change_changes_secret() {
        let first_transcript = TranscriptHash::from_messages(&[b"client-a", b"server"]);
        let second_transcript = TranscriptHash::from_messages(&[b"client-b", b"server"]);
        let ikm = [9_u8; 32];

        let first = match derive_traffic_secrets(SuiteId(1), &first_transcript, &ikm) {
            Ok(secrets) => secrets,
            Err(error) => unreachable!("first derivation should work: {error}"),
        };
        let second = match derive_traffic_secrets(SuiteId(1), &second_transcript, &ikm) {
            Ok(secrets) => secrets,
            Err(error) => unreachable!("second derivation should work: {error}"),
        };

        assert_ne!(
            first.client_to_gateway.expose_for_tests(),
            second.client_to_gateway.expose_for_tests()
        );
    }

    #[test]
    fn rejects_short_input_key_material() {
        let transcript = TranscriptHash::from_messages(&[b"client", b"server"]);
        let result = derive_traffic_secrets(SuiteId(1), &transcript, b"too short");
        assert!(result.is_err());
    }

    #[test]
    fn aead_round_trips_payload() {
        let transcript = TranscriptHash::from_messages(&[b"client", b"server"]);
        let secrets = derive_traffic_secrets(SuiteId(1), &transcript, &[7_u8; 32])
            .unwrap_or_else(|error| unreachable!("derivation should work: {error}"));
        let ciphertext = encrypt_chacha20poly1305(
            &secrets.client_to_gateway,
            1,
            b"frame-header",
            b"secret payload",
        )
        .unwrap_or_else(|error| unreachable!("encrypt should work: {error}"));
        assert_ne!(ciphertext, b"secret payload");
        let plaintext =
            decrypt_chacha20poly1305(&secrets.client_to_gateway, 1, b"frame-header", &ciphertext)
                .unwrap_or_else(|error| unreachable!("decrypt should work: {error}"));
        assert_eq!(plaintext, b"secret payload");
    }

    #[test]
    fn aead_rejects_tampered_payload() {
        let transcript = TranscriptHash::from_messages(&[b"client", b"server"]);
        let secrets = derive_traffic_secrets(SuiteId(1), &transcript, &[7_u8; 32])
            .unwrap_or_else(|error| unreachable!("derivation should work: {error}"));
        let mut ciphertext =
            encrypt_chacha20poly1305(&secrets.client_to_gateway, 2, b"aad", b"payload")
                .unwrap_or_else(|error| unreachable!("encrypt should work: {error}"));
        ciphertext[0] ^= 1;
        let decrypted =
            decrypt_chacha20poly1305(&secrets.client_to_gateway, 2, b"aad", &ciphertext);
        assert!(decrypted.is_err());
    }

    #[test]
    fn chacha20poly1305_in_place_round_trips_payload() {
        let transcript = TranscriptHash::from_messages(&[b"client", b"server"]);
        let secrets = derive_traffic_secrets(SuiteId(1), &transcript, &[7_u8; 32])
            .unwrap_or_else(|error| unreachable!("derivation should work: {error}"));
        let mut buffer = b"secret payload".to_vec();
        encrypt_chacha20poly1305_in_place(&secrets.client_to_gateway, 3, b"aad", &mut buffer)
            .unwrap_or_else(|error| unreachable!("in-place encrypt should work: {error}"));
        assert_ne!(buffer, b"secret payload");
        decrypt_chacha20poly1305_in_place(&secrets.client_to_gateway, 3, b"aad", &mut buffer)
            .unwrap_or_else(|error| unreachable!("in-place decrypt should work: {error}"));
        assert_eq!(buffer, b"secret payload");
    }

    #[test]
    fn aes256gcm_round_trips_payload() {
        let transcript = TranscriptHash::from_messages(&[b"client", b"server"]);
        let secrets = derive_traffic_secrets(SuiteId(1), &transcript, &[9_u8; 32])
            .unwrap_or_else(|error| unreachable!("derivation should work: {error}"));
        let ciphertext = encrypt_aes256gcm(
            &secrets.client_to_gateway,
            7,
            b"frame-header",
            b"secret payload",
        )
        .unwrap_or_else(|error| unreachable!("encrypt should work: {error}"));
        assert_ne!(ciphertext, b"secret payload");
        let plaintext =
            decrypt_aes256gcm(&secrets.client_to_gateway, 7, b"frame-header", &ciphertext)
                .unwrap_or_else(|error| unreachable!("decrypt should work: {error}"));
        assert_eq!(plaintext, b"secret payload");
    }

    #[test]
    fn aes256gcm_rejects_tampered_payload() {
        let transcript = TranscriptHash::from_messages(&[b"client", b"server"]);
        let secrets = derive_traffic_secrets(SuiteId(1), &transcript, &[9_u8; 32])
            .unwrap_or_else(|error| unreachable!("derivation should work: {error}"));
        let mut ciphertext = encrypt_aes256gcm(&secrets.client_to_gateway, 8, b"aad", b"payload")
            .unwrap_or_else(|error| unreachable!("encrypt should work: {error}"));
        ciphertext[0] ^= 1;
        let decrypted = decrypt_aes256gcm(&secrets.client_to_gateway, 8, b"aad", &ciphertext);
        assert!(decrypted.is_err());
    }

    #[test]
    fn aes256gcm_in_place_round_trips_payload() {
        let transcript = TranscriptHash::from_messages(&[b"client", b"server"]);
        let secrets = derive_traffic_secrets(SuiteId(1), &transcript, &[9_u8; 32])
            .unwrap_or_else(|error| unreachable!("derivation should work: {error}"));
        let mut buffer = b"secret payload".to_vec();
        encrypt_aes256gcm_in_place(&secrets.client_to_gateway, 9, b"aad", &mut buffer)
            .unwrap_or_else(|error| unreachable!("in-place encrypt should work: {error}"));
        assert_ne!(buffer, b"secret payload");
        super::decrypt_aes256gcm_in_place(&secrets.client_to_gateway, 9, b"aad", &mut buffer)
            .unwrap_or_else(|error| unreachable!("in-place decrypt should work: {error}"));
        assert_eq!(buffer, b"secret payload");
    }

    #[test]
    fn hybrid_derivation_requires_pq_input_and_changes_secret() {
        let transcript = TranscriptHash::from_messages(&[b"client", b"server"]);
        let classical = [7_u8; 32];
        let pq_a = [8_u8; 32];
        let pq_b = [9_u8; 32];
        let first = derive_hybrid_traffic_secrets(SuiteId(0x0201), &transcript, &classical, &pq_a)
            .unwrap_or_else(|error| unreachable!("hybrid derivation should work: {error}"));
        let second = derive_hybrid_traffic_secrets(SuiteId(0x0201), &transcript, &classical, &pq_b)
            .unwrap_or_else(|error| unreachable!("hybrid derivation should work: {error}"));
        assert_ne!(
            first.client_to_gateway.expose_for_tests(),
            second.client_to_gateway.expose_for_tests()
        );
        assert!(
            derive_hybrid_traffic_secrets(SuiteId(0x0201), &transcript, &classical, b"short")
                .is_err()
        );
    }

    #[test]
    fn ml_kem_768_round_trips_shared_secret() {
        let (decapsulation_key, encapsulation_key) = ml_kem_768_generate_keypair();
        let (ciphertext, sender_secret) = ml_kem_768_encapsulate(&encapsulation_key)
            .unwrap_or_else(|error| unreachable!("encapsulation should work: {error}"));
        let receiver_secret = ml_kem_768_decapsulate(&decapsulation_key, &ciphertext)
            .unwrap_or_else(|error| unreachable!("decapsulation should work: {error}"));
        assert_eq!(sender_secret, receiver_secret);
    }

    #[test]
    fn hybrid_derivation_accepts_real_ml_kem_secret() {
        let (decapsulation_key, encapsulation_key) = ml_kem_768_generate_keypair();
        let (ciphertext, sender_pq_secret) = ml_kem_768_encapsulate(&encapsulation_key)
            .unwrap_or_else(|error| unreachable!("encapsulation should work: {error}"));
        let receiver_pq_secret = ml_kem_768_decapsulate(&decapsulation_key, &ciphertext)
            .unwrap_or_else(|error| unreachable!("decapsulation should work: {error}"));
        let transcript = TranscriptHash::from_messages(&[b"client", b"server"]);
        let sender = derive_hybrid_traffic_secrets(
            SuiteId(0x0201),
            &transcript,
            &[7_u8; 32],
            &sender_pq_secret,
        )
        .unwrap_or_else(|error| unreachable!("sender hybrid derivation should work: {error}"));
        let receiver = derive_hybrid_traffic_secrets(
            SuiteId(0x0201),
            &transcript,
            &[7_u8; 32],
            &receiver_pq_secret,
        )
        .unwrap_or_else(|error| unreachable!("receiver hybrid derivation should work: {error}"));
        assert_eq!(
            sender.client_to_gateway.expose_for_tests(),
            receiver.client_to_gateway.expose_for_tests()
        );
    }

    #[test]
    fn x25519_peers_derive_same_shared_secret() {
        let alice = X25519Secret::from_private_bytes([1_u8; 32]);
        let bob = X25519Secret::from_private_bytes([2_u8; 32]);

        let alice_shared = alice.diffie_hellman(bob.public_key_bytes());
        let bob_shared = bob.diffie_hellman(alice.public_key_bytes());

        assert_eq!(
            alice_shared.expose_for_tests(),
            bob_shared.expose_for_tests()
        );
    }

    #[test]
    fn x25519_debug_redacts_secret_material() {
        let secret = X25519Secret::from_private_bytes([3_u8; 32]);
        let debug = format!("{secret:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains('3'));
    }
}
