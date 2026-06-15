use std::io::Read;
use std::net::{Shutdown, TcpStream};
use std::thread;

use chimera_mesh::{
    WeaveSealedTransitFrame, forward_weave_transit_frame, validate_weave_sealed_transit_frame,
};

use crate::peer_egress::net::tune_tcp;
use crate::peer_egress::pool::{SharedPeerPool, UniquePeerPop};
use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::secure_halves::{
    SecurePeerReader, SecurePeerWriter, split_secure_peer_stream,
};
use crate::peer_egress::transit_binding::{BoundTransitRelayFrame, TransitPathBinding};
use crate::peer_egress::transit_dispatch::SharedTransitNextHopDispatcher;
use crate::peer_egress::wire::{
    PeerMessage, write_bound_sealed_transit_message, write_sealed_transit_message,
};

const TRANSIT_FRAME_HEADER_REST_LEN: usize = 13;
const TRANSIT_FRAME_HEADER_LEN: usize = 1 + TRANSIT_FRAME_HEADER_REST_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerTransitPolicy {
    DenyPoolNextHop,
    AllowPoolNextHop,
}

impl PeerTransitPolicy {
    pub fn from_bool(allowed: bool) -> Self {
        if allowed {
            Self::AllowPoolNextHop
        } else {
            Self::DenyPoolNextHop
        }
    }
}

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
    mut peer: SecurePeerStream,
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
        eprintln!("event=weave_transit_frame_forwarded");
        write_sealed_transit_message(&mut peer, &transit)
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

pub fn relay_local_bound_sealed_transit(
    mut local: TcpStream,
    peer: SecurePeerStream,
    first_byte: u8,
) -> Result<(), String> {
    tune_tcp(&local)?;
    tune_tcp(&peer.stream)?;
    let first = read_weave_bound_sealed_transit_frame(&mut local, first_byte)?;
    relay_local_bound_sealed_transit_after_first(local, peer, first)
}

pub fn relay_local_bound_sealed_transit_to_next_hop(
    mut local: TcpStream,
    peer_pool: SharedPeerPool,
    first_byte: u8,
) -> Result<(), String> {
    tune_tcp(&local)?;
    let first = read_weave_bound_sealed_transit_frame(&mut local, first_byte)?;
    let peer = pop_bound_transit_lane_next_hop(&peer_pool, first.binding())?;
    relay_local_bound_sealed_transit_after_first(local, peer, first)
}

fn relay_local_bound_sealed_transit_after_first(
    mut local: TcpStream,
    mut peer: SecurePeerStream,
    first: BoundTransitRelayFrame,
) -> Result<(), String> {
    tune_tcp(&peer.stream)?;
    let binding = first.binding();
    let mut pending = Some(first);
    loop {
        let transit = match pending.take() {
            Some(frame) => frame,
            None => {
                let mut first = [0_u8; 1];
                match local.read(&mut first) {
                    Ok(0) => {
                        let _ = peer.stream.shutdown(Shutdown::Write);
                        return Ok(());
                    }
                    Ok(1) => read_weave_bound_sealed_transit_frame(&mut local, first[0])?,
                    Ok(_) => unreachable!("single-byte read returned more than one byte"),
                    Err(error) => {
                        return Err(format!("read bound transit frame prefix failed: {error}"));
                    }
                }
            }
        };
        if transit.binding() != binding {
            let _ = peer.stream.shutdown(Shutdown::Write);
            return Err("bound transit stream binding changed midstream".to_string());
        }
        eprintln!("event=weave_bound_transit_frame_forwarded");
        write_bound_sealed_transit_message(&mut peer, &transit)
            .map_err(|error| format!("write bound transit frame to peer failed: {error}"))?;
    }
}

fn pop_bound_transit_lane_next_hop(
    peer_pool: &SharedPeerPool,
    binding: TransitPathBinding,
) -> Result<SecurePeerStream, String> {
    let lane_index = usize::from(binding.lane_id().get().saturating_sub(1));
    peer_pool
        .try_pop_index(lane_index)?
        .ok_or_else(|| "sealed transit lane binding next hop unavailable".to_string())
}

