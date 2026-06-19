use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::OnceLock;

use crate::peer_egress::options::{AeadSuite, LOCAL_MAGIC, SECURE_MAX_CIPHERTEXT_LEN};
use chimera_crypto::TrafficSecret;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct Destination {
    pub host: String,
    pub port: u16,
}

impl Destination {
    pub fn connect_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn redacted_label(&self) -> String {
        redacted_destination_label(&self.host, self.port)
    }
}

pub fn redacted_destination_label(host: &str, port: u16) -> String {
    let mut hasher = Sha256::new();
    hasher.update(redaction_salt());
    hasher.update(host.as_bytes());
    hasher.update(b":");
    hasher.update(port.to_string().as_bytes());
    let digest = hasher.finalize();
    short_hex(&digest[..8])
}

pub fn redacted_log_reason(error: &str) -> &'static str {
    if error.contains("request") {
        "request_invalid_or_unsupported"
    } else if error.contains("target") {
        "target_connect_failed"
    } else if error.contains("connect") {
        "connect_failed"
    } else {
        "runtime_error"
    }
}

fn redaction_salt() -> &'static [u8; 16] {
    static SALT: OnceLock<[u8; 16]> = OnceLock::new();
    SALT.get_or_init(|| {
        let state = RandomState::new();
        let first = redaction_seed_part(&state, b"chimera-peer-egress-redaction-salt-1");
        let second = redaction_seed_part(&state, b"chimera-peer-egress-redaction-salt-2");
        let mut salt = [0_u8; 16];
        salt[..8].copy_from_slice(&first.to_be_bytes());
        salt[8..].copy_from_slice(&second.to_be_bytes());
        salt
    })
}

fn redaction_seed_part(state: &RandomState, label: &[u8]) -> u64 {
    let mut hasher = state.build_hasher();
    hasher.write(label);
    hasher.finish()
}

fn short_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[derive(Debug)]
pub struct SecurePeerStream {
    pub stream: TcpStream,
    pub send_secret: TrafficSecret,
    pub recv_secret: TrafficSecret,
    pub send_packet: u64,
    pub recv_packet: u64,
    pub aead: AeadSuite,
}

impl SecurePeerStream {
    pub fn write_line(&mut self, line: &str) -> Result<(), String> {
        let mut bytes = line.as_bytes().to_vec();
        bytes.push(b'\n');
        self.write_secure_payload(&bytes)
    }

    pub fn read_line(&mut self, max_len: usize) -> Result<String, String> {
        let payload = self.read_secure_payload()?;
        if payload.len() > max_len {
            return Err("secure line too long".to_string());
        }
        if payload.last() != Some(&b'\n') {
            return Err("secure line missing newline".to_string());
        }
        String::from_utf8(payload[..payload.len() - 1].to_vec())
            .map_err(|_| "secure line is not utf-8".to_string())
    }

    pub fn write_secure_payload(&mut self, plaintext: &[u8]) -> Result<(), String> {
        let packet = self.send_packet;
        self.send_packet = self
            .send_packet
            .checked_add(1)
            .ok_or_else(|| "secure send packet counter exhausted".to_string())?;
        let mut ciphertext = Vec::with_capacity(plaintext.len() + 16);
        ciphertext.extend_from_slice(plaintext);
        encrypt_secure_payload_in_place(
            self.aead,
            &self.send_secret,
            packet,
            b"peer-egress",
            &mut ciphertext,
        )
        .map_err(|error| format!("secure encrypt failed: {error}"))?;
        let len = u32::try_from(ciphertext.len())
            .map_err(|_| "secure ciphertext length overflow".to_string())?;
        self.stream
            .write_all(&packet.to_be_bytes())
            .and_then(|_| self.stream.write_all(&len.to_be_bytes()))
            .and_then(|_| self.stream.write_all(&ciphertext))
            .map_err(|error| format!("write secure frame failed: {error}"))
    }

    pub fn read_secure_payload(&mut self) -> Result<Vec<u8>, String> {
        let mut header = [0_u8; 12];
        self.stream
            .read_exact(&mut header)
            .map_err(|error| format!("read secure frame header failed: {error}"))?;
        let packet = u64::from_be_bytes(
            header[0..8]
                .try_into()
                .map_err(|_| "invalid secure packet field".to_string())?,
        );
        if packet != self.recv_packet {
            return Err("secure packet number mismatch".to_string());
        }
        self.recv_packet = self
            .recv_packet
            .checked_add(1)
            .ok_or_else(|| "secure receive packet counter exhausted".to_string())?;
        let len = u32::from_be_bytes(
            header[8..12]
                .try_into()
                .map_err(|_| "invalid secure length field".to_string())?,
        ) as usize;
        if len == 0 || len > SECURE_MAX_CIPHERTEXT_LEN {
            return Err("secure ciphertext length invalid".to_string());
        }
        let mut ciphertext = vec![0_u8; len];
        self.stream
            .read_exact(&mut ciphertext)
            .map_err(|error| format!("read secure frame payload failed: {error}"))?;
        decrypt_secure_payload_in_place(
            self.aead,
            &self.recv_secret,
            packet,
            b"peer-egress",
            &mut ciphertext,
        )
        .map_err(|error| format!("secure decrypt failed: {error}"))?;
        Ok(ciphertext)
    }
}

