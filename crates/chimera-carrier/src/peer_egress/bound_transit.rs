use std::net::Shutdown;
use std::thread;

use crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN;
use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::secure_halves::{
    SecurePeerReader, SecurePeerWriter, split_secure_peer_stream,
};
use crate::peer_egress::transit_binding::{
    BoundTransitRelayFrame, TransitPathBinding, encode_bound_transit_relay_frame,
};
use crate::peer_egress::wire::PeerMessage;

pub(crate) fn forward_bound_peer_transit_pair(
    source: SecurePeerStream,
    next_peer: SecurePeerStream,
    first: BoundTransitRelayFrame,
) -> Result<(), String> {
    let binding = first.binding();
    let (source_reader, source_writer) = split_secure_peer_stream(source)?;
    let (next_reader, next_writer) = split_secure_peer_stream(next_peer)?;
    let forward = thread::spawn(move || {
        pipe_bound_transit_direction(
            source_reader,
            next_writer,
            Some(first),
            binding,
            "source_to_next",
        )
    });
    let reverse = thread::spawn(move || {
        pipe_bound_transit_direction(next_reader, source_writer, None, binding, "next_to_source")
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
) -> Result<(), String> {
    let result = (|| {
        let mut pending = first;
        loop {
            let frame = match pending.take() {
                Some(frame) => frame,
                None => match read_bound_peer_message(&mut reader)? {
                    PeerMessage::BoundSealedTransit(frame) => frame,
                    PeerMessage::AggregateSealedTransit(_) => {
                        return Err(
                            "bound transit stream received aggregate transit frame".to_string()
                        );
                    }
                    PeerMessage::SealedTransit(_) => {
                        return Err(
                            "bound transit stream received unbound transit frame".to_string()
                        );
                    }
                    PeerMessage::Connect(_) => {
                        return Err("bound transit stream received connect message".to_string());
                    }
                    PeerMessage::AckOk => {
                        return Err("bound transit stream received ack".to_string());
                    }
                },
            };
            if frame.binding() != binding {
                return Err("bound transit stream binding changed midstream".to_string());
            }
            let is_fin = frame.frame().kind() == chimera_session::FrameKind::Fin;
            eprintln!("event=weave_bound_peer_transit_frame_forwarded direction={direction}");
            writer
                .write_secure_payload(&encode_bound_transit_relay_frame(&frame))
                .map_err(|error| {
                    format!("write bound peer transit frame to next hop failed: {error}")
                })?;
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

fn read_bound_peer_message(reader: &mut SecurePeerReader) -> Result<PeerMessage, String> {
    let payload = reader.read_secure_payload()?;
    crate::peer_egress::wire::parse_peer_payload(payload, SECURE_PLAINTEXT_CHUNK_LEN)
}
