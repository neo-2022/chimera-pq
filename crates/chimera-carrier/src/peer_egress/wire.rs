use core::fmt;

use crate::peer_egress::protocol::{Destination, SecurePeerStream, parse_peer_connect_destination};
use crate::peer_egress::transit::{TransitRelayFrame, validate_transit_relay_frame};
#[cfg(test)]
use crate::peer_egress::transit_binding::encode_bound_transit_relay_frame;
use crate::peer_egress::transit_binding::{
    BOUND_TRANSIT_MAGIC, BoundTransitRelayFrame, validate_bound_transit_relay_frame,
};

pub(crate) enum PeerMessage {
    Connect(Destination),
    AckOk,
    SealedTransit(TransitRelayFrame),
    BoundSealedTransit(BoundTransitRelayFrame),
}

impl fmt::Debug for PeerMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect(destination) => f
                .debug_struct("PeerMessage::Connect")
                .field("destination_id", &destination.redacted_label())
                .finish(),
            Self::AckOk => f.write_str("PeerMessage::AckOk"),
            Self::SealedTransit(frame) => f
                .debug_struct("PeerMessage::SealedTransit")
                .field("frame", frame)
                .finish(),
            Self::BoundSealedTransit(frame) => f
                .debug_struct("PeerMessage::BoundSealedTransit")
                .field("frame", frame)
                .finish(),
        }
    }
}

pub(crate) fn read_peer_message(
    peer: &mut SecurePeerStream,
    max_line_len: usize,
) -> Result<PeerMessage, String> {
    let payload = peer.read_secure_payload()?;
    parse_peer_payload(payload, max_line_len)
}

pub(crate) fn write_connect_message(
    peer: &mut SecurePeerStream,
    destination: &Destination,
) -> Result<(), String> {
    peer.write_line(&format!(
        "CONNECT {} {}",
        destination.host, destination.port
    ))
}

pub(crate) fn write_ack_ok(peer: &mut SecurePeerStream) -> Result<(), String> {
    peer.write_line("OK")
}

pub(crate) fn write_sealed_transit_message(
    peer: &mut SecurePeerStream,
    frame: &TransitRelayFrame,
) -> Result<(), String> {
    peer.write_secure_payload(frame.sealed_bytes())
}

#[cfg(test)]
pub(crate) fn write_bound_sealed_transit_message(
    peer: &mut SecurePeerStream,
    frame: &BoundTransitRelayFrame,
) -> Result<(), String> {
    peer.write_secure_payload(&encode_bound_transit_relay_frame(frame))
}

pub(crate) fn parse_peer_payload(
    payload: Vec<u8>,
    max_line_len: usize,
) -> Result<PeerMessage, String> {
    if payload.first() == Some(&BOUND_TRANSIT_MAGIC) {
        return validate_bound_transit_relay_frame(&payload).map(PeerMessage::BoundSealedTransit);
    }
    if payload.first() == Some(&chimera_session::FRAME_VERSION) {
        return validate_transit_relay_frame(&payload).map(PeerMessage::SealedTransit);
    }
    if payload.len() > max_line_len {
        return Err("peer message line too long".to_string());
    }
    if payload.last() != Some(&b'\n') {
        return Err("peer message missing newline".to_string());
    }
    let line = String::from_utf8(payload[..payload.len() - 1].to_vec())
        .map_err(|_| "peer message is not utf-8".to_string())?;
    if line == "OK" {
        return Ok(PeerMessage::AckOk);
    }
    parse_peer_connect_destination(&line).map(PeerMessage::Connect)
}

#[cfg(test)]
mod tests {
    use super::{
        PeerMessage, parse_peer_payload, read_peer_message, write_ack_ok,
        write_bound_sealed_transit_message, write_connect_message, write_sealed_transit_message,
    };
    use chimera_session::{Frame, FrameKind};

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
        let transcript = chimera_crypto::TranscriptHash::from_messages(&[b"peer-wire-test"]);
        let secrets = chimera_crypto::derive_traffic_secrets(
            chimera_crypto::SuiteId(
                crate::peer_egress::options::AeadSuite::Chacha20Poly1305.suite_id(),
            ),
            &transcript,
            &[17_u8; 32],
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
    fn peer_payload_classifies_legacy_connect_without_logging_destination() -> Result<(), String> {
        let message = parse_peer_payload(b"CONNECT example.org 443\n".to_vec(), 512)?;
        let debug = format!("{message:?}");

        match message {
            PeerMessage::Connect(destination) => {
                assert_eq!(destination.host, "example.org");
                assert_eq!(destination.port, 443);
            }
            other => return Err(format!("unexpected message: {other:?}")),
        }
        assert!(!debug.contains("example.org"));
        assert!(!debug.contains("443"));
        Ok(())
    }

    #[test]
    fn peer_payload_classifies_ack() -> Result<(), String> {
        let message = parse_peer_payload(b"OK\n".to_vec(), 16)?;
        assert!(matches!(message, PeerMessage::AckOk));
        Ok(())
    }

    #[test]
    fn peer_payload_classifies_sealed_transit_without_payload_debug_leak() -> Result<(), String> {
        let encoded = encoded_frame(FrameKind::Data, 17, b"third-party closed payload");
        let message = parse_peer_payload(encoded.clone(), 512)?;
        let debug = format!("{message:?}");

        match message {
            PeerMessage::SealedTransit(frame) => {
                assert_eq!(frame.kind(), FrameKind::Data);
                assert_eq!(frame.packet_number(), 17);
                assert_eq!(frame.sealed_bytes(), encoded.as_slice());
            }
            other => return Err(format!("unexpected message: {other:?}")),
        }
        assert!(debug.contains("<sealed>"));
        assert!(!debug.contains("third-party closed payload"));
        Ok(())
    }

    #[test]
    fn peer_payload_classifies_bound_sealed_transit_without_payload_debug_leak()
    -> Result<(), String> {
        let encoded = encoded_frame(FrameKind::Data, 27, b"bound third-party payload");
        let frame = crate::peer_egress::transit::validate_transit_relay_frame(&encoded)?;
        let binding = crate::peer_egress::transit_binding::TransitPathBinding::new(
            crate::peer_egress::transit_binding::TransitRouteId::new(7)?,
            crate::peer_egress::transit_binding::TransitLaneId::new(2)?,
        );
        let bound =
            crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(binding, frame);
        let payload = crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&bound);
        let message = parse_peer_payload(payload, 512)?;
        let debug = format!("{message:?}");

        match message {
            PeerMessage::BoundSealedTransit(frame) => {
                assert_eq!(frame.binding(), binding);
                assert_eq!(frame.frame().sealed_bytes(), encoded.as_slice());
            }
            other => return Err(format!("unexpected message: {other:?}")),
        }
        assert!(debug.contains("<opaque>"));
        assert!(debug.contains("<sealed>"));
        assert!(!debug.contains("bound third-party payload"));
        Ok(())
    }

