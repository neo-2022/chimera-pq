use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};

use chimera_crypto::TrafficSecret;

use crate::peer_egress::options::{AeadSuite, SECURE_MAX_CIPHERTEXT_LEN};
use crate::peer_egress::protocol::SecurePeerStream;

pub(crate) struct SecurePeerReader {
    stream: TcpStream,
    recv_secret: TrafficSecret,
    recv_packet: u64,
    aead: AeadSuite,
}

pub(crate) enum SecurePayloadRead {
    Payload(Vec<u8>),
    Idle,
}

impl SecurePeerReader {
    pub(crate) fn read_secure_payload_or_idle(&mut self) -> Result<SecurePayloadRead, String> {
        let mut header = [0_u8; 12];
        if !read_exact_or_idle(&mut self.stream, &mut header, "read secure frame header")? {
            return Ok(SecurePayloadRead::Idle);
        }
        let packet = u64::from_be_bytes(
            header[0..8]
                .try_into()
                .map_err(|_| "invalid secure packet field".to_string())?,
        );
        if packet != self.recv_packet {
            return Err("secure packet number mismatch".to_string());
        }
        let next_recv_packet = self
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
        read_exact_mid_frame(
            &mut self.stream,
            &mut ciphertext,
            "read secure frame payload",
        )?;
        crate::peer_egress::protocol::decrypt_secure_payload_in_place(
            self.aead,
            &self.recv_secret,
            packet,
            b"peer-egress",
            &mut ciphertext,
        )
        .map_err(|error| format!("secure decrypt failed: {error}"))?;
        self.recv_packet = next_recv_packet;
        Ok(SecurePayloadRead::Payload(ciphertext))
    }

    pub(crate) fn shutdown(&self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

fn read_exact_or_idle(
    stream: &mut TcpStream,
    buffer: &mut [u8],
    context: &str,
) -> Result<bool, String> {
    let mut offset = 0usize;
    while offset < buffer.len() {
        match stream.read(&mut buffer[offset..]) {
            Ok(0) => return Err(format!("{context} failed: unexpected eof")),
            Ok(read) => offset += read,
            Err(error) if is_idle_timeout(error.kind()) && offset == 0 => return Ok(false),
            Err(error) if is_idle_timeout(error.kind()) => {
                return Err(format!("{context} timed out mid-frame"));
            }
            Err(error) => return Err(format!("{context} failed: {error}")),
        }
    }
    Ok(true)
}

fn read_exact_mid_frame(
    stream: &mut TcpStream,
    buffer: &mut [u8],
    context: &str,
) -> Result<(), String> {
    let mut offset = 0usize;
    while offset < buffer.len() {
        match stream.read(&mut buffer[offset..]) {
            Ok(0) => return Err(format!("{context} failed: unexpected eof")),
            Ok(read) => offset += read,
            Err(error) if is_idle_timeout(error.kind()) => {
                return Err(format!("{context} timed out mid-frame"));
            }
            Err(error) => return Err(format!("{context} failed: {error}")),
        }
    }
    Ok(())
}

fn is_idle_timeout(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

pub(crate) struct SecurePeerWriter {
    pub(crate) stream: TcpStream,
    send_secret: TrafficSecret,
    send_packet: u64,
    aead: AeadSuite,
}

impl SecurePeerWriter {
    pub(crate) fn write_secure_payload(&mut self, plaintext: &[u8]) -> Result<(), String> {
        let packet = self.send_packet;
        self.send_packet = self
            .send_packet
            .checked_add(1)
            .ok_or_else(|| "secure send packet counter exhausted".to_string())?;
        let mut ciphertext = Vec::with_capacity(plaintext.len() + 16);
        ciphertext.extend_from_slice(plaintext);
        crate::peer_egress::protocol::encrypt_secure_payload_in_place(
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

    pub(crate) fn shutdown(&self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

pub(crate) fn split_secure_peer_stream(
    peer: SecurePeerStream,
) -> Result<(SecurePeerReader, SecurePeerWriter), String> {
    let reader = SecurePeerReader {
        stream: peer
            .stream
            .try_clone()
            .map_err(|error| format!("clone secure peer stream failed: {error}"))?,
        recv_secret: peer.recv_secret.clone(),
        recv_packet: peer.recv_packet,
        aead: peer.aead,
    };
    let writer = SecurePeerWriter {
        stream: peer.stream,
        send_secret: peer.send_secret,
        send_packet: peer.send_packet,
        aead: peer.aead,
    };
    Ok((reader, writer))
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::net::TcpListener;
    use std::time::Duration;

    use chimera_crypto::{SuiteId, TranscriptHash, derive_traffic_secrets};

    use super::{SecurePayloadRead, SecurePeerReader};
    use crate::peer_egress::options::AeadSuite;

    fn test_reader() -> Result<(std::net::TcpStream, SecurePeerReader), String> {
        let transcript = TranscriptHash::from_messages(&[b"secure-halves-idle-test"]);
        let secrets = derive_traffic_secrets(
            SuiteId(AeadSuite::Chacha20Poly1305.suite_id()),
            &transcript,
            &[17_u8; 32],
        )
        .map_err(|error| format!("derive test secrets failed: {error}"))?;
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("bind test listener failed: {error}"))?;
        let addr = listener
            .local_addr()
            .map_err(|error| format!("resolve test listener addr failed: {error}"))?;
        let writer = std::net::TcpStream::connect(addr)
            .map_err(|error| format!("connect test writer failed: {error}"))?;
        let (reader_stream, _) = listener
            .accept()
            .map_err(|error| format!("accept test reader failed: {error}"))?;
        reader_stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .map_err(|error| format!("set reader timeout failed: {error}"))?;
        Ok((
            writer,
            SecurePeerReader {
                stream: reader_stream,
                recv_secret: secrets.initiator_to_responder().clone(),
                recv_packet: 0,
                aead: AeadSuite::Chacha20Poly1305,
            },
        ))
    }

    #[test]
    fn secure_reader_idle_before_header_is_idle() -> Result<(), String> {
        let (_writer, mut reader) = test_reader()?;

        match reader.read_secure_payload_or_idle()? {
            SecurePayloadRead::Idle => Ok(()),
            SecurePayloadRead::Payload(_) => {
                Err("idle reader must not produce secure payload".to_string())
            }
        }
    }

    #[test]
    fn secure_reader_timeout_after_header_is_mid_frame_error() -> Result<(), String> {
        let (mut writer, mut reader) = test_reader()?;
        let packet = 0_u64;
        let len = 32_u32;
        writer
            .write_all(&packet.to_be_bytes())
            .and_then(|_| writer.write_all(&len.to_be_bytes()))
            .map_err(|error| format!("write partial secure header failed: {error}"))?;

        let error = match reader.read_secure_payload_or_idle() {
            Ok(_) => return Err("payload timeout after header must fail closed".to_string()),
            Err(error) => error,
        };

        assert!(error.contains("timed out mid-frame"), "{error}");
        assert_eq!(reader.recv_packet, 0);
        Ok(())
    }
}