pub(crate) fn encrypt_secure_payload_in_place(
    aead: AeadSuite,
    secret: &TrafficSecret,
    packet: u64,
    associated_data: &[u8],
    buffer: &mut Vec<u8>,
) -> chimera_core::ChimeraResult<()> {
    match aead {
        AeadSuite::Chacha20Poly1305 => chimera_crypto::encrypt_chacha20poly1305_in_place(
            secret,
            packet,
            associated_data,
            buffer,
        ),
        AeadSuite::Aes256Gcm => {
            chimera_crypto::encrypt_aes256gcm_in_place(secret, packet, associated_data, buffer)
        }
    }
}

pub(crate) fn decrypt_secure_payload_in_place(
    aead: AeadSuite,
    secret: &TrafficSecret,
    packet: u64,
    associated_data: &[u8],
    buffer: &mut Vec<u8>,
) -> chimera_core::ChimeraResult<()> {
    match aead {
        AeadSuite::Chacha20Poly1305 => chimera_crypto::decrypt_chacha20poly1305_in_place(
            secret,
            packet,
            associated_data,
            buffer,
        ),
        AeadSuite::Aes256Gcm => {
            chimera_crypto::decrypt_aes256gcm_in_place(secret, packet, associated_data, buffer)
        }
    }
}

pub fn read_native_connect_destination(
    stream: &mut TcpStream,
    first_byte: u8,
) -> Result<Destination, String> {
    let mut rest = vec![0_u8; LOCAL_MAGIC.len() - 1];
    stream
        .read_exact(&mut rest)
        .map_err(|error| format!("read native local magic failed: {error}"))?;
    let mut magic = vec![first_byte];
    magic.extend(rest);
    if magic != LOCAL_MAGIC {
        return Err("bad native local magic".to_string());
    }
    let request = read_line_limited(stream, 512)?;
    let mut parts = request.split_whitespace();
    if parts.next() != Some("CONNECT") {
        return Err("native local request must be CONNECT".to_string());
    }
    let host = parts
        .next()
        .ok_or_else(|| "native local request missing host".to_string())?;
    let port = parts
        .next()
        .ok_or_else(|| "native local request missing port".to_string())?
        .parse::<u16>()
        .map_err(|_| "native local request has invalid port".to_string())?;
    if parts.next().is_some() || host.is_empty() || host.contains('\r') || host.contains('\n') {
        return Err("native local request is invalid".to_string());
    }
    Ok(Destination {
        host: host.to_string(),
        port,
    })
}

pub fn parse_peer_connect_destination(line: &str) -> Result<Destination, String> {
    let mut parts = line.split_whitespace();
    let Some(kind) = parts.next() else {
        return Err("empty peer request".to_string());
    };
    if kind != "CONNECT" {
        return Err("unsupported peer request".to_string());
    }
    let host = parts
        .next()
        .ok_or_else(|| "peer request missing host".to_string())?;
    let port = parts
        .next()
        .ok_or_else(|| "peer request missing port".to_string())?
        .parse::<u16>()
        .map_err(|_| "peer request has invalid port".to_string())?;
    if parts.next().is_some() {
        return Err("peer request has trailing fields".to_string());
    }
    if host.is_empty() || host.contains('\n') || host.contains('\r') {
        return Err("peer request has invalid host".to_string());
    }
    Ok(Destination {
        host: host.to_string(),
        port,
    })
}

pub fn parse_peer_connect_request(line: &str) -> Result<String, String> {
    parse_peer_connect_destination(line).map(|destination| destination.connect_addr())
}

