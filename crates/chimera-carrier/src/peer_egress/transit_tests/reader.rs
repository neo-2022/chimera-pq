use std::io;
use std::io::{Cursor, Read};

use chimera_session::FrameKind;

use super::super::{
    forward_transit_relay_frame, read_weave_bound_sealed_transit_frame,
    read_weave_sealed_transit_frame, validate_transit_relay_frame,
};
use super::helpers::{binding, encoded_frame};

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

#[test]
fn bound_transit_relay_reader_preserves_binding_and_sealed_bytes_without_payload_leak()
-> Result<(), String> {
    let encoded = encoded_frame(FrameKind::Data, 100, b"bound opaque bytes");
    let bound = crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(
        binding(700, 2),
        validate_transit_relay_frame(&encoded)?,
    );
    let encoded_bound =
        crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&bound);
    let mut cursor = Cursor::new(encoded_bound);
    let first = {
        let mut byte = [0_u8; 1];
        cursor
            .read_exact(&mut byte)
            .map_err(|error| error.to_string())?;
        byte[0]
    };
    let parsed = read_weave_bound_sealed_transit_frame(&mut cursor, first)?;
    let debug = format!("{parsed:?}");
    assert_eq!(parsed.binding(), binding(700, 2));
    assert_eq!(parsed.frame().sealed_bytes(), encoded.as_slice());
    assert!(debug.contains("<opaque>"));
    assert!(debug.contains("<sealed>"));
    assert!(!debug.contains("bound opaque bytes"));
    Ok(())
}

#[test]
fn bound_transit_relay_reader_rejects_text_without_fallback() {
    let mut cursor = Cursor::new(b"OK\n".to_vec());
    let result = read_weave_bound_sealed_transit_frame(
        &mut cursor,
        crate::peer_egress::transit_binding::BOUND_TRANSIT_MAGIC,
    );
    assert!(result.is_err());
}

#[test]
fn transit_relay_reader_rejects_bad_version_without_reading_payload() {
    struct CountingReader {
        reads: usize,
    }

    impl Read for CountingReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            self.reads += 1;
            Ok(0)
        }
    }

    let mut reader = CountingReader { reads: 0 };
    let result = read_weave_sealed_transit_frame(&mut reader, 0xff);
    assert!(result.is_err_and(|error| error.contains("version invalid")));
    assert_eq!(reader.reads, 0);
}

#[test]
fn transit_relay_reader_rejects_declared_payload_too_large_without_reading_payload() {
    struct HeaderOnlyReader {
        data: Vec<u8>,
        reads: usize,
    }

    impl Read for HeaderOnlyReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.reads += 1;
            if self.data.is_empty() {
                return Ok(0);
            }
            let n = buf.len().min(self.data.len());
            buf[..n].copy_from_slice(&self.data[..n]);
            self.data.drain(..n);
            Ok(n)
        }
    }

    let declared = (chimera_session::MAX_PAYLOAD_LEN as u32 + 1).to_be_bytes();
    let mut header = vec![FrameKind::Data as u8];
    header.extend_from_slice(&7_u64.to_be_bytes());
    header.extend_from_slice(&declared);
    let mut reader = HeaderOnlyReader {
        data: header,
        reads: 0,
    };
    let result = read_weave_sealed_transit_frame(&mut reader, chimera_session::FRAME_VERSION);
    assert!(result.is_err_and(|error| error.contains("payload too large")));
    assert_eq!(reader.reads, 1);
}