fn pop_bound_transit_dispatch_next_hop(
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    next_hops: Option<SharedPeerPool>,
    binding: TransitPathBinding,
) -> Result<SecurePeerStream, String> {
    let mut dispatcher_binding_error = None;
    if let Some(dispatcher) = dispatcher {
        match dispatcher.pop_for(binding) {
            Ok(peer) => return Ok(peer),
            Err(error) if error.contains("binding unavailable") => {
                dispatcher_binding_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }
    let pool = next_hops.ok_or_else(|| {
        dispatcher_binding_error
            .unwrap_or_else(|| "sealed transit path binding dispatcher unavailable".to_string())
    })?;
    pop_bound_transit_lane_next_hop(&pool, binding)
}

pub fn forward_peer_sealed_transit_to_next_hop(
    source: SecurePeerStream,
    policy: PeerTransitPolicy,
    next_hops: Option<SharedPeerPool>,
    first: TransitRelayFrame,
) -> Result<(), String> {
    if policy != PeerTransitPolicy::AllowPoolNextHop {
        return Err("sealed transit next hop denied by policy".to_string());
    }
    let pool = next_hops.ok_or_else(|| "sealed transit next hop unavailable".to_string())?;
    let next_peer = match pool.try_pop_unique()? {
        UniquePeerPop::Ready(peer) => peer,
        UniquePeerPop::Unavailable => {
            return Err("sealed transit next hop unavailable".to_string());
        }
        UniquePeerPop::Ambiguous => {
            return Err("sealed transit next hop ambiguous without path binding".to_string());
        }
    };
    forward_peer_sealed_transit_pair(source, next_peer, first)
}

pub fn forward_bound_peer_sealed_transit_to_next_hop(
    source: SecurePeerStream,
    policy: PeerTransitPolicy,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
    next_hops: Option<SharedPeerPool>,
    first: BoundTransitRelayFrame,
) -> Result<(), String> {
    if policy != PeerTransitPolicy::AllowPoolNextHop {
        return Err("sealed transit next hop denied by policy".to_string());
    }
    let next_peer = pop_bound_transit_dispatch_next_hop(dispatcher, next_hops, first.binding())?;
    forward_peer_sealed_transit_pair(source, next_peer, first.into_frame())
}

fn forward_peer_sealed_transit_pair(
    source: SecurePeerStream,
    next_peer: SecurePeerStream,
    first: TransitRelayFrame,
) -> Result<(), String> {
    let (source_reader, source_writer) = split_secure_peer_stream(source)?;
    let (next_reader, next_writer) = split_secure_peer_stream(next_peer)?;
    let forward = thread::spawn(move || {
        pipe_sealed_transit_direction(source_reader, next_writer, Some(first), "source_to_next")
    });
    let reverse = thread::spawn(move || {
        pipe_sealed_transit_direction(next_reader, source_writer, None, "next_to_source")
    });
    let forward_result = forward
        .join()
        .map_err(|_| "sealed transit forward worker panicked".to_string())?;
    let reverse_result = reverse
        .join()
        .map_err(|_| "sealed transit reverse worker panicked".to_string())?;
    forward_result?;
    reverse_result?;
    Ok(())
}

fn pipe_sealed_transit_direction(
    mut reader: SecurePeerReader,
    mut writer: SecurePeerWriter,
    first: Option<TransitRelayFrame>,
    direction: &'static str,
) -> Result<(), String> {
    let result = (|| {
        let mut pending = first;
        loop {
            let frame = match pending.take() {
                Some(frame) => frame,
                None => match read_peer_message_from_reader(
                    &mut reader,
                    crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
                )? {
                    PeerMessage::SealedTransit(frame) => frame,
                    PeerMessage::Connect(_) => {
                        return Err("sealed transit stream received connect message".to_string());
                    }
                    PeerMessage::AckOk => {
                        return Err("sealed transit stream received ack".to_string());
                    }
                    PeerMessage::BoundSealedTransit(_) => {
                        return Err(
                            "sealed transit stream received nested bound transit frame".to_string()
                        );
                    }
                },
            };
            eprintln!("event=weave_peer_transit_frame_forwarded direction={direction}");
            let is_fin = frame.kind() == chimera_session::FrameKind::Fin;
            writer
                .write_secure_payload(frame.sealed_bytes())
                .map_err(|error| format!("write peer transit frame to next hop failed: {error}"))?;
            if is_fin {
                let _ = writer.stream.shutdown(Shutdown::Write);
                return Ok(());
            }
        }
    })();
    if result.is_err() {
        reader.shutdown();
        writer.shutdown();
    }
    result
}

fn read_peer_message_from_reader(
    reader: &mut SecurePeerReader,
    max_line_len: usize,
) -> Result<PeerMessage, String> {
    let payload = reader.read_secure_payload()?;
    crate::peer_egress::wire::parse_peer_payload(payload, max_line_len)
}

pub fn read_weave_sealed_transit_frame<R: Read>(
    stream: &mut R,
    first_byte: u8,
) -> Result<TransitRelayFrame, String> {
    if first_byte != chimera_session::FRAME_VERSION {
        return Err("transit frame version invalid".to_string());
    }
    let mut header = [0_u8; TRANSIT_FRAME_HEADER_REST_LEN];
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
    let mut frame = Vec::with_capacity(TRANSIT_FRAME_HEADER_LEN + payload_len);
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

pub fn read_weave_bound_sealed_transit_frame<R: Read>(
    stream: &mut R,
    first_byte: u8,
) -> Result<BoundTransitRelayFrame, String> {
    if first_byte != crate::peer_egress::transit_binding::BOUND_TRANSIT_MAGIC {
        return Err("bound transit frame magic invalid".to_string());
    }
    let mut header = [0_u8; crate::peer_egress::transit_binding::BOUND_TRANSIT_HEADER_REST_LEN];
    stream
        .read_exact(&mut header)
        .map_err(|error| format!("read bound transit frame header failed: {error}"))?;
    let mut nested_first = [0_u8; 1];
    stream
        .read_exact(&mut nested_first)
        .map_err(|error| format!("read nested sealed transit frame version failed: {error}"))?;
    let sealed = read_weave_sealed_transit_frame(stream, nested_first[0])?;
    let mut frame = Vec::with_capacity(
        1 + crate::peer_egress::transit_binding::BOUND_TRANSIT_HEADER_REST_LEN
            + sealed.sealed_bytes().len(),
    );
    frame.push(first_byte);
    frame.extend_from_slice(&header);
    frame.extend_from_slice(sealed.sealed_bytes());
    crate::peer_egress::transit_binding::validate_bound_transit_relay_frame(&frame)
}

#[cfg(test)]
mod tests {
    use super::{
        PeerTransitPolicy, forward_bound_peer_sealed_transit_to_next_hop,
        forward_peer_sealed_transit_to_next_hop, forward_transit_relay_frame,
        read_weave_bound_sealed_transit_frame, read_weave_sealed_transit_frame,
        relay_local_bound_sealed_transit, validate_transit_relay_frame,
    };
    use crate::peer_egress::wire::{PeerMessage, read_peer_message, write_connect_message};
    use chimera_session::{Frame, FrameKind};
    use std::io;
    use std::io::{Cursor, Read, Write};

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

    fn binding(route: u64, lane: u16) -> crate::peer_egress::transit_binding::TransitPathBinding {
        crate::peer_egress::transit_binding::TransitPathBinding::new(
            crate::peer_egress::transit_binding::TransitRouteId::new(route)
                .unwrap_or_else(|error| unreachable!("route id must be valid: {error}")),
            crate::peer_egress::transit_binding::TransitLaneId::new(lane)
                .unwrap_or_else(|error| unreachable!("lane id must be valid: {error}")),
        )
    }

    fn tcp_pair() -> Result<(std::net::TcpStream, std::net::TcpStream), String> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("bind test listener failed: {error}"))?;
        let addr = listener
            .local_addr()
            .map_err(|error| format!("read test listener addr failed: {error}"))?;
        let client = std::net::TcpStream::connect(addr)
            .map_err(|error| format!("connect test client failed: {error}"))?;
        let (server, _) = listener
            .accept()
            .map_err(|error| format!("accept test server failed: {error}"))?;
        Ok((client, server))
    }

    fn test_peer_pair() -> Result<
        (
            crate::peer_egress::protocol::SecurePeerStream,
            crate::peer_egress::protocol::SecurePeerStream,
        ),
        String,
    > {
        let transcript = chimera_crypto::TranscriptHash::from_messages(&[b"peer-transit-test"]);
        let secrets = chimera_crypto::derive_traffic_secrets(
            chimera_crypto::SuiteId(
                crate::peer_egress::options::AeadSuite::Chacha20Poly1305.suite_id(),
            ),
            &transcript,
            &[11_u8; 32],
        )
        .map_err(|error| format!("derive test secrets failed: {error}"))?;
        let (left, right) = tcp_pair()?;
        Ok((
            crate::peer_egress::protocol::SecurePeerStream {
                stream: left,
                send_secret: secrets.initiator_to_responder().clone(),
                recv_secret: secrets.responder_to_initiator().clone(),
                send_packet: 0,
                recv_packet: 0,
                aead: crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
            },
            crate::peer_egress::protocol::SecurePeerStream {
                stream: right,
                send_secret: secrets.responder_to_initiator().clone(),
                recv_secret: secrets.initiator_to_responder().clone(),
                send_packet: 0,
                recv_packet: 0,
                aead: crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
            },
        ))
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

    #[test]
    fn peer_sealed_transit_pumps_both_directions_without_connect_parsing() -> Result<(), String> {
        let (mut source_writer, source_reader) = test_peer_pair()?;
        let (next_writer, mut next_reader) = test_peer_pair()?;
        let first_encoded = encoded_frame(FrameKind::Data, 201, b"closed transit payload");
        let fin_encoded = encoded_frame(FrameKind::Fin, 202, b"");
        let reverse_encoded = encoded_frame(FrameKind::Data, 301, b"closed reverse payload");
        let reverse_fin_encoded = encoded_frame(FrameKind::Fin, 302, b"");
        source_writer.write_secure_payload(&first_encoded)?;
        let mut source_reader = source_reader;
        let first_frame = match read_peer_message(
            &mut source_reader,
            crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
        )? {
            PeerMessage::SealedTransit(frame) => frame,
            other => return Err(format!("unexpected first message: {other:?}")),
        };
        let pool = crate::peer_egress::pool::new_shared_pool();
        pool.push(next_writer)?;

        source_writer.write_secure_payload(&fin_encoded)?;
        next_reader.write_secure_payload(&reverse_encoded)?;
        next_reader.write_secure_payload(&reverse_fin_encoded)?;
        forward_peer_sealed_transit_to_next_hop(
            source_reader,
            PeerTransitPolicy::AllowPoolNextHop,
            Some(pool),
            first_frame,
        )?;

        let forwarded_first = next_reader.read_secure_payload()?;
        let forwarded_fin = next_reader.read_secure_payload()?;
        let reverse_first = source_writer.read_secure_payload()?;
        let reverse_fin = source_writer.read_secure_payload()?;
        assert_eq!(forwarded_first, first_encoded);
        assert_eq!(forwarded_fin, fin_encoded);
        assert_eq!(reverse_first, reverse_encoded);
        assert_eq!(reverse_fin, reverse_fin_encoded);
        assert!(
            !String::from_utf8_lossy(&forwarded_first)
                .to_ascii_uppercase()
                .contains("CONNECT")
        );
        assert!(
            !String::from_utf8_lossy(&reverse_first)
                .to_ascii_uppercase()
                .contains("CONNECT")
        );
        Ok(())
    }

    #[test]
    fn bound_peer_sealed_transit_dispatches_matching_next_hop() -> Result<(), String> {
        let (mut source_writer, source_reader) = test_peer_pair()?;
        let (next_writer, mut next_reader) = test_peer_pair()?;
        let first_encoded = encoded_frame(FrameKind::Data, 601, b"bound closed transit payload");
        let fin_encoded = encoded_frame(FrameKind::Fin, 602, b"");
        let reverse_encoded = encoded_frame(FrameKind::Data, 701, b"bound closed reverse payload");
        let reverse_fin_encoded = encoded_frame(FrameKind::Fin, 702, b"");
        let binding = binding(77, 3);
        let first_frame = validate_transit_relay_frame(&first_encoded)?;
        let bound_first =
            crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(binding, first_frame);
        let bound_payload =
            crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&bound_first);

        source_writer.write_secure_payload(&bound_payload)?;
        let mut source_reader = source_reader;
        let first_bound = match read_peer_message(
            &mut source_reader,
            crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
        )? {
            PeerMessage::BoundSealedTransit(frame) => frame,
            other => return Err(format!("unexpected first bound message: {other:?}")),
        };
        let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
        dispatcher.register(binding, next_writer)?;

        source_writer.write_secure_payload(&fin_encoded)?;
        next_reader.write_secure_payload(&reverse_encoded)?;
        next_reader.write_secure_payload(&reverse_fin_encoded)?;
        forward_bound_peer_sealed_transit_to_next_hop(
            source_reader,
            PeerTransitPolicy::AllowPoolNextHop,
            Some(dispatcher),
            None,
            first_bound,
        )?;

        let forwarded_first = next_reader.read_secure_payload()?;
        let forwarded_fin = next_reader.read_secure_payload()?;
        let reverse_first = source_writer.read_secure_payload()?;
        let reverse_fin = source_writer.read_secure_payload()?;
        assert_eq!(forwarded_first, first_encoded);
        assert_eq!(forwarded_fin, fin_encoded);
        assert_eq!(reverse_first, reverse_encoded);
        assert_eq!(reverse_fin, reverse_fin_encoded);
        assert!(
            !forwarded_first
                .starts_with(&[crate::peer_egress::transit_binding::BOUND_TRANSIT_MAGIC,])
        );
        Ok(())
    }

    #[test]
    fn bound_peer_sealed_transit_uses_lane_pool_when_dispatcher_has_no_binding()
    -> Result<(), String> {
        let (mut source_writer, source_reader) = test_peer_pair()?;
        let (first_next_writer, mut first_next_reader) = test_peer_pair()?;
        let (second_next_writer, mut second_next_reader) = test_peer_pair()?;
        let first_encoded = encoded_frame(FrameKind::Data, 621, b"bound lane payload");
        let fin_encoded = encoded_frame(FrameKind::Fin, 622, b"");
        let reverse_fin_encoded = encoded_frame(FrameKind::Fin, 623, b"");
        let binding = binding(79, 2);
        let first_frame = validate_transit_relay_frame(&first_encoded)?;
        let bound_first =
            crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(binding, first_frame);
        let bound_payload =
            crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&bound_first);

        source_writer.write_secure_payload(&bound_payload)?;
        let mut source_reader = source_reader;
        let first_bound = match read_peer_message(
            &mut source_reader,
            crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
        )? {
            PeerMessage::BoundSealedTransit(frame) => frame,
            other => return Err(format!("unexpected first bound message: {other:?}")),
        };
        let pool = crate::peer_egress::pool::new_shared_pool();
        pool.push(first_next_writer)?;
        pool.push(second_next_writer)?;

        source_writer.write_secure_payload(&fin_encoded)?;
        second_next_reader.write_secure_payload(&reverse_fin_encoded)?;
        first_next_reader
            .stream
            .set_read_timeout(Some(std::time::Duration::from_millis(50)))
            .map_err(|error| format!("set wrong lane read timeout failed: {error}"))?;
        forward_bound_peer_sealed_transit_to_next_hop(
            source_reader,
            PeerTransitPolicy::AllowPoolNextHop,
            Some(crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher()),
            Some(pool),
            first_bound,
        )?;

        assert!(first_next_reader.read_secure_payload().is_err());
        assert_eq!(second_next_reader.read_secure_payload()?, first_encoded);
        assert_eq!(second_next_reader.read_secure_payload()?, fin_encoded);
        assert_eq!(source_writer.read_secure_payload()?, reverse_fin_encoded);
        Ok(())
    }

    #[test]
    fn bound_peer_sealed_transit_requires_policy_dispatcher_and_known_binding() -> Result<(), String>
    {
        let (_source_writer, source_reader) = test_peer_pair()?;
        let first_encoded = encoded_frame(FrameKind::Data, 801, b"bound closed transit payload");
        let binding = binding(88, 4);
        let first = crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(
            binding,
            validate_transit_relay_frame(&first_encoded)?,
        );

        let error = match forward_bound_peer_sealed_transit_to_next_hop(
            source_reader,
            PeerTransitPolicy::DenyPoolNextHop,
            None,
            None,
            first.clone(),
        ) {
            Ok(()) => return Err("bound transit denied by policy must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("denied by policy"));

        let (_source_writer, source_reader) = test_peer_pair()?;
        let error = match forward_bound_peer_sealed_transit_to_next_hop(
            source_reader,
            PeerTransitPolicy::AllowPoolNextHop,
            None,
            None,
            first.clone(),
        ) {
            Ok(()) => return Err("bound transit without dispatcher must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("dispatcher unavailable"));

        let (_source_writer, source_reader) = test_peer_pair()?;
        let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
        let error = match forward_bound_peer_sealed_transit_to_next_hop(
            source_reader,
            PeerTransitPolicy::AllowPoolNextHop,
            Some(dispatcher),
            None,
            first,
        ) {
            Ok(()) => return Err("bound transit with unknown binding must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("binding unavailable"));
        Ok(())
    }

    #[test]
    fn local_bound_sealed_transit_rejects_midstream_binding_change() -> Result<(), String> {
        let (mut local_writer, local_reader) = tcp_pair()?;
        let (mut peer_writer, peer_reader) = test_peer_pair()?;
        let first_encoded = encoded_frame(FrameKind::Data, 831, b"first bound lane payload");
        let changed_encoded = encoded_frame(FrameKind::Data, 832, b"changed bound lane payload");
        let first_bound = crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(
            binding(91, 1),
            validate_transit_relay_frame(&first_encoded)?,
        );
        let changed_bound = crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(
            binding(91, 2),
            validate_transit_relay_frame(&changed_encoded)?,
        );
        let first_payload =
            crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&first_bound);
        let changed_payload =
            crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&changed_bound);
        let first_byte = first_payload[0];
        local_writer
            .write_all(&first_payload[1..])
            .map_err(|error| format!("write first local bound payload failed: {error}"))?;
        local_writer
            .write_all(&changed_payload)
            .map_err(|error| format!("write changed local bound payload failed: {error}"))?;
        drop(local_writer);

        let error = match relay_local_bound_sealed_transit(local_reader, peer_reader, first_byte) {
            Ok(()) => return Err("local bound transit binding change must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("binding changed"));
        let forwarded = peer_writer.read_secure_payload()?;
        assert_eq!(forwarded, first_payload);
        assert!(peer_writer.read_secure_payload().is_err());
        Ok(())
    }

    #[test]
    fn peer_sealed_transit_denies_pool_next_hop_without_policy() -> Result<(), String> {
        let (mut source_writer, source_reader) = test_peer_pair()?;
        let (next_writer, _next_reader) = test_peer_pair()?;
        let first_encoded = encoded_frame(FrameKind::Data, 401, b"closed transit payload");
        source_writer.write_secure_payload(&first_encoded)?;
        let mut source_reader = source_reader;
        let first_frame = match read_peer_message(
            &mut source_reader,
            crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
        )? {
            PeerMessage::SealedTransit(frame) => frame,
            other => return Err(format!("unexpected first message: {other:?}")),
        };
        let pool = crate::peer_egress::pool::new_shared_pool();
        pool.push(next_writer)?;

        let error = match forward_peer_sealed_transit_to_next_hop(
            source_reader,
            PeerTransitPolicy::DenyPoolNextHop,
            Some(pool),
            first_frame,
        ) {
            Ok(()) => return Err("pool transit without policy must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("denied by policy"));
        Ok(())
    }

    #[test]
    fn peer_sealed_transit_rejects_ambiguous_pool_next_hop() -> Result<(), String> {
        let (mut source_writer, source_reader) = test_peer_pair()?;
        let (first_next_writer, _first_next_reader) = test_peer_pair()?;
        let (second_next_writer, _second_next_reader) = test_peer_pair()?;
        let first_encoded = encoded_frame(FrameKind::Data, 451, b"closed transit payload");
        source_writer.write_secure_payload(&first_encoded)?;
        let mut source_reader = source_reader;
        let first_frame = match read_peer_message(
            &mut source_reader,
            crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
        )? {
            PeerMessage::SealedTransit(frame) => frame,
            other => return Err(format!("unexpected first message: {other:?}")),
        };
        let pool = crate::peer_egress::pool::new_shared_pool();
        pool.push(first_next_writer)?;
        pool.push(second_next_writer)?;

        let error = match forward_peer_sealed_transit_to_next_hop(
            source_reader,
            PeerTransitPolicy::AllowPoolNextHop,
            Some(pool),
            first_frame,
        ) {
            Ok(()) => return Err("ambiguous pool transit next hop must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("ambiguous"));
        Ok(())
    }

    #[test]
    fn peer_sealed_transit_rejects_midstream_connect_and_unblocks_reverse() -> Result<(), String> {
        let (mut source_writer, source_reader) = test_peer_pair()?;
        let (next_writer, mut next_reader) = test_peer_pair()?;
        let first_encoded = encoded_frame(FrameKind::Data, 501, b"closed transit payload");
        source_writer.write_secure_payload(&first_encoded)?;
        let mut source_reader = source_reader;
        let first_frame = match read_peer_message(
            &mut source_reader,
            crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
        )? {
            PeerMessage::SealedTransit(frame) => frame,
            other => return Err(format!("unexpected first message: {other:?}")),
        };
        let pool = crate::peer_egress::pool::new_shared_pool();
        pool.push(next_writer)?;
        let destination = crate::peer_egress::protocol::Destination {
            host: "example.org".to_string(),
            port: 443,
        };
        write_connect_message(&mut source_writer, &destination)?;

        let error = match forward_peer_sealed_transit_to_next_hop(
            source_reader,
            PeerTransitPolicy::AllowPoolNextHop,
            Some(pool),
            first_frame,
        ) {
            Ok(()) => return Err("midstream CONNECT in sealed transit must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("connect message"));
        assert_eq!(next_reader.read_secure_payload()?, first_encoded);
        assert!(source_writer.read_secure_payload().is_err());
        Ok(())
    }

    #[test]
    fn peer_sealed_transit_rejects_midstream_bound_frame() -> Result<(), String> {
        let (mut source_writer, source_reader) = test_peer_pair()?;
        let (next_writer, mut next_reader) = test_peer_pair()?;
        let first_encoded = encoded_frame(FrameKind::Data, 901, b"closed transit payload");
        let nested_encoded = encoded_frame(FrameKind::Data, 902, b"nested bound payload");
        source_writer.write_secure_payload(&first_encoded)?;
        let mut source_reader = source_reader;
        let first_frame = match read_peer_message(
            &mut source_reader,
            crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
        )? {
            PeerMessage::SealedTransit(frame) => frame,
            other => return Err(format!("unexpected first message: {other:?}")),
        };
        let nested_bound = crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(
            binding(99, 5),
            validate_transit_relay_frame(&nested_encoded)?,
        );
        let nested_payload =
            crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&nested_bound);
        source_writer.write_secure_payload(&nested_payload)?;
        let pool = crate::peer_egress::pool::new_shared_pool();
        pool.push(next_writer)?;

        let error = match forward_peer_sealed_transit_to_next_hop(
            source_reader,
            PeerTransitPolicy::AllowPoolNextHop,
            Some(pool),
            first_frame,
        ) {
            Ok(()) => return Err("midstream bound transit frame must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("nested bound transit frame"));
        assert_eq!(next_reader.read_secure_payload()?, first_encoded);
        assert!(source_writer.read_secure_payload().is_err());
        Ok(())
    }
}