pub fn read_line_limited(stream: &mut TcpStream, max_len: usize) -> Result<String, String> {
    let mut out = Vec::new();
    let mut buf = [0_u8; 1];
    while out.len() <= max_len {
        stream
            .read_exact(&mut buf)
            .map_err(|error| format!("read line failed: {error}"))?;
        if buf[0] == b'\n' {
            return String::from_utf8(out).map_err(|_| "line is not utf-8".to_string());
        }
        out.push(buf[0]);
    }
    Err("line too long".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_peer_connect_request_accepts_host_port() {
        let target = parse_peer_connect_request("CONNECT example.org 443")
            .unwrap_or_else(|error| unreachable!("request must parse: {error}"));
        assert_eq!(target, "example.org:443");
    }

    #[test]
    fn native_local_request_rejects_bad_shape() {
        let request = parse_peer_connect_request("GET example.org 443");
        assert!(request.is_err());
    }

    #[test]
    fn redacted_destination_label_is_stable_for_same_destination() {
        let label1 = redacted_destination_label("example.org", 443);
        let label2 = redacted_destination_label("example.org", 443);
        assert_eq!(label1, label2);
        assert_eq!(label1.len(), 16);
    }

    #[test]
    fn redacted_destination_label_does_not_expose_raw_destination() {
        let label = redacted_destination_label("example.org", 443);
        assert!(!label.contains("example"));
        assert!(!label.contains("443"));
        assert_ne!(label, "example.org:443");
    }

    #[test]
    fn redacted_log_reason_maps_known_error_classes() {
        assert_eq!(
            redacted_log_reason("request missing host"),
            "request_invalid_or_unsupported"
        );
        assert_eq!(
            redacted_log_reason("target connect failed"),
            "target_connect_failed"
        );
        assert_eq!(redacted_log_reason("connect timeout"), "connect_failed");
        assert_eq!(redacted_log_reason("something else"), "runtime_error");
    }

    #[test]
    fn parse_peer_connect_destination_preserves_parts_without_logging_shape() {
        let destination = parse_peer_connect_destination("CONNECT example.org 443")
            .unwrap_or_else(|error| unreachable!("request must parse: {error}"));
        assert_eq!(destination.host, "example.org");
        assert_eq!(destination.port, 443);
    }

    #[test]
    fn write_secure_payload_rejects_packet_counter_overflow() {
        let transcript =
            chimera_crypto::TranscriptHash::from_messages(&[b"peer-egress-overflow-test"]);
        let secrets = chimera_crypto::derive_traffic_secrets(
            chimera_crypto::SuiteId(
                crate::peer_egress::options::AeadSuite::Chacha20Poly1305.suite_id(),
            ),
            &transcript,
            &[7_u8; 32],
        )
        .unwrap_or_else(|error| unreachable!("test secrets must derive: {error}"));
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap_or_else(|error| unreachable!("listener bind failed: {error}"));
        let addr = listener
            .local_addr()
            .unwrap_or_else(|error| unreachable!("listener addr failed: {error}"));
        let client = std::net::TcpStream::connect(addr)
            .unwrap_or_else(|error| unreachable!("client connect failed: {error}"));
        let (server, _) = listener
            .accept()
            .unwrap_or_else(|error| unreachable!("server accept failed: {error}"));
        drop(server);

        let mut peer = SecurePeerStream {
            stream: client,
            send_secret: secrets.initiator_to_responder().clone(),
            recv_secret: secrets.responder_to_initiator().clone(),
            send_packet: u64::MAX,
            recv_packet: 0,
            aead: crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
        };

        let error = match peer.write_secure_payload(b"payload") {
            Ok(()) => unreachable!("packet counter overflow must fail"),
            Err(error) => error,
        };
        assert!(error.contains("counter exhausted"));
    }

    #[test]
    fn read_secure_payload_rejects_packet_counter_overflow() {
        use std::io::Write;

        let transcript =
            chimera_crypto::TranscriptHash::from_messages(&[b"peer-egress-read-overflow-test"]);
        let secrets = chimera_crypto::derive_traffic_secrets(
            chimera_crypto::SuiteId(
                crate::peer_egress::options::AeadSuite::Chacha20Poly1305.suite_id(),
            ),
            &transcript,
            &[13_u8; 32],
        )
        .unwrap_or_else(|error| unreachable!("test secrets must derive: {error}"));
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap_or_else(|error| unreachable!("listener bind failed: {error}"));
        let addr = listener
            .local_addr()
            .unwrap_or_else(|error| unreachable!("listener addr failed: {error}"));
        let client = std::net::TcpStream::connect(addr)
            .unwrap_or_else(|error| unreachable!("client connect failed: {error}"));
        let (server, _) = listener
            .accept()
            .unwrap_or_else(|error| unreachable!("server accept failed: {error}"));

        let mut writer = client;
        let mut reader = SecurePeerStream {
            stream: server,
            send_secret: secrets.responder_to_initiator().clone(),
            recv_secret: secrets.initiator_to_responder().clone(),
            send_packet: 0,
            recv_packet: u64::MAX,
            aead: crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
        };

        let mut ciphertext = b"payload".to_vec();
        encrypt_secure_payload_in_place(
            crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
            secrets.initiator_to_responder(),
            u64::MAX,
            b"peer-egress",
            &mut ciphertext,
        )
        .unwrap_or_else(|error| unreachable!("test payload must encrypt: {error}"));
        let len = u32::try_from(ciphertext.len())
            .unwrap_or_else(|error| unreachable!("test ciphertext length must fit: {error}"));
        let writer_thread = std::thread::spawn(move || {
            writer
                .write_all(&u64::MAX.to_be_bytes())
                .and_then(|_| writer.write_all(&len.to_be_bytes()))
                .and_then(|_| writer.write_all(&ciphertext))
        });
        let error = match reader.read_secure_payload() {
            Ok(_) => unreachable!("receive packet counter overflow must fail"),
            Err(error) => error,
        };
        writer_thread
            .join()
            .unwrap_or_else(|_| unreachable!("writer thread must not panic"))
            .unwrap_or_else(|error| unreachable!("writer must write frame: {error}"));
        assert!(error.contains("counter exhausted"));
    }
}
