use std::io::Read;
use std::net::{Shutdown, TcpStream};

use chimera_mesh::{
    WeaveSealedTransitFrame, forward_weave_transit_frame, validate_weave_sealed_transit_frame,
};

use crate::peer_egress::net::tune_tcp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitRelayFrame {
    frame: WeaveSealedTransitFrame,
}

impl TransitRelayFrame {
    pub fn kind(&self) -> chimera_session::FrameKind {
        self.frame.kind()
    }

    pub fn packet_number(&self) -> u64 {
        self.frame.packet_number()
    }

    pub fn payload_len(&self) -> usize {
        self.frame.payload_len()
    }

    pub fn sealed_bytes(&self) -> &[u8] {
        self.frame.sealed_bytes()
    }
}

pub fn validate_transit_relay_frame(input: &[u8]) -> Result<TransitRelayFrame, String> {
    validate_weave_sealed_transit_frame(input)
        .map(|frame| TransitRelayFrame { frame })
        .map_err(|error| format!("validate transit frame failed: {error}"))
}

pub fn forward_transit_relay_frame(input: &[u8]) -> Result<Vec<u8>, String> {
    forward_weave_transit_frame(input)
        .map_err(|error| format!("forward transit frame failed: {error}"))
}

pub fn relay_local_sealed_transit(
    mut local: TcpStream,
    mut peer: crate::peer_egress::protocol::SecurePeerStream,
    first_byte: u8,
) -> Result<(), String> {
    tune_tcp(&local)?;
    tune_tcp(&peer.stream)?;
    let mut next_first = Some(first_byte);
    loop {
        let byte = match next_first.take() {
            Some(byte) => byte,
            None => unreachable!("missing transit frame prefix"),
        };
        let transit = read_weave_sealed_transit_frame(&mut local, byte)?;
        eprintln!(
            "event=weave_transit_frame_forwarded kind={:?} packet_number={} payload_len={}",
            transit.kind(),
            transit.packet_number(),
            transit.payload_len()
        );
        peer.write_secure_payload(transit.sealed_bytes())
            .map_err(|error| format!("write transit frame to peer failed: {error}"))?;
        let mut first = [0_u8; 1];
        match local.read(&mut first) {
            Ok(0) => {
                let _ = peer.stream.shutdown(Shutdown::Write);
                return Ok(());
            }
            Ok(1) => next_first = Some(first[0]),
            Ok(_) => unreachable!("single-byte read returned more than one byte"),
            Err(error) => return Err(format!("read transit frame prefix failed: {error}")),
        }
    }
}

pub fn read_weave_sealed_transit_frame<R: Read>(
    stream: &mut R,
    first_byte: u8,
) -> Result<TransitRelayFrame, String> {
    let mut header = [0_u8; 13];
    stream
        .read_exact(&mut header)
        .map_err(|error| format!("read transit frame header failed: {error}"))?;
    let payload_len = u32::from_be_bytes(
        header[9..13]
            .try_into()
            .map_err(|_| "invalid transit frame length field".to_string())?,
    ) as usize;
    if payload_len > chimera_session::MAX_PAYLOAD_LEN {
        return Err("transit frame payload too large".to_string());
    }
    let mut frame = Vec::with_capacity(14 + payload_len);
    frame.push(first_byte);
    frame.extend_from_slice(&header);
    let mut payload = vec![0_u8; payload_len];
    if payload_len > 0 {
        stream
            .read_exact(&mut payload)
            .map_err(|error| format!("read transit frame payload failed: {error}"))?;
    }
    frame.extend_from_slice(&payload);
    validate_transit_relay_frame(&frame)
}

#[cfg(test)]
mod tests {
    use super::{
        forward_transit_relay_frame, read_weave_sealed_transit_frame, validate_transit_relay_frame,
    };
    use chimera_session::{Frame, FrameKind};
    use std::io::{Cursor, Read};

    fn encoded_frame(kind: FrameKind, packet_number: u64, payload: &[u8]) -> Vec<u8> {
        match (Frame {
            kind,
            packet_number,
            payload: payload.to_vec(),
        })
        .encode()
        {
            Ok(encoded) => encoded,
            Err(error) => unreachable!("frame must encode: {error}"),
        }
    }

    #[test]
    fn transit_relay_frame_forwards_unchanged() -> Result<(), String> {
        let encoded = encoded_frame(FrameKind::Data, 42, b"third-party payload");
        let frame = validate_transit_relay_frame(&encoded)?;
        assert_eq!(frame.kind(), FrameKind::Data);
        assert_eq!(frame.packet_number(), 42);
        assert_eq!(frame.payload_len(), "third-party payload".len());
        assert_eq!(frame.sealed_bytes(), encoded.as_slice());
        let forwarded = forward_transit_relay_frame(&encoded)?;
        assert_eq!(forwarded, encoded);
        Ok(())
    }

    #[test]
    fn transit_relay_frame_rejects_truncated_input() {
        let encoded = encoded_frame(FrameKind::Fin, 7, b"opaque");
        let truncated = &encoded[..encoded.len() - 1];
        let result = validate_transit_relay_frame(truncated);
        assert!(result.is_err());
        assert!(forward_transit_relay_frame(truncated).is_err());
    }

    #[test]
    fn transit_relay_frame_debug_redacts_bytes() -> Result<(), String> {
        let encoded = encoded_frame(FrameKind::Fin, 11, b"closed payload");
        let frame = validate_transit_relay_frame(&encoded)?;
        let debug = format!("{frame:?}");
        assert!(debug.contains("<sealed>"));
        assert!(!debug.contains("closed payload"));
        Ok(())
    }

    #[test]
    fn transit_relay_reader_preserves_metadata_without_payload_leak() -> Result<(), String> {
        let encoded = encoded_frame(FrameKind::Data, 99, b"opaque bytes");
        let mut cursor = Cursor::new(encoded.clone());
        let first = {
            let mut byte = [0_u8; 1];
            cursor
                .read_exact(&mut byte)
                .map_err(|error| error.to_string())?;
            byte[0]
        };
        let frame = read_weave_sealed_transit_frame(&mut cursor, first)?;
        let debug = format!("{frame:?}");
        assert_eq!(frame.sealed_bytes(), encoded.as_slice());
        assert!(debug.contains("<sealed>"));
        assert!(!debug.contains("opaque bytes"));
        Ok(())
    }
}
