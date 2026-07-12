use std::net::Shutdown;
use std::thread;

use crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN;
use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::relay_activity::RelayActivity;
use crate::peer_egress::secure_halves::{
    SecurePayloadRead, SecurePeerReader, SecurePeerWriter, split_secure_peer_stream,
};
use crate::peer_egress::transit_binding::{
    BOUND_TRANSIT_HEADER_LEN, BoundTransitRelayFrame, TransitPathBinding,
    encode_bound_transit_relay_frame,
};
use crate::peer_egress::transit_guard::{
    TransitRelayGuard, TransitRelayLimits, apply_transit_stream_limits,
};
use crate::peer_egress::wire::PeerMessage;

pub(crate) fn forward_bound_peer_transit_pair(
    source: SecurePeerStream,
    next_peer: SecurePeerStream,
    first: BoundTransitRelayFrame,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    limits.validate()?;
    apply_transit_stream_limits(&source.stream, limits)?;
    apply_transit_stream_limits(&next_peer.stream, limits)?;
    let binding = first.binding();
    let (source_reader, source_writer) = split_secure_peer_stream(source)?;
    let (next_reader, next_writer) = split_secure_peer_stream(next_peer)?;
    let relay_activity = RelayActivity::new();
    let forward_activity = relay_activity.clone();
    let forward = thread::spawn(move || {
        pipe_bound_transit_direction(
            source_reader,
            next_writer,
            Some(first),
            binding,
            "source_to_next",
            limits,
            forward_activity,
        )
    });
    let reverse = thread::spawn(move || {
        pipe_bound_transit_direction(
            next_reader,
            source_writer,
            None,
            binding,
            "next_to_source",
            limits,
            relay_activity,
        )
    });
    let forward_result = forward
        .join()
        .map_err(|_| "bound transit forward worker panicked".to_string())?;
    let reverse_result = reverse
        .join()
        .map_err(|_| "bound transit reverse worker panicked".to_string())?;
    if let Err(error) = &forward_result
        && error.contains("binding changed")
    {
        return Err(error.clone());
    }
    if let Err(error) = &reverse_result
        && error.contains("binding changed")
    {
        return Err(error.clone());
    }
    if let Err(error) = &forward_result
        && error.contains("session idle timeout")
    {
        return Err(error.clone());
    }
    if let Err(error) = &reverse_result
        && error.contains("session idle timeout")
    {
        return Err(error.clone());
    }
    forward_result?;
    reverse_result?;
    Ok(())
}

fn pipe_bound_transit_direction(
    mut reader: SecurePeerReader,
    mut writer: SecurePeerWriter,
    first: Option<BoundTransitRelayFrame>,
    binding: TransitPathBinding,
    direction: &'static str,
    limits: TransitRelayLimits,
    relay_activity: RelayActivity,
) -> Result<(), String> {
    limits.validate()?;
    let result = (|| {
        let mut guard = TransitRelayGuard::new(limits);
        let mut pending = first;
        loop {
            let frame = match pending.take() {
                Some(frame) => frame,
                None => loop {
                    let observed_activity = relay_activity.snapshot();
                    match read_bound_peer_message(&mut reader)? {
                        PeerReadOutcome::Message(PeerMessage::BoundSealedTransit(frame)) => {
                            break frame;
                        }
                        PeerReadOutcome::Message(PeerMessage::AggregateSealedTransit(_)) => {
                            return Err(
                                "bound transit stream received aggregate transit frame".to_string()
                            );
                        }
                        PeerReadOutcome::Message(PeerMessage::SealedTransit(_)) => {
                            return Err(
                                "bound transit stream received unbound transit frame".to_string()
                            );
                        }
                        PeerReadOutcome::Message(PeerMessage::Connect(_)) => {
                            return Err("bound transit stream received connect message".to_string());
                        }
                        PeerReadOutcome::Message(PeerMessage::AckOk) => {
                            return Err("bound transit stream received ack".to_string());
                        }
                        PeerReadOutcome::Message(PeerMessage::Announce(_)) => {
                            return Err("bound transit stream received announce message".to_string());
                        }
                        PeerReadOutcome::Idle => {
                            if relay_activity.has_finished_direction() {
                                return Ok(());
                            }
                            if relay_activity.unchanged_since(observed_activity) {
                                return Err("bound transit session idle timeout".to_string());
                            }
                        }
                    }
                },
            };
            if frame.binding() != binding {
                return Err("bound transit stream binding changed midstream".to_string());
            }
            guard.record_frame(
                frame
                    .frame()
                    .sealed_bytes()
                    .len()
                    .saturating_add(BOUND_TRANSIT_HEADER_LEN),
            )?;
            relay_activity.record();
            let is_fin = frame.frame().kind() == chimera_session::FrameKind::Fin;
            eprintln!("event=weave_bound_peer_transit_frame_forwarded direction={direction}");
            writer
                .write_secure_payload(&encode_bound_transit_relay_frame(&frame))
                .map_err(|error| {
                    format!("write bound peer transit frame to next hop failed: {error}")
                })?;
            if is_fin {
                relay_activity.record_finished_direction();
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

fn read_bound_peer_message(reader: &mut SecurePeerReader) -> Result<PeerReadOutcome, String> {
    match reader.read_secure_payload_or_idle()? {
        SecurePayloadRead::Payload(payload) => {
            crate::peer_egress::wire::parse_peer_payload(payload, SECURE_PLAINTEXT_CHUNK_LEN)
                .map(PeerReadOutcome::Message)
        }
        SecurePayloadRead::Idle => Ok(PeerReadOutcome::Idle),
    }
}

enum PeerReadOutcome {
    Message(PeerMessage),
    Idle,
}
