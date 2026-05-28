use std::io::{Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, TcpStream};

use crate::peer_egress::options::{
    AeadSuite, LOCAL_MAGIC, SECURE_MAX_CIPHERTEXT_LEN,
};
use chimera_crypto::TrafficSecret;

#[derive(Debug, Clone)]
pub struct Destination {
    pub host: String,
    pub port: u16,
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
        self.send_packet = self.send_packet.saturating_add(1);
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
        self.recv_packet = self.recv_packet.saturating_add(1);
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

fn encrypt_secure_payload_in_place(
    aead: AeadSuite,
    secret: &TrafficSecret,
    packet: u64,
    associated_data: &[u8],
    buffer: &mut Vec<u8>,
) -> chimera_core::ChimeraResult<()> {
    match aead {
        AeadSuite::Chacha20Poly1305 => {
            chimera_crypto::encrypt_chacha20poly1305_in_place(secret, packet, associated_data, buffer)
        }
        AeadSuite::Aes256Gcm => {
            chimera_crypto::encrypt_aes256gcm_in_place(secret, packet, associated_data, buffer)
        }
    }
}

fn decrypt_secure_payload_in_place(
    aead: AeadSuite,
    secret: &TrafficSecret,
    packet: u64,
    associated_data: &[u8],
    buffer: &mut Vec<u8>,
) -> chimera_core::ChimeraResult<()> {
    match aead {
        AeadSuite::Chacha20Poly1305 => {
            chimera_crypto::decrypt_chacha20poly1305_in_place(secret, packet, associated_data, buffer)
        }
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

pub fn read_socks5_connect_destination(
    stream: &mut TcpStream,
    first_byte: u8,
) -> Result<Destination, String> {
    let mut greeting_tail = [0_u8; 1];
    stream
        .read_exact(&mut greeting_tail)
        .map_err(|error| format!("read socks greeting failed: {error}"))?;
    if first_byte != 5 {
        return Err("unsupported socks version".to_string());
    }
    let methods_len = greeting_tail[0] as usize;
    let mut methods = vec![0_u8; methods_len];
    stream
        .read_exact(&mut methods)
        .map_err(|error| format!("read socks methods failed: {error}"))?;
    stream
        .write_all(&[5, 0])
        .map_err(|error| format!("write socks method response failed: {error}"))?;

    let mut head = [0_u8; 4];
    stream
        .read_exact(&mut head)
        .map_err(|error| format!("read socks connect head failed: {error}"))?;
    if head[0] != 5 || head[1] != 1 {
        return Err("only socks5 CONNECT is supported".to_string());
    }
    let host = match head[3] {
        1 => {
            let mut octets = [0_u8; 4];
            stream
                .read_exact(&mut octets)
                .map_err(|error| format!("read ipv4 target failed: {error}"))?;
            Ipv4Addr::from(octets).to_string()
        }
        3 => {
            let mut len = [0_u8; 1];
            stream
                .read_exact(&mut len)
                .map_err(|error| format!("read domain length failed: {error}"))?;
            let mut raw = vec![0_u8; len[0] as usize];
            stream
                .read_exact(&mut raw)
                .map_err(|error| format!("read domain target failed: {error}"))?;
            String::from_utf8(raw).map_err(|_| "domain target is not utf-8".to_string())?
        }
        4 => {
            let mut octets = [0_u8; 16];
            stream
                .read_exact(&mut octets)
                .map_err(|error| format!("read ipv6 target failed: {error}"))?;
            Ipv6Addr::from(octets).to_string()
        }
        _ => return Err("unsupported socks address type".to_string()),
    };
    let mut port_raw = [0_u8; 2];
    stream
        .read_exact(&mut port_raw)
        .map_err(|error| format!("read socks target port failed: {error}"))?;
    let port = u16::from_be_bytes(port_raw);
    Ok(Destination { host, port })
}

pub fn write_socks5_success(stream: &mut TcpStream) -> Result<(), String> {
    stream
        .write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0])
        .map_err(|error| format!("write socks success failed: {error}"))
}

pub fn parse_peer_connect_request(line: &str) -> Result<String, String> {
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
    Ok(format!("{host}:{port}"))
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
}
