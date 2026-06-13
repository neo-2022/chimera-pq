use std::io::{Read, Write};
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

impl SecurePeerReader {
    pub(crate) fn read_secure_payload(&mut self) -> Result<Vec<u8>, String> {
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
        crate::peer_egress::protocol::decrypt_secure_payload_in_place(
            self.aead,
            &self.recv_secret,
            packet,
            b"peer-egress",
            &mut ciphertext,
        )
        .map_err(|error| format!("secure decrypt failed: {error}"))?;
        Ok(ciphertext)
    }

    pub(crate) fn shutdown(&self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
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