    #[test]
    fn peer_payload_rejects_malformed_sealed_transit_without_text_fallback() {
        let mut encoded = encoded_frame(FrameKind::Data, 18, b"opaque");
        encoded.truncate(encoded.len() - 1);

        let error = match parse_peer_payload(encoded, 512) {
            Ok(message) => unreachable!("malformed sealed transit frame must fail: {message:?}"),
            Err(error) => error,
        };
        assert!(error.contains("transit frame"));
    }

    #[test]
    fn peer_payload_rejects_malformed_bound_transit_without_text_fallback() {
        let payload = vec![
            crate::peer_egress::transit_binding::BOUND_TRANSIT_MAGIC,
            b'O',
            b'K',
            b'\n',
        ];

        let error = match parse_peer_payload(payload, 512) {
            Ok(message) => unreachable!("malformed bound transit frame must fail: {message:?}"),
            Err(error) => error,
        };
        assert!(error.contains("bound sealed transit"));
        assert!(!error.contains("OK"));
    }

    #[test]
    fn peer_wire_messages_round_trip_connect_ack_and_sealed_transit() -> Result<(), String> {
        let (mut left, mut right) = test_peer_pair()?;
        let destination = crate::peer_egress::protocol::Destination {
            host: "example.org".to_string(),
            port: 443,
        };
        write_connect_message(&mut left, &destination)?;
        match read_peer_message(&mut right, 512)? {
            PeerMessage::Connect(destination) => {
                assert_eq!(destination.host, "example.org");
                assert_eq!(destination.port, 443);
            }
            other => return Err(format!("unexpected connect message: {other:?}")),
        }

        write_ack_ok(&mut right)?;
        assert!(matches!(
            read_peer_message(&mut left, 16)?,
            PeerMessage::AckOk
        ));

        let encoded = encoded_frame(FrameKind::Data, 33, b"sealed round trip");
        let frame = crate::peer_egress::transit::validate_transit_relay_frame(&encoded)?;
        write_sealed_transit_message(&mut left, &frame)?;
        match read_peer_message(&mut right, 512)? {
            PeerMessage::SealedTransit(frame) => {
                assert_eq!(frame.sealed_bytes(), encoded.as_slice());
            }
            other => return Err(format!("unexpected transit message: {other:?}")),
        }
        Ok(())
    }

    #[test]
    fn peer_wire_messages_round_trip_bound_sealed_transit() -> Result<(), String> {
        let (mut left, mut right) = test_peer_pair()?;
        let encoded = encoded_frame(FrameKind::Data, 44, b"bound sealed round trip");
        let frame = crate::peer_egress::transit::validate_transit_relay_frame(&encoded)?;
        let binding = crate::peer_egress::transit_binding::TransitPathBinding::new(
            crate::peer_egress::transit_binding::TransitRouteId::new(9)?,
            crate::peer_egress::transit_binding::TransitLaneId::new(3)?,
        );
        let bound =
            crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(binding, frame);

        write_bound_sealed_transit_message(&mut left, &bound)?;
        let message = read_peer_message(
            &mut right,
            crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
        )?;
        let debug = format!("{message:?}");
        match message {
            PeerMessage::BoundSealedTransit(frame) => {
                assert_eq!(frame.binding(), binding);
                assert_eq!(frame.frame().sealed_bytes(), encoded.as_slice());
            }
            other => return Err(format!("unexpected bound transit message: {other:?}")),
        }
        assert!(!debug.contains("bound sealed round trip"));
        assert!(!debug.contains("route_id: 9"));
        assert!(!debug.contains("lane_id: 3"));
        Ok(())
    }
}
